//! Bytecode representation for the custom-lang VM.
//!
//! This is a *real* compilation target: programs are lowered to a flat opcode
//! stream over a stack machine, can be serialized to a `.clbc` binary and read
//! back, and execute independently of the tree-walking interpreter. It covers a
//! coherent core sub-language (literals, arithmetic/logic/comparison, globals +
//! lexical locals, full control flow, and first-class functions with
//! recursion). Constructs outside that core are rejected at compile time with a
//! clear error rather than silently mis-compiled.

use std::fmt;
use std::rc::Rc;

/// A single VM instruction. Operands are encoded inline in the byte stream
/// (see [`Chunk`]); this enum is only the operationcode tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    Constant = 0, // u16 constant index
    Null = 1,
    True = 2,
    False = 3,
    Pop = 4,
    DefineGlobal = 5, // u16 name-constant index
    GetGlobal = 6,    // u16
    SetGlobal = 7,    // u16
    GetLocal = 8,     // u8 slot
    SetLocal = 9,     // u8 slot
    Add = 10,
    Sub = 11,
    Mul = 12,
    Div = 13,
    Mod = 14,
    Pow = 15,
    Neg = 16,
    Not = 17,
    Equal = 18,
    NotEqual = 19,
    Less = 20,
    LessEqual = 21,
    Greater = 22,
    GreaterEqual = 23,
    BitAnd = 24,
    BitOr = 25,
    BitXor = 26,
    Shl = 27,
    Shr = 28,
    ShrU = 29,
    Jump = 30,        // u16 forward offset
    JumpIfFalse = 31, // u16 forward offset (peeks, does not pop)
    Loop = 32,        // u16 backward offset
    Print = 33,
    Call = 34, // u8 arg count
    Return = 35,
}

impl Op {
    pub fn from_u8(b: u8) -> Option<Op> {
        if b <= Op::Return as u8 {
            // Safe: contiguous 0..=Return discriminants with explicit repr(u8).
            Some(unsafe { std::mem::transmute::<u8, Op>(b) })
        } else {
            None
        }
    }
}

/// A compiled function (also used for the top-level script, named `<main>`).
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
}

/// A constant-pool entry. Functions nest recursively.
#[derive(Debug, Clone)]
pub enum Constant {
    Number(f64),
    Str(Rc<str>),
    Bool(bool),
    Null,
    Function(Rc<Function>),
}

/// A flat instruction stream plus its constant pool and per-byte line numbers.
#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Constant>,
    pub lines: Vec<u32>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk::default()
    }

    /// Emit one opcode byte.
    pub fn emit(&mut self, op: Op, line: u32) {
        self.code.push(op as u8);
        self.lines.push(line);
    }

    /// Emit a raw operand byte.
    pub fn emit_byte(&mut self, b: u8, line: u32) {
        self.code.push(b);
        self.lines.push(line);
    }

    /// Emit a u16 operand (little-endian).
    pub fn emit_u16(&mut self, v: u16, line: u32) {
        self.code.push((v & 0xff) as u8);
        self.code.push((v >> 8) as u8);
        self.lines.push(line);
        self.lines.push(line);
    }

    pub fn read_u16(&self, offset: usize) -> u16 {
        (self.code[offset] as u16) | ((self.code[offset + 1] as u16) << 8)
    }

    /// Add a constant, returning its index (deduplicating simple values).
    pub fn add_constant(&mut self, c: Constant) -> usize {
        if let Some(i) = self.constants.iter().position(|e| e.same_as(&c)) {
            return i;
        }
        self.constants.push(c);
        self.constants.len() - 1
    }

    pub fn line_at(&self, ip: usize) -> u32 {
        self.lines.get(ip).copied().unwrap_or(0)
    }
}

impl Constant {
    fn same_as(&self, other: &Constant) -> bool {
        match (self, other) {
            (Constant::Number(a), Constant::Number(b)) => a.to_bits() == b.to_bits(),
            (Constant::Str(a), Constant::Str(b)) => a == b,
            (Constant::Bool(a), Constant::Bool(b)) => a == b,
            (Constant::Null, Constant::Null) => true,
            // Functions are never deduplicated.
            _ => false,
        }
    }
}

// ─── Binary serialization (.clbc) ────────────────────────────────────────────

const MAGIC: &[u8; 4] = b"CLBC";
const VERSION: u8 = 1;

/// Serialize a top-level function to a self-describing `.clbc` byte buffer.
pub fn serialize(main: &Function) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    write_function(&mut out, main);
    out
}

