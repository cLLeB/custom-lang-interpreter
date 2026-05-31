//! A stack-based virtual machine that executes [`crate::bytecode`].
//!
//! Runtime semantics (arithmetic, string concatenation, equality, comparison,
//! truthiness, division-by-zero) mirror the tree-walking interpreter so the two
//! engines agree on the core sub-language. `print` output is captured into a
//! buffer so programs are observable and testable.

use crate::bytecode::{Constant, Function, Op};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// A VM runtime value. A deliberately small set matching the compiled core.
#[derive(Clone)]
pub enum Val {
    Number(f64),
    Str(Rc<str>),
    Bool(bool),
    Null,
    Fn(Rc<Function>),
}

impl Val {
    fn truthy(&self) -> bool {
        match self {
            Val::Bool(b) => *b,
            Val::Null => false,
            Val::Number(n) => *n != 0.0,
            Val::Str(s) => !s.is_empty(),
            Val::Fn(_) => true,
        }
    }

    fn equals(&self, other: &Val) -> bool {
        match (self, other) {
            (Val::Number(a), Val::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Val::Str(a), Val::Str(b)) => a == b,
            (Val::Bool(a), Val::Bool(b)) => a == b,
            (Val::Null, Val::Null) => true,
            (Val::Fn(a), Val::Fn(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Val::Number(_) => "number",
            Val::Str(_) => "string",
            Val::Bool(_) => "boolean",
            Val::Null => "null",
            Val::Fn(_) => "function",
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Mirror ast::Value: whole numbers print without a decimal point.
            Val::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            Val::Str(s) => write!(f, "{s}"),
            Val::Bool(b) => write!(f, "{b}"),
            Val::Null => write!(f, "null"),
            Val::Fn(func) => write!(f, "<function {}>", func.name),
        }
    }
}

#[derive(Debug)]
pub struct VmError {
    pub line: u32,
    pub msg: String,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "runtime error (line {}): {}", self.line, self.msg)
    }
}

struct Frame {
    func: Rc<Function>,
    ip: usize,
    slot_base: usize,
}

pub struct Vm {
    stack: Vec<Val>,
    frames: Vec<Frame>,
    globals: HashMap<String, Val>,
    output: String,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Vm {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            globals: HashMap::new(),
            output: String::new(),
        }
    }

    /// Run a top-level `<main>` function to completion, returning captured
    /// `print` output.
    pub fn run(&mut self, main: Rc<Function>) -> Result<String, VmError> {
        self.stack.push(Val::Fn(Rc::clone(&main)));
        self.frames.push(Frame {
            func: main,
            ip: 0,
            slot_base: 0,
        });
        self.execute()?;
        Ok(std::mem::take(&mut self.output))
    }

    fn execute(&mut self) -> Result<(), VmError> {
        let mut fi = self.frames.len() - 1;
        let mut func = Rc::clone(&self.frames[fi].func);

        loop {
            let ip = self.frames[fi].ip;
            let op_byte = func.chunk.code[ip];
            let line = func.chunk.line_at(ip);
            self.frames[fi].ip += 1;

            let op = Op::from_u8(op_byte).ok_or_else(|| VmError {
                line,
                msg: format!("invalid opcode {op_byte}"),
            })?;

            match op {
                Op::Constant => {
                    let idx = self.read_u16(fi, &func);
                    self.stack.push(constant_to_val(&func.chunk.constants[idx]));
                }
                Op::Null => self.stack.push(Val::Null),
                Op::True => self.stack.push(Val::Bool(true)),
                Op::False => self.stack.push(Val::Bool(false)),
                Op::Pop => {
                    self.stack.pop();
                }
                Op::DefineGlobal => {
                    let idx = self.read_u16(fi, &func);
                    let name = self.const_str(&func, idx, line)?;
                    let v = self.pop(line)?;
                    self.globals.insert(name, v);
                }
                Op::GetGlobal => {
                    let idx = self.read_u16(fi, &func);
                    let name = self.const_str(&func, idx, line)?;
                    match self.globals.get(&name) {
                        Some(v) => self.stack.push(v.clone()),
                        None => {
                            return Err(VmError {
                                line,
                                msg: format!("undefined variable '{name}'"),
                            })
                        }
                    }
                }
                Op::SetGlobal => {
                    let idx = self.read_u16(fi, &func);
                    let name = self.const_str(&func, idx, line)?;
                    if !self.globals.contains_key(&name) {
                        return Err(VmError {
                            line,
                            msg: format!("assignment to undefined variable '{name}'"),
                        });
                    }
                    let v = self.peek(line)?.clone();
                    self.globals.insert(name, v);
                }
                Op::GetLocal => {
                    let slot = self.read_u8(fi, &func);
                    let base = self.frames[fi].slot_base;
                    self.stack.push(self.stack[base + slot as usize].clone());
                }
                Op::SetLocal => {
                    let slot = self.read_u8(fi, &func);
                    let base = self.frames[fi].slot_base;
                    let v = self.peek(line)?.clone();
                    self.stack[base + slot as usize] = v;
                }
                Op::Add => self.binary_add(line)?,
                Op::Sub => self.binary_num(line, "subtraction", |a, b| a - b)?,
                Op::Mul => self.binary_num(line, "multiplication", |a, b| a * b)?,
                Op::Div => self.binary_div(line)?,
                Op::Mod => self.binary_mod(line)?,
                Op::Pow => self.binary_num(line, "exponentiation", |a, b| a.powf(b))?,
                Op::BitAnd => self.binary_bit(line, "&", |a, b| a & b)?,
                Op::BitOr => self.binary_bit(line, "|", |a, b| a | b)?,
                Op::BitXor => self.binary_bit(line, "^", |a, b| a ^ b)?,
                Op::Shl => self.binary_bit(line, "<<", |a, b| a << (b as u32))?,
                Op::Shr => self.binary_bit(line, ">>", |a, b| a >> (b as u32))?,
                Op::ShrU => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    match (a, b) {
                        (Val::Number(x), Val::Number(y)) => self
                            .stack
                            .push(Val::Number((((x as u64) >> (y as u32)) as i64) as f64)),
                        (a, b) => return Err(type_err(line, ">>>", &a, &b)),
                    }
                }
                Op::Neg => {
                    let v = self.pop(line)?;
                    match v {
                        Val::Number(n) => self.stack.push(Val::Number(-n)),
                        other => {
                            return Err(VmError {
                                line,
                                msg: format!("cannot negate {}", other.type_name()),
                            })
                        }
                    }
                }
                Op::Not => {
                    let v = self.pop(line)?;
                    self.stack.push(Val::Bool(!v.truthy()));
                }
                Op::Equal => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    self.stack.push(Val::Bool(a.equals(&b)));
                }
                Op::NotEqual => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    self.stack.push(Val::Bool(!a.equals(&b)));
                }
                Op::Less => self.binary_cmp(line, "<", |o| o.is_lt())?,
                Op::LessEqual => self.binary_cmp(line, "<=", |o| o.is_le())?,
                Op::Greater => self.binary_cmp(line, ">", |o| o.is_gt())?,
                Op::GreaterEqual => self.binary_cmp(line, ">=", |o| o.is_ge())?,
                Op::Jump => {
                    let offset = self.read_u16(fi, &func);
                    self.frames[fi].ip += offset;
                }
                Op::JumpIfFalse => {
                    let offset = self.read_u16(fi, &func);
                    if !self.peek(line)?.truthy() {
                        self.frames[fi].ip += offset;
                    }
                }
                Op::Loop => {
                    let offset = self.read_u16(fi, &func);
                    self.frames[fi].ip -= offset;
                }
                Op::Print => {
                    let v = self.pop(line)?;
                    self.output.push_str(&v.to_string());
                    self.output.push('\n');
                }
                Op::Call => {
                    let argc = self.read_u8(fi, &func) as usize;
                    self.call(argc, line)?;
                    fi = self.frames.len() - 1;
                    func = Rc::clone(&self.frames[fi].func);
                }
                Op::Return => {
                    let result = self.pop(line)?;
                    let base = self.frames[fi].slot_base;
                    self.frames.pop();
                    if self.frames.is_empty() {
                        // Top-level script done.
                        self.stack.truncate(base);
                        return Ok(());
                    }
                    self.stack.truncate(base);
                    self.stack.push(result);
                    fi = self.frames.len() - 1;
                    func = Rc::clone(&self.frames[fi].func);
                }
            }
        }
    }

    fn call(&mut self, argc: usize, line: u32) -> Result<(), VmError> {
        let callee_index = self.stack.len() - argc - 1;
        let callee = self.stack[callee_index].clone();
        match callee {
            Val::Fn(f) => {
                if f.arity != argc {
                    return Err(VmError {
                        line,
                        msg: format!(
                            "function '{}' expects {} argument(s), got {argc}",
                            f.name, f.arity
                        ),
                    });
                }
                if self.frames.len() >= 1024 {
                    return Err(VmError {
                        line,
                        msg: "stack overflow (call depth exceeded 1024)".to_string(),
                    });
                }
                self.frames.push(Frame {
                    func: f,
                    ip: 0,
                    slot_base: callee_index,
                });
                Ok(())
            }
            other => Err(VmError {
                line,
                msg: format!("'{}' is not callable", other.type_name()),
            }),
        }
    }

    // ── operand decoding (advances the current frame's ip) ──────────────────

    fn read_u16(&mut self, fi: usize, func: &Function) -> usize {
        let ip = self.frames[fi].ip;
        let v = func.chunk.read_u16(ip) as usize;
        self.frames[fi].ip += 2;
        v
    }

    fn read_u8(&mut self, fi: usize, func: &Function) -> u8 {
        let ip = self.frames[fi].ip;
        let v = func.chunk.code[ip];
        self.frames[fi].ip += 1;
        v
    }

    fn const_str(&self, func: &Function, idx: usize, line: u32) -> Result<String, VmError> {
        match &func.chunk.constants[idx] {
            Constant::Str(s) => Ok(s.to_string()),
            other => Err(VmError {
                line,
                msg: format!("expected a name constant, found {other:?}"),
            }),
        }
    }

    // ── stack helpers ───────────────────────────────────────────────────────

    fn pop(&mut self, line: u32) -> Result<Val, VmError> {
        self.stack.pop().ok_or_else(|| VmError {
            line,
            msg: "stack underflow".to_string(),
        })
    }

    fn peek(&self, line: u32) -> Result<&Val, VmError> {
        self.stack.last().ok_or_else(|| VmError {
            line,
            msg: "stack underflow".to_string(),
        })
    }

    // ── arithmetic / comparison (mirrors interpreter semantics) ─────────────

    fn binary_add(&mut self, line: u32) -> Result<(), VmError> {
        let b = self.pop(line)?;
        let a = self.pop(line)?;
        let v = match (&a, &b) {
            (Val::Number(x), Val::Number(y)) => Val::Number(x + y),
            (Val::Str(_), _) => Val::Str(Rc::from(format!("{a}{b}").as_str())),
            (_, Val::Str(_)) => Val::Str(Rc::from(format!("{a}{b}").as_str())),
            _ => return Err(type_err(line, "addition", &a, &b)),
        };
        self.stack.push(v);
        Ok(())
    }

    fn binary_num<F: Fn(f64, f64) -> f64>(
        &mut self,
        line: u32,
        what: &str,
        f: F,
    ) -> Result<(), VmError> {
        let b = self.pop(line)?;
        let a = self.pop(line)?;
        match (&a, &b) {
            (Val::Number(x), Val::Number(y)) => {
                self.stack.push(Val::Number(f(*x, *y)));
                Ok(())
            }
            _ => Err(type_err(line, what, &a, &b)),
        }
    }

    fn binary_div(&mut self, line: u32) -> Result<(), VmError> {
        let b = self.pop(line)?;
        let a = self.pop(line)?;
        match (&a, &b) {
            (Val::Number(x), Val::Number(y)) => {
                if *y == 0.0 {
                    return Err(VmError {
                        line,
                        msg: "division by zero".to_string(),
                    });
                }
                self.stack.push(Val::Number(x / y));
                Ok(())
            }
            _ => Err(type_err(line, "division", &a, &b)),
        }
    }

    fn binary_mod(&mut self, line: u32) -> Result<(), VmError> {
        let b = self.pop(line)?;
        let a = self.pop(line)?;
        match (&a, &b) {
            (Val::Number(x), Val::Number(y)) => {
                if *y == 0.0 {
                    return Err(VmError {
                        line,
                        msg: "division by zero".to_string(),
                    });
                }
                self.stack.push(Val::Number(x % y));
                Ok(())
            }
            _ => Err(type_err(line, "modulo", &a, &b)),
        }
    }

    fn binary_bit<F: Fn(i64, i64) -> i64>(
        &mut self,
        line: u32,
        what: &str,
        f: F,
    ) -> Result<(), VmError> {
        let b = self.pop(line)?;
        let a = self.pop(line)?;
        match (&a, &b) {
            (Val::Number(x), Val::Number(y)) => {
                self.stack.push(Val::Number(f(*x as i64, *y as i64) as f64));
                Ok(())
            }
            _ => Err(type_err(line, what, &a, &b)),
        }
    }

    fn binary_cmp<F: Fn(std::cmp::Ordering) -> bool>(
        &mut self,
        line: u32,
        what: &str,
        f: F,
    ) -> Result<(), VmError> {
        let b = self.pop(line)?;
        let a = self.pop(line)?;
        let ord = match (&a, &b) {
            (Val::Number(x), Val::Number(y)) => {
                x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Val::Str(x), Val::Str(y)) => x.cmp(y),
            _ => return Err(type_err(line, what, &a, &b)),
        };
        self.stack.push(Val::Bool(f(ord)));
        Ok(())
    }
}

fn constant_to_val(c: &Constant) -> Val {
    match c {
        Constant::Number(n) => Val::Number(*n),
        Constant::Str(s) => Val::Str(Rc::clone(s)),
        Constant::Bool(b) => Val::Bool(*b),
        Constant::Null => Val::Null,
        Constant::Function(f) => Val::Fn(Rc::clone(f)),
    }
}

fn type_err(line: u32, op: &str, a: &Val, b: &Val) -> VmError {
    VmError {
        line,
        msg: format!(
            "cannot apply {op} to {} and {}",
            a.type_name(),
            b.type_name()
        ),
    }
}