/// Parse a `.clbc` byte buffer back into a top-level function.
pub fn deserialize(bytes: &[u8]) -> Result<Function, String> {
    if bytes.len() < 5 || &bytes[0..4] != MAGIC {
        return Err("not a valid .clbc file (bad magic)".to_string());
    }
    if bytes[4] != VERSION {
        return Err(format!(
            "unsupported .clbc version {} (expected {VERSION})",
            bytes[4]
        ));
    }
    let mut cur = Cursor { bytes, pos: 5 };
    read_function(&mut cur)
}

fn write_function(out: &mut Vec<u8>, f: &Function) {
    write_str(out, &f.name);
    write_u32(out, f.arity as u32);
    write_chunk(out, &f.chunk);
}

fn write_chunk(out: &mut Vec<u8>, c: &Chunk) {
    write_u32(out, c.code.len() as u32);
    out.extend_from_slice(&c.code);
    write_u32(out, c.lines.len() as u32);
    for &l in &c.lines {
        write_u32(out, l);
    }
    write_u32(out, c.constants.len() as u32);
    for k in &c.constants {
        match k {
            Constant::Number(n) => {
                out.push(0);
                out.extend_from_slice(&n.to_le_bytes());
            }
            Constant::Str(s) => {
                out.push(1);
                write_str(out, s);
            }
            Constant::Bool(b) => {
                out.push(2);
                out.push(*b as u8);
            }
            Constant::Null => out.push(3),
            Constant::Function(func) => {
                out.push(4);
                write_function(out, func);
            }
        }
    }
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    write_u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        if self.pos + n > self.bytes.len() {
            return Err("unexpected end of bytecode".to_string());
        }
        let s = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f64(&mut self) -> Result<f64, String> {
        let b = self.take(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    fn string(&mut self) -> Result<String, String> {
        let len = self.u32()? as usize;
        let b = self.take(len)?;
        String::from_utf8(b.to_vec()).map_err(|_| "invalid utf-8 in bytecode".to_string())
    }
}

fn read_function(cur: &mut Cursor) -> Result<Function, String> {
    let name = cur.string()?;
    let arity = cur.u32()? as usize;
    let chunk = read_chunk(cur)?;
    Ok(Function { name, arity, chunk })
}

fn read_chunk(cur: &mut Cursor) -> Result<Chunk, String> {
    let code_len = cur.u32()? as usize;
    let code = cur.take(code_len)?.to_vec();
    let lines_len = cur.u32()? as usize;
    let mut lines = Vec::with_capacity(lines_len);
    for _ in 0..lines_len {
        lines.push(cur.u32()?);
    }
    let const_count = cur.u32()? as usize;
    let mut constants = Vec::with_capacity(const_count);
    for _ in 0..const_count {
        let tag = cur.u8()?;
        let k = match tag {
            0 => Constant::Number(cur.f64()?),
            1 => Constant::Str(Rc::from(cur.string()?.as_str())),
            2 => Constant::Bool(cur.u8()? != 0),
            3 => Constant::Null,
            4 => Constant::Function(Rc::new(read_function(cur)?)),
            other => return Err(format!("unknown constant tag {other}")),
        };
        constants.push(k);
    }
    Ok(Chunk {
        code,
        constants,
        lines,
    })
}

// ─── Disassembly (debug aid) ─────────────────────────────────────────────────

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "== <fn {} arity={}> ==", self.name, self.arity)?;
        let chunk = &self.chunk;
        let mut ip = 0;
        while ip < chunk.code.len() {
            write!(f, "{ip:04} ")?;
            let op = match Op::from_u8(chunk.code[ip]) {
                Some(o) => o,
                None => {
                    writeln!(f, "BAD_OP {}", chunk.code[ip])?;
                    ip += 1;
                    continue;
                }
            };
            ip += 1;
            match op {
                Op::Constant | Op::DefineGlobal | Op::GetGlobal | Op::SetGlobal => {
                    let idx = chunk.read_u16(ip);
                    ip += 2;
                    writeln!(f, "{op:?} {idx} ; {:?}", chunk.constants.get(idx as usize))?;
                }
                Op::GetLocal | Op::SetLocal | Op::Call => {
                    let b = chunk.code[ip];
                    ip += 1;
                    writeln!(f, "{op:?} {b}")?;
                }
                Op::Jump | Op::JumpIfFalse | Op::Loop => {
                    let off = chunk.read_u16(ip);
                    ip += 2;
                    writeln!(f, "{op:?} {off}")?;
                }
                _ => writeln!(f, "{op:?}")?,
            }
        }
        // Nested functions.
        for k in &chunk.constants {
            if let Constant::Function(func) = k {
                write!(f, "{func}")?;
            }
        }
        Ok(())
    }
}
