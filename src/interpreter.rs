use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;

use crate::ast::*;
use crate::env::{Env, EnvRef};
use crate::error::{CustomLangError, Result};

const MAX_CALL_DEPTH: usize = 500;

#[derive(Debug, Clone)]
pub enum Signal {
    None,
    ExprValue(Value),
    Return(Value),
    Break,
    Continue,
}

pub struct Interpreter {
    pub env: EnvRef,
    call_depth: usize,
    pub thrown_value: Option<Value>,
}

impl Interpreter {
    pub fn new() -> Self {
        let env = Env::root();
        Self::register_builtins(&env);
        Self { env, call_depth: 0, thrown_value: None }
    }

    pub fn interpret(&mut self, program: &Program) -> Result<()> {
        for stmt in &program.stmts {
            match self.exec_stmt(stmt)? {
                Signal::Return(_) => return Err(CustomLangError::runtime("cannot use 'return' at top level")),
                Signal::Break | Signal::Continue => return Err(CustomLangError::runtime("'break'/'continue' outside loop")),
                Signal::None | Signal::ExprValue(_) => {}
            }
        }
        Ok(())
    }

    pub fn exec_repl(&mut self, program: &Program) -> Result<Option<Value>> {
        let mut last = None;
        for stmt in &program.stmts {
            match self.exec_stmt(stmt)? {
                Signal::ExprValue(v) => last = Some(v),
                Signal::Return(v) => last = Some(v),
                Signal::None | Signal::Break | Signal::Continue => {}
            }
        }
        Ok(last)
    }

    // ─── statements ───────────────────────────────────────────────────────────

    pub fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Signal> {
        match stmt {
            Stmt::Expr { expr, .. } => {
                let val = self.eval_expr(expr)?;
                Ok(Signal::ExprValue(val))
            }
            Stmt::Let { name, init, .. } => {
                let val = match init {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Null,
                };
                Env::define(&self.env, name, val);
                Ok(Signal::None)
            }
            Stmt::Block { stmts, .. } => self.exec_block(stmts),
            Stmt::If { cond, then_b, else_b, .. } => {
                let c = self.eval_expr(cond)?;
                if c.is_truthy() {
                    self.exec_stmt(then_b)
                } else if let Some(e) = else_b {
                    self.exec_stmt(e)
                } else {
                    Ok(Signal::None)
                }
            }
            Stmt::While { cond, body, .. } => {
                loop {
                    if !self.eval_expr(cond)?.is_truthy() { break; }
                    match self.exec_stmt(body)? {
                        Signal::Return(v) => return Ok(Signal::Return(v)),
                        Signal::Break => break,
                        Signal::Continue => continue,
                        Signal::None | Signal::ExprValue(_) => {}
                    }
                }
                Ok(Signal::None)
            }
            Stmt::DoWhile { body, cond, .. } => {
                loop {
                    match self.exec_stmt(body)? {
                        Signal::Return(v) => return Ok(Signal::Return(v)),
                        Signal::Break => break,
                        Signal::Continue => {}
                        Signal::None | Signal::ExprValue(_) => {}
                    }
                    if !self.eval_expr(cond)?.is_truthy() { break; }
                }
                Ok(Signal::None)
            }
            Stmt::For { init, cond, update, body, .. } => {
                let loop_env = Env::child(&self.env);
                let outer = std::mem::replace(&mut self.env, loop_env);
                if let Some(i) = init { self.exec_stmt(i)?; }
                let result = loop {
                    if let Some(c) = cond {
                        if !self.eval_expr(c)?.is_truthy() { break Ok(Signal::None); }
                    }
                    match self.exec_stmt(body) {
                        Err(e) => break Err(e),
                        Ok(Signal::Return(v)) => break Ok(Signal::Return(v)),
                        Ok(Signal::Break) => break Ok(Signal::None),
                        Ok(Signal::Continue) | Ok(Signal::None) | Ok(Signal::ExprValue(_)) => {}
                    }
                    if let Some(u) = update {
                        if let Err(e) = self.eval_expr(u) { break Err(e); }
                    }
                };
                self.env = outer;
                result
            }
            Stmt::ForIn { var, iter, body, .. } => {
                let iter_val = self.eval_expr(iter)?;
                let items: Vec<Value> = match &iter_val {
                    Value::Array(arr) => arr.borrow().clone(),
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    Value::Object(obj) => obj.borrow().keys().map(|k| Value::Str(k.clone())).collect(),
                    _ => return Err(CustomLangError::type_err(format!("cannot iterate over {}", iter_val.type_name()))),
                };
                let loop_env = Env::child(&self.env);
                let outer = std::mem::replace(&mut self.env, loop_env);
                let mut result = Ok(Signal::None);
                for item in items {
                    Env::define(&self.env, var, item);
                    match self.exec_stmt(body) {
                        Err(e) => { result = Err(e); break; }
                        Ok(Signal::Return(v)) => { result = Ok(Signal::Return(v)); break; }
                        Ok(Signal::Break) => break,
                        Ok(Signal::Continue) | Ok(Signal::None) | Ok(Signal::ExprValue(_)) => {}
                    }
                }
                self.env = outer;
                result
            }
            Stmt::Function { name, params, body, is_static: _, .. } => {
                let fd = Rc::new(FnData {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.env),
                });
                Env::define(&self.env, name, Value::Function(fd));
                Ok(Signal::None)
            }
            Stmt::Return { value, .. } => {
                let v = match value { Some(e) => self.eval_expr(e)?, None => Value::Null };
                Ok(Signal::Return(v))
            }
            Stmt::Break { .. } => Ok(Signal::Break),
            Stmt::Continue { .. } => Ok(Signal::Continue),
            Stmt::Print { expr, .. } => {
                let v = self.eval_expr(expr)?;
                println!("{v}");
                Ok(Signal::None)
            }
            Stmt::Import { path, alias, .. } => {
                self.exec_import(path, alias.as_deref())?;
                Ok(Signal::None)
            }
            Stmt::Export { name, .. } => {
                if Env::get(&self.env, name).is_none() {
                    return Err(CustomLangError::runtime(format!("cannot export undefined name '{name}'")));
                }
                Ok(Signal::None)
            }
            Stmt::Class { name, super_name, methods, .. } => {
                self.exec_class(name, super_name.as_deref(), methods)?;
                Ok(Signal::None)
            }
            Stmt::TryCatch { try_b, catch_var, catch_b, finally_b, .. } => {
                let try_result = self.exec_stmt(try_b);
                let sig = match try_result {
                    Ok(s) => Ok(s),
                    Err(CustomLangError::ThrownException) => {
                        let thrown = self.thrown_value.take().unwrap_or(Value::Null);
                        if let Some(cb) = catch_b {
                            let catch_env = Env::child(&self.env);
                            if let Some(var) = catch_var {
                                Env::define(&catch_env, var, thrown);
                            }
                            let outer = std::mem::replace(&mut self.env, catch_env);
                            let sig = self.exec_stmt(cb);
                            self.env = outer;
                            sig
                        } else {
                            Ok(Signal::None)
                        }
                    }
                    Err(e) => Err(e),
                };
                if let Some(fin) = finally_b {
                    self.exec_stmt(fin)?;
                }
                sig
            }
            Stmt::Throw { value, .. } => {
                let v = self.eval_expr(value)?;
                self.thrown_value = Some(v);
                Err(CustomLangError::ThrownException)
            }
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Signal> {
        let block_env = Env::child(&self.env);
        let outer = std::mem::replace(&mut self.env, block_env);
        let mut signal = Signal::None;
        for stmt in stmts {
            match self.exec_stmt(stmt)? {
                Signal::None | Signal::ExprValue(_) => {}
                s => { signal = s; break; }
            }
        }
        self.env = outer;
        Ok(signal)
    }

    // ─── expressions ──────────────────────────────────────────────────────────

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Literal { value, .. } => Ok(value.clone()),

            Expr::Var { name, pos } => Env::get(&self.env, name).ok_or_else(|| {
                let names = Env::all_names(&self.env);
                let hint = CustomLangError::find_similar(name, &names).map(|s| format!("did you mean '{s}'?"));
                CustomLangError::undef_var(name, hint).with_pos(pos)
            }),

            Expr::Assign { name, value, pos } => {
                let v = self.eval_expr(value)?;
                if Env::set(&self.env, name, v.clone()) {
                    Ok(v)
                } else {
                    let names = Env::all_names(&self.env);
                    let hint = CustomLangError::find_similar(name, &names)
                        .map(|s| format!("did you mean '{s}'? Or declare with 'let {name} = ...'"));
                    Err(CustomLangError::undef_var(name, hint).with_pos(pos))
                }
            }

            Expr::CompoundAssign { name, op, value, pos } => {
                let current = Env::get(&self.env, name)
                    .ok_or_else(|| CustomLangError::undef_var(name, None).with_pos(pos))?;
                // Short-circuit logical ops
                match op {
                    CompoundOp::LogicalOr => {
                        if current.is_truthy() {
                            if !Env::set(&self.env, name, current.clone()) {
                                return Err(CustomLangError::undef_var(name, None).with_pos(pos));
                            }
                            return Ok(current);
                        }
                        let rhs = self.eval_expr(value)?;
                        if !Env::set(&self.env, name, rhs.clone()) {
                            return Err(CustomLangError::undef_var(name, None).with_pos(pos));
                        }
                        return Ok(rhs);
                    }
                    CompoundOp::LogicalAnd => {
                        if !current.is_truthy() {
                            return Ok(current);
                        }
                        let rhs = self.eval_expr(value)?;
                        if !Env::set(&self.env, name, rhs.clone()) {
                            return Err(CustomLangError::undef_var(name, None).with_pos(pos));
                        }
                        return Ok(rhs);
                    }
                    CompoundOp::NullCoalesce => {
                        if !matches!(current, Value::Null) {
                            return Ok(current);
                        }
                        let rhs = self.eval_expr(value)?;
                        if !Env::set(&self.env, name, rhs.clone()) {
                            return Err(CustomLangError::undef_var(name, None).with_pos(pos));
                        }
                        return Ok(rhs);
                    }
                    _ => {}
                }
                let rhs = self.eval_expr(value)?;
                let new_val = self.apply_binop(&current, &op.to_binary(), &rhs, pos)?;
                if !Env::set(&self.env, name, new_val.clone()) {
                    return Err(CustomLangError::undef_var(name, None).with_pos(pos));
                }
                Ok(new_val)
            }

            Expr::IndexAssign { object, index, value, pos } => {
                let obj_val = self.eval_expr(object)?;
                let idx_val = self.eval_expr(index)?;
                let new_val = self.eval_expr(value)?;
                match (&obj_val, &idx_val) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let idx = *n as usize;
                        let mut arr = arr.borrow_mut();
                        if idx < arr.len() {
                            arr[idx] = new_val.clone();
                            Ok(new_val)
                        } else {
                            Err(CustomLangError::runtime(format!("array index {idx} out of bounds")).with_pos(pos))
                        }
                    }
                    (Value::Object(obj), Value::Str(key)) => {
                        obj.borrow_mut().insert(key.clone(), new_val.clone());
                        Ok(new_val)
                    }
                    _ => Err(CustomLangError::type_err(format!("cannot index-assign {} with {}", obj_val.type_name(), idx_val.type_name())).with_pos(pos)),
                }
            }

            Expr::PropAssign { object, prop, value, pos } => {
                let obj_val = self.eval_expr(object)?;
                let new_val = self.eval_expr(value)?;
                match &obj_val {
                    Value::Object(obj) => { obj.borrow_mut().insert(prop.clone(), new_val.clone()); Ok(new_val) }
                    Value::Instance(inst) => { inst.borrow_mut().fields.insert(prop.clone(), new_val.clone()); Ok(new_val) }
                    _ => Err(CustomLangError::type_err(format!("cannot set property '{}' on {}", prop, obj_val.type_name())).with_pos(pos)),
                }
            }

            Expr::Binary { left, op, right, pos } => {
                match op {
                    BinaryOp::And => {
                        let l = self.eval_expr(left)?;
                        return if !l.is_truthy() { Ok(l) } else { self.eval_expr(right) };
                    }
                    BinaryOp::Or => {
                        let l = self.eval_expr(left)?;
                        return if l.is_truthy() { Ok(l) } else { self.eval_expr(right) };
                    }
                    BinaryOp::NullCoalesce => {
                        let l = self.eval_expr(left)?;
                        return if !matches!(l, Value::Null) { Ok(l) } else { self.eval_expr(right) };
                    }
                    _ => {}
                }
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.apply_binop(&l, op, &r, pos)
            }

            Expr::Unary { op, expr, pos } => {
                let v = self.eval_expr(expr)?;
                match op {
                    UnaryOp::Minus => match v {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err(CustomLangError::type_err(format!("unary '-' requires number, got {}", v.type_name())).with_pos(pos)),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!v.is_truthy())),
                    UnaryOp::BitwiseNot => match v {
                        Value::Number(n) => Ok(Value::Number(!(n as i64) as f64)),
                        _ => Err(CustomLangError::type_err("'~' requires number").with_pos(pos)),
                    },
                    UnaryOp::Typeof => Ok(Value::Str(v.type_name().to_string())),
                }
            }

            Expr::Ternary { cond, then_e, else_e, .. } => {
                if self.eval_expr(cond)?.is_truthy() {
                    self.eval_expr(then_e)
                } else {
                    self.eval_expr(else_e)
                }
            }

            Expr::Call { callee, args, pos } => {
                match callee.as_ref() {
                    Expr::Prop { object, name, .. } => {
                        let receiver = self.eval_expr(object)?;
                        let method = self.get_method(&receiver, name, pos)?;
                        let arg_vals = self.eval_args(args)?;
                        self.call_with_this(method, Some(receiver), arg_vals, pos)
                    }
                    Expr::Super { .. } => {
                        // super(args) — call parent constructor
                        let super_cls = Env::get(&self.env, "__super_class__")
                            .ok_or_else(|| CustomLangError::runtime("'super()' used outside class method"))?;
                        let this_val = Env::get(&self.env, "this")
                            .ok_or_else(|| CustomLangError::runtime("'super()' used outside class method"))?;
                        if let Value::Class(cls) = super_cls {
                            if let Some(init_fd) = cls.methods.get("init") {
                                let arg_vals = self.eval_args(args)?;
                                self.call_fn(init_fd, arg_vals, Some(this_val), pos)?;
                            }
                        }
                        Ok(Value::Null)
                    }
                    _ => {
                        let func = self.eval_expr(callee)?;
                        let arg_vals = self.eval_args(args)?;
                        self.call_value(func, arg_vals, None, pos)
                    }
                }
            }

            Expr::Index { object, index, pos } => {
                let obj_val = self.eval_expr(object)?;
                let idx_val = self.eval_expr(index)?;
                self.eval_index(&obj_val, &idx_val, pos)
            }

            Expr::Prop { object, name, pos } => {
                let obj_val = self.eval_expr(object)?;
                self.get_property(&obj_val, name, pos)
            }

            Expr::OptionalProp { object, name, pos } => {
                let obj_val = self.eval_expr(object)?;
                if matches!(obj_val, Value::Null) { return Ok(Value::Null); }
                self.get_property(&obj_val, name, pos)
            }

            Expr::OptionalIndex { object, index, pos } => {
                let obj_val = self.eval_expr(object)?;
                if matches!(obj_val, Value::Null) { return Ok(Value::Null); }
                let idx_val = self.eval_expr(index)?;
                self.eval_index(&obj_val, &idx_val, pos)
            }

            Expr::OptionalCall { callee, args, pos } => {
                let func = self.eval_expr(callee)?;
                if matches!(func, Value::Null) { return Ok(Value::Null); }
                let arg_vals = self.eval_args(args)?;
                self.call_value(func, arg_vals, None, pos)
            }

            Expr::Array { elements, .. } => {
                let mut result = Vec::new();
                for elem in elements {
                    match elem {
                        Expr::Spread { expr, .. } => {
                            let v = self.eval_expr(expr)?;
                            match v {
                                Value::Array(arr) => result.extend(arr.borrow().clone()),
                                _ => result.push(v),
                            }
                        }
                        _ => result.push(self.eval_expr(elem)?),
                    }
                }
                Ok(Value::make_array(result))
            }

            Expr::Object { pairs, .. } => {
                let mut map = HashMap::new();
                for (key, v) in pairs {
                    match key {
                        ObjectKey::Static(k) => {
                            // Check for spread: value is Expr::Spread
                            if let Expr::Spread { expr, .. } = v {
                                let spread_val = self.eval_expr(expr)?;
                                if let Value::Object(obj) = spread_val {
                                    for (sk, sv) in obj.borrow().iter() {
                                        map.insert(sk.clone(), sv.clone());
                                    }
                                }
                            } else {
                                map.insert(k.clone(), self.eval_expr(v)?);
                            }
                        }
                        ObjectKey::Computed(key_expr) => {
                            // Check for spread marker
                            if let Expr::Spread { expr, .. } = v {
                                let spread_val = self.eval_expr(expr)?;
                                if let Value::Object(obj) = spread_val {
                                    for (sk, sv) in obj.borrow().iter() {
                                        map.insert(sk.clone(), sv.clone());
                                    }
                                }
                            } else {
                                let k = self.eval_expr(key_expr)?;
                                let key_str = k.to_string();
                                map.insert(key_str, self.eval_expr(v)?);
                            }
                        }
                    }
                }
                Ok(Value::make_object(map))
            }

            Expr::Spread { pos, .. } => {
                Err(CustomLangError::runtime("spread operator used in invalid context").with_pos(pos))
            }

            Expr::New { class, args, pos } => {
                let class_val = self.eval_expr(class)?;
                let arg_vals = self.eval_args(args)?;
                self.instantiate(class_val, arg_vals, pos)
            }

            Expr::This { pos } => Env::get(&self.env, "this")
                .ok_or_else(|| CustomLangError::runtime("'this' used outside of a class method").with_pos(pos)),

            Expr::Super { pos } => {
                // super as expression — evaluate to a proxy-like value
                // We'll represent it as the superclass Value::Class
                Env::get(&self.env, "__super_class__")
                    .ok_or_else(|| CustomLangError::runtime("'super' used outside class method").with_pos(pos))
            }

            Expr::Lambda { params, body, .. } => {
                let fd = Rc::new(FnData {
                    name: "<lambda>".to_string(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.env),
                });
                Ok(Value::Function(fd))
            }

            Expr::Match { expr, arms, pos } => {
                let val = self.eval_expr(expr)?;
                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &val)? {
                        // Check guard
                        if let Some(guard) = &arm.guard {
                            let match_env = Env::child(&self.env);
                            for (name, v) in &bindings {
                                Env::define(&match_env, name, v.clone());
                            }
                            let outer = std::mem::replace(&mut self.env, match_env);
                            let guard_val = self.eval_expr(guard);
                            self.env = outer;
                            if !guard_val?.is_truthy() { continue; }
                        }
                        let match_env = Env::child(&self.env);
                        for (name, v) in bindings {
                            Env::define(&match_env, &name, v);
                        }
                        let outer = std::mem::replace(&mut self.env, match_env);
                        let result = self.eval_expr(&arm.body);
                        self.env = outer;
                        return result;
                    }
                }
                Err(CustomLangError::runtime("no match arm matched the value").with_pos(pos))
            }
        }
    }

    // ─── operators ────────────────────────────────────────────────────────────

    fn apply_binop(&self, l: &Value, op: &BinaryOp, r: &Value, pos: &Position) -> Result<Value> {
        match op {
            BinaryOp::Add => self.op_add(l, r, pos),
            BinaryOp::Subtract => self.numeric_op(l, r, op, pos, |a, b| a - b),
            BinaryOp::Multiply => self.numeric_op(l, r, op, pos, |a, b| a * b),
            BinaryOp::Divide => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    if *b == 0.0 { return Err(CustomLangError::DivisionByZero.with_pos(pos)); }
                    Ok(Value::Number(a / b))
                } else {
                    Err(self.type_err_binop("division", l, r, pos))
                }
            }
            BinaryOp::Modulo => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    if *b == 0.0 { return Err(CustomLangError::DivisionByZero.with_pos(pos)); }
                    Ok(Value::Number(a % b))
                } else {
                    Err(self.type_err_binop("modulo", l, r, pos))
                }
            }
            BinaryOp::Power => self.numeric_op(l, r, op, pos, |a, b| a.powf(b)),
            BinaryOp::BitwiseAnd => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    Ok(Value::Number(((*a as i64) & (*b as i64)) as f64))
                } else { Err(self.type_err_binop("&", l, r, pos)) }
            }
            BinaryOp::BitwiseOr => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    Ok(Value::Number(((*a as i64) | (*b as i64)) as f64))
                } else { Err(self.type_err_binop("|", l, r, pos)) }
            }
            BinaryOp::BitwiseXor => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    Ok(Value::Number(((*a as i64) ^ (*b as i64)) as f64))
                } else { Err(self.type_err_binop("^", l, r, pos)) }
            }
            BinaryOp::ShiftLeft => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    Ok(Value::Number(((*a as i64) << (*b as u32)) as f64))
                } else { Err(self.type_err_binop("<<", l, r, pos)) }
            }
            BinaryOp::ShiftRight => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    Ok(Value::Number(((*a as i64) >> (*b as u32)) as f64))
                } else { Err(self.type_err_binop(">>", l, r, pos)) }
            }
            BinaryOp::ShiftRightU => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    Ok(Value::Number((((*a as u64) >> (*b as u32)) as i64) as f64))
                } else { Err(self.type_err_binop(">>>", l, r, pos)) }
            }
            BinaryOp::Equal => Ok(Value::Bool(l.equals(r))),
            BinaryOp::NotEqual => Ok(Value::Bool(!l.equals(r))),
            BinaryOp::Less => self.compare_op(l, r, op, pos, |o| o.is_lt()),
            BinaryOp::LessEqual => self.compare_op(l, r, op, pos, |o| o.is_le()),
            BinaryOp::Greater => self.compare_op(l, r, op, pos, |o| o.is_gt()),
            BinaryOp::GreaterEqual => self.compare_op(l, r, op, pos, |o| o.is_ge()),
            BinaryOp::And => Ok(if l.is_truthy() { r.clone() } else { l.clone() }),
            BinaryOp::Or => Ok(if l.is_truthy() { l.clone() } else { r.clone() }),
            BinaryOp::NullCoalesce => Ok(if !matches!(l, Value::Null) { l.clone() } else { r.clone() }),
            BinaryOp::In => {
                match r {
                    Value::Object(obj) => {
                        if let Value::Str(key) = l {
                            Ok(Value::Bool(obj.borrow().contains_key(key.as_str())))
                        } else {
                            Ok(Value::Bool(false))
                        }
                    }
                    Value::Array(arr) => Ok(Value::Bool(arr.borrow().iter().any(|v| v.equals(l)))),
                    _ => Err(CustomLangError::type_err("'in' requires object or array on right side").with_pos(pos)),
                }
            }
            BinaryOp::Instanceof => {
                match r {
                    Value::Class(cls) => Ok(Value::Bool(l.is_instance_of(&cls.name))),
                    Value::Str(s) => Ok(Value::Bool(l.type_name() == s.as_str() || l.is_instance_of(s))),
                    _ => Ok(Value::Bool(false)),
                }
            }
        }
    }

    fn op_add(&self, l: &Value, r: &Value, pos: &Position) -> Result<Value> {
        match (l, r) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Array(a), Value::Array(b)) => {
                let mut v = a.borrow().clone();
                v.extend(b.borrow().clone());
                Ok(Value::make_array(v))
            }
            (Value::Str(a), other) if matches!(l, Value::Str(_)) => Ok(Value::Str(format!("{a}{other}"))),
            (other, Value::Str(b)) => Ok(Value::Str(format!("{other}{b}"))),
            _ => Err(self.type_err_binop("addition", l, r, pos)),
        }
    }

    fn numeric_op<F>(&self, l: &Value, r: &Value, op: &BinaryOp, pos: &Position, f: F) -> Result<Value>
    where F: Fn(f64, f64) -> f64 {
        match (l, r) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(f(*a, *b))),
            _ => Err(self.type_err_binop(&format!("{op:?}"), l, r, pos)),
        }
    }

    fn compare_op<F>(&self, l: &Value, r: &Value, op: &BinaryOp, pos: &Position, f: F) -> Result<Value>
    where F: Fn(std::cmp::Ordering) -> bool {
        let ord = match (l, r) {
            (Value::Number(a), Value::Number(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Str(a), Value::Str(b)) => a.cmp(b),
            _ => return Err(self.type_err_binop(&format!("{op:?}"), l, r, pos)),
        };
        Ok(Value::Bool(f(ord)))
    }

    fn type_err_binop(&self, op: &str, l: &Value, r: &Value, pos: &Position) -> CustomLangError {
        CustomLangError::type_err(format!("cannot apply {op} to {} and {}", l.type_name(), r.type_name())).with_pos(pos)
    }

    // ─── property / index access ──────────────────────────────────────────────

    fn eval_index(&self, obj: &Value, idx: &Value, pos: &Position) -> Result<Value> {
        match (obj, idx) {
            (Value::Array(arr), Value::Number(n)) => {
                let len = arr.borrow().len();
                let i = if *n < 0.0 {
                    let i = (len as f64 + n) as usize;
                    i
                } else {
                    *n as usize
                };
                arr.borrow().get(i).cloned().ok_or_else(|| {
                    CustomLangError::runtime(format!("array index {i} out of bounds (length {len})")).with_pos(pos)
                })
            }
            (Value::Str(s), Value::Number(n)) => {
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();
                let i = if *n < 0.0 { (len as f64 + n) as usize } else { *n as usize };
                chars.get(i).map(|c| Value::Str(c.to_string())).ok_or_else(|| {
                    CustomLangError::runtime(format!("string index {i} out of bounds (length {len})")).with_pos(pos)
                })
            }
            (Value::Object(obj), Value::Str(key)) => Ok(obj.borrow().get(key).cloned().unwrap_or(Value::Null)),
            _ => Err(CustomLangError::type_err(format!("cannot index {} with {}", obj.type_name(), idx.type_name())).with_pos(pos)),
        }
    }

    fn get_property(&self, obj: &Value, name: &str, pos: &Position) -> Result<Value> {
        match obj {
            Value::Instance(inst) => {
                let inst_b = inst.borrow();
                if let Some(v) = inst_b.fields.get(name) { return Ok(v.clone()); }
                if let Some(m) = inst_b.class.methods.get(name) { return Ok(Value::Function(Rc::clone(m))); }
                let mut super_cls = inst_b.class.superclass.clone();
                while let Some(sc) = super_cls {
                    if let Some(m) = sc.methods.get(name) { return Ok(Value::Function(Rc::clone(m))); }
                    super_cls = sc.superclass.clone();
                }
                Ok(Value::Null)
            }
            Value::Object(obj) => Ok(obj.borrow().get(name).cloned().unwrap_or(Value::Null)),
            Value::Array(arr) => {
                if name == "length" { return Ok(Value::Number(arr.borrow().len() as f64)); }
                Ok(Value::Null)
            }
            Value::Str(s) => {
                if name == "length" { return Ok(Value::Number(s.chars().count() as f64)); }
                Ok(Value::Null)
            }
            Value::Class(cls) => {
                // Static field or method access
                if let Some(v) = cls.static_fields.get(name) { return Ok(v.clone()); }
                if let Some(m) = cls.static_methods.get(name) { return Ok(Value::Function(Rc::clone(m))); }
                Ok(Value::Null)
            }
            _ => Err(CustomLangError::type_err(format!("cannot access property '{}' on {}", name, obj.type_name())).with_pos(pos)),
        }
    }

    fn get_method(&self, obj: &Value, name: &str, pos: &Position) -> Result<Value> {
        match obj {
            Value::Instance(inst) => {
                let inst_b = inst.borrow();
                if let Some(m) = inst_b.class.methods.get(name) { return Ok(Value::Function(Rc::clone(m))); }
                let mut super_cls = inst_b.class.superclass.clone();
                while let Some(sc) = super_cls {
                    if let Some(m) = sc.methods.get(name) { return Ok(Value::Function(Rc::clone(m))); }
                    super_cls = sc.superclass.clone();
                }
                if let Some(v) = inst_b.fields.get(name) { return Ok(v.clone()); }
                Err(CustomLangError::runtime(format!("instance of '{}' has no method '{name}'", inst_b.class.name)).with_pos(pos))
            }
            Value::Object(obj) => obj.borrow().get(name).cloned().ok_or_else(|| {
                CustomLangError::runtime(format!("object has no property '{name}'")).with_pos(pos)
            }),
            Value::Class(cls) => {
                if let Some(m) = cls.static_methods.get(name) { return Ok(Value::Function(Rc::clone(m))); }
                Err(CustomLangError::runtime(format!("class '{}' has no static method '{name}'", cls.name)).with_pos(pos))
            }
            _ => Err(CustomLangError::type_err(format!("cannot call method '{name}' on {}", obj.type_name())).with_pos(pos)),
        }
    }

    // ─── function calls ───────────────────────────────────────────────────────

    fn eval_args(&mut self, args: &[Expr]) -> Result<Vec<Value>> {
        let mut result = Vec::new();
        for arg in args {
            match arg {
                Expr::Spread { expr, .. } => {
                    let v = self.eval_expr(expr)?;
                    match v {
                        Value::Array(arr) => result.extend(arr.borrow().clone()),
                        _ => result.push(v),
                    }
                }
                _ => result.push(self.eval_expr(arg)?),
            }
        }
        Ok(result)
    }

    pub fn call_value(&mut self, func: Value, args: Vec<Value>, this: Option<Value>, pos: &Position) -> Result<Value> {
        match func {
            Value::Function(fd) => self.call_fn(&fd, args, this, pos),
            Value::Builtin(name) => self.call_builtin(&name, args, pos),
            Value::Class(cls) => self.instantiate(Value::Class(cls), args, pos),
            _ => Err(CustomLangError::type_err(format!("cannot call value of type {}", func.type_name())).with_pos(pos)),
        }
    }

    fn call_with_this(&mut self, func: Value, this: Option<Value>, args: Vec<Value>, pos: &Position) -> Result<Value> {
        self.call_value(func, args, this, pos)
    }

    fn call_fn(&mut self, fd: &Rc<FnData>, args: Vec<Value>, this: Option<Value>, pos: &Position) -> Result<Value> {
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(CustomLangError::StackOverflow.with_pos(pos));
        }

        let fn_env = Env::child(&fd.closure);
        if let Some(t) = this {
            // Store the super class so super.method() works
            if let Value::Instance(ref inst) = t {
                if let Some(ref super_cls) = inst.borrow().class.superclass {
                    Env::define(&fn_env, "__super_class__", Value::Class(Rc::clone(super_cls)));
                }
            }
            Env::define(&fn_env, "this", t);
        }
        Env::define(&fn_env, &fd.name, Value::Function(Rc::clone(fd)));

        // Bind parameters with defaults and rest support
        let mut arg_idx = 0;
        for (i, param) in fd.params.iter().enumerate() {
            if param.is_rest {
                // Rest param: collect remaining args
                let rest_vals: Vec<Value> = args[arg_idx..].to_vec();
                Env::define(&fn_env, &param.name, Value::make_array(rest_vals));
                break;
            } else {
                let val = if arg_idx < args.len() {
                    arg_idx += 1;
                    args[arg_idx - 1].clone()
                } else if let Some(default_expr) = &param.default {
                    // Evaluate default in the function's environment
                    let outer = std::mem::replace(&mut self.env, Rc::clone(&fn_env));
                    let default_val = self.eval_expr(default_expr);
                    self.env = outer;
                    default_val?
                } else {
                    if i < args.len() { args[i].clone() } else { Value::Null }
                };
                Env::define(&fn_env, &param.name, val);
            }
        }

        let outer = std::mem::replace(&mut self.env, fn_env);
        self.call_depth += 1;
        let result = self.exec_stmt(&fd.body);
        self.call_depth -= 1;
        self.env = outer;

        match result? {
            Signal::Return(v) => Ok(v),
            _ => Ok(Value::Null),
        }
    }

    // ─── class / instance ─────────────────────────────────────────────────────

    fn exec_class(&mut self, name: &str, super_name: Option<&str>, methods: &[Stmt]) -> Result<()> {
        let superclass = if let Some(sn) = super_name {
            match Env::get(&self.env, sn) {
                Some(Value::Class(c)) => Some(c),
                Some(_) => return Err(CustomLangError::runtime(format!("'{sn}' is not a class"))),
                None => return Err(CustomLangError::undef_var(sn, None)),
            }
        } else {
            None
        };

        let mut method_map = HashMap::new();
        let mut static_method_map = HashMap::new();
        let static_fields = HashMap::new();

        for method in methods {
            if let Stmt::Function { name: mn, params, body, is_static, .. } = method {
                let fd = Rc::new(FnData {
                    name: mn.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.env),
                });
                if *is_static {
                    static_method_map.insert(mn.clone(), fd);
                } else {
                    method_map.insert(mn.clone(), fd);
                }
            }
        }

        let cls = Rc::new(ClassData {
            name: name.to_string(),
            methods: method_map,
            static_methods: static_method_map,
            static_fields,
            superclass,
        });
        Env::define(&self.env, name, Value::Class(cls));
        Ok(())
    }

    fn instantiate(&mut self, class_val: Value, args: Vec<Value>, pos: &Position) -> Result<Value> {
        let cls = match class_val {
            Value::Class(c) => c,
            _ => return Err(CustomLangError::type_err(format!("cannot instantiate {}", class_val.type_name())).with_pos(pos)),
        };

        let inst = Rc::new(RefCell::new(InstanceData {
            class: Rc::clone(&cls),
            fields: HashMap::new(),
        }));
        let inst_val = Value::Instance(Rc::clone(&inst));

        if let Some(init_fd) = cls.methods.get("init") {
            self.call_fn(init_fd, args, Some(inst_val.clone()), pos)?;
        }
        Ok(inst_val)
    }

    // ─── pattern matching ─────────────────────────────────────────────────────

    fn match_pattern(&self, pattern: &Pattern, value: &Value) -> Result<Option<Vec<(String, Value)>>> {
        Ok(match pattern {
            Pattern::Number(n) => {
                if let Value::Number(v) = value { if (v - n).abs() < f64::EPSILON { Some(vec![]) } else { None } } else { None }
            }
            Pattern::Str(s) => {
                if let Value::Str(v) = value { if v == s { Some(vec![]) } else { None } } else { None }
            }
            Pattern::Bool(b) => {
                if let Value::Bool(v) = value { if v == b { Some(vec![]) } else { None } } else { None }
            }
            Pattern::Null => if matches!(value, Value::Null) { Some(vec![]) } else { None },
            Pattern::Wildcard => Some(vec![]),
            Pattern::Binding(name) => Some(vec![(name.clone(), value.clone())]),
            Pattern::Array(pats) => {
                if let Value::Array(arr) = value {
                    let arr = arr.borrow();
                    if pats.len() != arr.len() { return Ok(None); }
                    let mut bindings = Vec::new();
                    for (p, v) in pats.iter().zip(arr.iter()) {
                        match self.match_pattern(p, v)? {
                            Some(mut b) => bindings.append(&mut b),
                            None => return Ok(None),
                        }
                    }
                    Some(bindings)
                } else { None }
            }
            Pattern::Object(pairs) => {
                if let Value::Object(obj) = value {
                    let obj = obj.borrow();
                    let mut bindings = Vec::new();
                    for (key, pat) in pairs {
                        let v = obj.get(key).unwrap_or(&Value::Null);
                        match self.match_pattern(pat, v)? {
                            Some(mut b) => bindings.append(&mut b),
                            None => return Ok(None),
                        }
                    }
                    Some(bindings)
                } else { None }
            }
        })
    }

    // ─── import ───────────────────────────────────────────────────────────────

    fn exec_import(&mut self, path: &str, alias: Option<&str>) -> Result<()> {
        // Handle standard library modules
        if path.starts_with("std/") {
            return self.exec_std_import(path, alias);
        }

        let file_path = if path.ends_with(".cl") { path.to_string() } else { format!("{path}.cl") };
        let source = std::fs::read_to_string(&file_path).map_err(|e| {
            CustomLangError::io_err(format!("cannot read module '{file_path}': {e}"))
        })?;

        let tokens = crate::lexer::Lexer::new(&source).tokenize()
            .map_err(|e| CustomLangError::runtime(format!("error in module '{file_path}': {e}")))?;
        let program = crate::parser::Parser::new(tokens).parse()
            .map_err(|e| CustomLangError::runtime(format!("error in module '{file_path}': {e}")))?;

        let mod_env = Env::root();
        Self::register_builtins(&mod_env);
        let mut mod_interp = Interpreter { env: mod_env, call_depth: self.call_depth, thrown_value: None };
        mod_interp.interpret(&program)?;

        let mod_name = alias.unwrap_or_else(|| {
            std::path::Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or(path)
        });

        if alias.is_some() {
            let mut ns = HashMap::new();
            let names = Env::all_names(&mod_interp.env);
            for name in names {
                if let Some(v) = Env::get(&mod_interp.env, &name) {
                    ns.insert(name, v);
                }
            }
            Env::define(&self.env, mod_name, Value::make_object(ns));
        } else {
            let names = Env::all_names(&mod_interp.env);
            for name in names {
                if let Some(v) = Env::get(&mod_interp.env, &name) {
                    Env::define(&self.env, &name, v);
                }
            }
        }
        Ok(())
    }

    fn exec_std_import(&mut self, path: &str, alias: Option<&str>) -> Result<()> {
        let mod_name = alias.unwrap_or_else(|| path.trim_start_matches("std/"));
        let ns = match path {
            "std/json" => Self::make_json_module(),
            "std/random" => Self::make_random_module(),
            "std/math" => Self::make_math_module(),
            _ => return Err(CustomLangError::runtime(format!("unknown standard module '{path}'"))),
        };
        Env::define(&self.env, mod_name, Value::make_object(ns));
        Ok(())
    }

    fn make_json_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        m.insert("parse".to_string(), Value::Builtin("json_parse".to_string()));
        m.insert("stringify".to_string(), Value::Builtin("json_stringify".to_string()));
        m.insert("is_valid".to_string(), Value::Builtin("json_is_valid".to_string()));
        m
    }

    fn make_random_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        m.insert("float".to_string(), Value::Builtin("random_float".to_string()));
        m.insert("int".to_string(), Value::Builtin("random_int".to_string()));
        m.insert("bool".to_string(), Value::Builtin("random_bool".to_string()));
        m.insert("choice".to_string(), Value::Builtin("random_choice".to_string()));
        m.insert("shuffle".to_string(), Value::Builtin("random_shuffle".to_string()));
        m
    }

    fn make_math_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        m.insert("PI".to_string(), Value::Number(std::f64::consts::PI));
        m.insert("E".to_string(), Value::Number(std::f64::consts::E));
        m.insert("TAU".to_string(), Value::Number(std::f64::consts::TAU));
        m.insert("INFINITY".to_string(), Value::Number(f64::INFINITY));
        m.insert("NAN".to_string(), Value::Number(f64::NAN));
        m.insert("sqrt".to_string(), Value::Builtin("sqrt".to_string()));
        m.insert("abs".to_string(), Value::Builtin("abs".to_string()));
        m.insert("floor".to_string(), Value::Builtin("floor".to_string()));
        m.insert("ceil".to_string(), Value::Builtin("ceil".to_string()));
        m.insert("round".to_string(), Value::Builtin("round".to_string()));
        m.insert("sin".to_string(), Value::Builtin("sin".to_string()));
        m.insert("cos".to_string(), Value::Builtin("cos".to_string()));
        m.insert("tan".to_string(), Value::Builtin("tan".to_string()));
        m.insert("log".to_string(), Value::Builtin("log".to_string()));
        m.insert("pow".to_string(), Value::Builtin("pow".to_string()));
        m.insert("clamp".to_string(), Value::Builtin("math_clamp".to_string()));
        m.insert("sign".to_string(), Value::Builtin("math_sign".to_string()));
        m.insert("hypot".to_string(), Value::Builtin("math_hypot".to_string()));
        m.insert("gcd".to_string(), Value::Builtin("math_gcd".to_string()));
        m.insert("lcm".to_string(), Value::Builtin("math_lcm".to_string()));
        m.insert("factorial".to_string(), Value::Builtin("math_factorial".to_string()));
        m.insert("is_nan".to_string(), Value::Builtin("math_is_nan".to_string()));
        m.insert("is_finite".to_string(), Value::Builtin("math_is_finite".to_string()));
        m.insert("random".to_string(), Value::Builtin("random_float".to_string()));
        m.insert("random_int".to_string(), Value::Builtin("random_int".to_string()));
        m
    }

    // ─── builtins ─────────────────────────────────────────────────────────────

    fn register_builtins(env: &EnvRef) {
        let builtins = [
            "print","println","input","len","type","abs","sqrt","pow","min","max",
            "floor","ceil","round","log","sin","cos","tan","to_string","to_number",
            "to_bool","push","pop","shift","unshift","first","last","sort","reverse",
            "slice","includes","find","index_of","filter","map","reduce","for_each",
            "every","some","split","join","substring","to_upper","to_lower","trim",
            "trim_start","trim_end","starts_with","ends_with","contains","replace",
            "char_at","char_code","format","read_file","write_file","append_file",
            "keys","values","entries","has_key","delete_key","parse_int","parse_float",
            "is_number","is_string","is_bool","is_null","is_array","is_object",
            "assert","exit","now","range",
            // New builtins
            "json_parse","json_stringify","json_is_valid",
            "random_float","random_int","random_bool","random_choice","random_shuffle",
            "math_clamp","math_sign","math_hypot","math_gcd","math_lcm",
            "math_factorial","math_is_nan","math_is_finite",
            "flat","flat_map","zip","chunk","unique","count","sum","average",
            "repeat","pad_start","pad_end",
        ];
        for name in builtins {
            Env::define(env, name, Value::Builtin(name.to_string()));
        }
    }

    fn call_builtin(&mut self, name: &str, args: Vec<Value>, pos: &Position) -> Result<Value> {
        let argc = args.len();
        let err_argc = |expected: &str| {
            Err(CustomLangError::runtime(format!("{name}() expects {expected} argument(s), got {argc}")).with_pos(pos))
        };

        match name {
            "print" => {
                let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                print!("{}", parts.join(" "));
                let _ = io::stdout().flush();
                Ok(Value::Null)
            }
            "println" => {
                let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                println!("{}", parts.join(" "));
                Ok(Value::Null)
            }
            "input" => {
                if !args.is_empty() { print!("{}", args[0]); let _ = io::stdout().flush(); }
                let mut line = String::new();
                io::stdin().read_line(&mut line).map_err(|e| CustomLangError::io_err(format!("failed to read input: {e}")))?;
                Ok(Value::Str(line.trim_end_matches(['\n', '\r']).to_string()))
            }
            "type" => {
                if argc != 1 { return err_argc("1"); }
                Ok(Value::Str(args[0].type_name().to_string()))
            }
            "to_string" => {
                if argc != 1 { return err_argc("1"); }
                Ok(Value::Str(args[0].to_string()))
            }
            "to_number" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Number(n) => Ok(Value::Number(*n)),
                    Value::Str(s) => s.trim().parse::<f64>().map(Value::Number).map_err(|_| CustomLangError::type_err(format!("cannot convert '{s}' to number"))),
                    Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
                    Value::Null => Ok(Value::Number(0.0)),
                    v => Err(CustomLangError::type_err(format!("cannot convert {} to number", v.type_name()))),
                }
            }
            "to_bool" => {
                if argc != 1 { return err_argc("1"); }
                Ok(Value::Bool(args[0].is_truthy()))
            }
            "parse_int" => {
                if !(1..=2).contains(&argc) { return err_argc("1 or 2"); }
                let s = match &args[0] { Value::Str(s) => s.trim().to_string(), v => v.to_string() };
                let radix = if argc == 2 { match &args[1] { Value::Number(n) => *n as u32, _ => 10 } } else { 10 };
                i64::from_str_radix(&s, radix).map(|n| Value::Number(n as f64))
                    .map_err(|_| CustomLangError::runtime(format!("cannot parse '{s}' as integer")))
            }
            "parse_float" => {
                if argc != 1 { return err_argc("1"); }
                let s = args[0].to_string();
                s.trim().parse::<f64>().map(Value::Number).map_err(|_| CustomLangError::runtime(format!("cannot parse '{s}' as float")))
            }
            "is_number" => { if argc != 1 { return err_argc("1"); } Ok(Value::Bool(matches!(args[0], Value::Number(_)))) }
            "is_string" => { if argc != 1 { return err_argc("1"); } Ok(Value::Bool(matches!(args[0], Value::Str(_)))) }
            "is_bool" => { if argc != 1 { return err_argc("1"); } Ok(Value::Bool(matches!(args[0], Value::Bool(_)))) }
            "is_null" => { if argc != 1 { return err_argc("1"); } Ok(Value::Bool(matches!(args[0], Value::Null))) }
            "is_array" => { if argc != 1 { return err_argc("1"); } Ok(Value::Bool(matches!(args[0], Value::Array(_)))) }
            "is_object" => { if argc != 1 { return err_argc("1"); } Ok(Value::Bool(matches!(args[0], Value::Object(_)))) }
            "len" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Number(s.chars().count() as f64)),
                    Value::Array(a) => Ok(Value::Number(a.borrow().len() as f64)),
                    Value::Object(o) => Ok(Value::Number(o.borrow().len() as f64)),
                    v => Err(CustomLangError::type_err(format!("len() not supported for {}", v.type_name()))),
                }
            }
            "abs" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.abs())), ref v => Err(CustomLangError::type_err(format!("abs() requires number, got {}", v.type_name()))) } }
            "sqrt" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) if n >= 0.0 => Ok(Value::Number(n.sqrt())), Value::Number(_) => Err(CustomLangError::runtime("sqrt() of negative number")), ref v => Err(CustomLangError::type_err(format!("sqrt() requires number, got {}", v.type_name()))) } }
            "pow" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Number(b), Value::Number(e)) => Ok(Value::Number(b.powf(*e))), _ => Err(CustomLangError::type_err("pow() requires two numbers")) } }
            "min" => {
                if argc < 1 { return err_argc("1+"); }
                if argc == 2 { if let (Value::Number(a), Value::Number(b)) = (&args[0], &args[1]) { return Ok(Value::Number(a.min(*b))); } }
                let nums = Self::extract_numbers(&args, "min", pos)?;
                Ok(Value::Number(nums.into_iter().fold(f64::INFINITY, f64::min)))
            }
            "max" => {
                if argc < 1 { return err_argc("1+"); }
                if argc == 2 { if let (Value::Number(a), Value::Number(b)) = (&args[0], &args[1]) { return Ok(Value::Number(a.max(*b))); } }
                let nums = Self::extract_numbers(&args, "max", pos)?;
                Ok(Value::Number(nums.into_iter().fold(f64::NEG_INFINITY, f64::max)))
            }
            "floor" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.floor())), ref v => Err(CustomLangError::type_err(format!("floor() requires number, got {}", v.type_name()))) } }
            "ceil" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.ceil())), ref v => Err(CustomLangError::type_err(format!("ceil() requires number, got {}", v.type_name()))) } }
            "round" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.round())), ref v => Err(CustomLangError::type_err(format!("round() requires number, got {}", v.type_name()))) } }
            "log" => {
                if !(1..=2).contains(&argc) { return err_argc("1 or 2"); }
                match args[0] { Value::Number(n) => {
                    let result = if argc == 2 { match args[1] { Value::Number(base) => n.log(base), _ => return Err(CustomLangError::type_err("log() base must be a number")) } } else { n.ln() };
                    Ok(Value::Number(result))
                }, ref v => Err(CustomLangError::type_err(format!("log() requires number, got {}", v.type_name()))) }
            }
            "sin" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.sin())), _ => Err(CustomLangError::type_err("sin() requires number")) } }
            "cos" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.cos())), _ => Err(CustomLangError::type_err("cos() requires number")) } }
            "tan" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.tan())), _ => Err(CustomLangError::type_err("tan() requires number")) } }
            "push" => {
                if argc < 2 { return err_argc("2+"); }
                match &args[0] { Value::Array(arr) => { for v in &args[1..] { arr.borrow_mut().push(v.clone()); } Ok(args[0].clone()) }, v => Err(CustomLangError::type_err(format!("push() requires array, got {}", v.type_name()))) }
            }
            "pop" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Array(arr) => Ok(arr.borrow_mut().pop().unwrap_or(Value::Null)), v => Err(CustomLangError::type_err(format!("pop() requires array, got {}", v.type_name()))) } }
            "shift" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] { Value::Array(arr) => { let mut a = arr.borrow_mut(); if a.is_empty() { Ok(Value::Null) } else { Ok(a.remove(0)) } }, v => Err(CustomLangError::type_err(format!("shift() requires array, got {}", v.type_name()))) }
            }
            "unshift" => {
                if argc < 2 { return err_argc("2+"); }
                match &args[0] { Value::Array(arr) => { for (i, v) in args[1..].iter().enumerate() { arr.borrow_mut().insert(i, v.clone()); } Ok(args[0].clone()) }, v => Err(CustomLangError::type_err(format!("unshift() requires array, got {}", v.type_name()))) }
            }
            "first" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Array(arr) => Ok(arr.borrow().first().cloned().unwrap_or(Value::Null)), v => Err(CustomLangError::type_err(format!("first() requires array, got {}", v.type_name()))) } }
            "last" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Array(arr) => Ok(arr.borrow().last().cloned().unwrap_or(Value::Null)), v => Err(CustomLangError::type_err(format!("last() requires array, got {}", v.type_name()))) } }
            "sort" => {
                if !(1..=2).contains(&argc) { return err_argc("1 or 2"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut a = arr.borrow_mut();
                        if argc == 2 {
                            let cmp_fn = args[1].clone();
                            let mut error: Option<CustomLangError> = None;
                            a.sort_by(|x, y| {
                                if error.is_some() { return std::cmp::Ordering::Equal; }
                                match self.call_value(cmp_fn.clone(), vec![x.clone(), y.clone()], None, pos) {
                                    Ok(Value::Number(n)) => if n < 0.0 { std::cmp::Ordering::Less } else if n > 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Equal },
                                    Ok(_) => std::cmp::Ordering::Equal,
                                    Err(e) => { error = Some(e); std::cmp::Ordering::Equal }
                                }
                            });
                            if let Some(e) = error { return Err(e); }
                        } else {
                            a.sort_by(|x, y| match (x, y) {
                                (Value::Number(a), Value::Number(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                                (Value::Str(a), Value::Str(b)) => a.cmp(b),
                                _ => std::cmp::Ordering::Equal,
                            });
                        }
                        drop(a);
                        Ok(args[0].clone())
                    }
                    v => Err(CustomLangError::type_err(format!("sort() requires array, got {}", v.type_name()))),
                }
            }
            "reverse" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Array(arr) => { arr.borrow_mut().reverse(); Ok(args[0].clone()) }, v => Err(CustomLangError::type_err(format!("reverse() requires array, got {}", v.type_name()))) } }
            "slice" => {
                if !(2..=3).contains(&argc) { return err_argc("2 or 3"); }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::Number(start)) => {
                        let arr = arr.borrow();
                        let len = arr.len();
                        let s = if *start < 0.0 { (len as f64 + start).max(0.0) as usize } else { (*start as usize).min(len) };
                        let e = if argc == 3 { match args[2] { Value::Number(n) => if n < 0.0 { (len as f64 + n).max(0.0) as usize } else { (n as usize).min(len) }, _ => return Err(CustomLangError::type_err("slice() end must be number")) } } else { len };
                        let sliced: Vec<Value> = arr[s..e.max(s)].to_vec();
                        Ok(Value::make_array(sliced))
                    }
                    _ => Err(CustomLangError::type_err("slice() requires (array, number[, number])")),
                }
            }
            "includes" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => Ok(Value::Bool(arr.borrow().iter().any(|v| v.equals(&args[1])))),
                    Value::Str(s) => { if let Value::Str(sub) = &args[1] { Ok(Value::Bool(s.contains(sub.as_str()))) } else { Ok(Value::Bool(false)) } }
                    v => Err(CustomLangError::type_err(format!("includes() requires array or string, got {}", v.type_name()))),
                }
            }
            "find" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let needle = args[1].clone();
                        let arr = arr.borrow().clone();
                        match &needle {
                            Value::Function(_) | Value::Builtin(_) => {
                                for item in arr { let result = self.call_value(needle.clone(), vec![item.clone()], None, pos)?; if result.is_truthy() { return Ok(item); } }
                            }
                            _ => { for item in arr { if item.equals(&needle) { return Ok(item); } } }
                        }
                        Ok(Value::Null)
                    }
                    v => Err(CustomLangError::type_err(format!("find() requires array, got {}", v.type_name()))),
                }
            }
            "index_of" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => { let arr = arr.borrow(); let idx = arr.iter().position(|v| v.equals(&args[1])); Ok(Value::Number(idx.map(|i| i as f64).unwrap_or(-1.0))) }
                    Value::Str(s) => { if let Value::Str(sub) = &args[1] { Ok(Value::Number(s.find(sub.as_str()).map(|i| i as f64).unwrap_or(-1.0))) } else { Ok(Value::Number(-1.0)) } }
                    v => Err(CustomLangError::type_err(format!("index_of() requires array or string, got {}", v.type_name()))),
                }
            }
            "filter" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        let mut result = Vec::new();
                        for item in items { if self.call_value(callback.clone(), vec![item.clone()], None, pos)?.is_truthy() { result.push(item); } }
                        Ok(Value::make_array(result))
                    }
                    v => Err(CustomLangError::type_err(format!("filter() requires array, got {}", v.type_name()))),
                }
            }
            "map" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        let mut result = Vec::new();
                        for item in items { result.push(self.call_value(callback.clone(), vec![item], None, pos)?); }
                        Ok(Value::make_array(result))
                    }
                    v => Err(CustomLangError::type_err(format!("map() requires array, got {}", v.type_name()))),
                }
            }
            "reduce" => {
                if !(2..=3).contains(&argc) { return err_argc("2 or 3"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        let mut acc = if argc == 3 { args[2].clone() } else { items.first().cloned().unwrap_or(Value::Null) };
                        let start = if argc == 3 { 0 } else { 1 };
                        for item in items[start..].iter() { acc = self.call_value(callback.clone(), vec![acc, item.clone()], None, pos)?; }
                        Ok(acc)
                    }
                    v => Err(CustomLangError::type_err(format!("reduce() requires array, got {}", v.type_name()))),
                }
            }
            "for_each" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => { let callback = args[1].clone(); let items = arr.borrow().clone(); for item in items { self.call_value(callback.clone(), vec![item], None, pos)?; } Ok(Value::Null) }
                    v => Err(CustomLangError::type_err(format!("for_each() requires array, got {}", v.type_name()))),
                }
            }
            "every" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => { let callback = args[1].clone(); let items = arr.borrow().clone(); for item in items { if !self.call_value(callback.clone(), vec![item], None, pos)?.is_truthy() { return Ok(Value::Bool(false)); } } Ok(Value::Bool(true)) }
                    v => Err(CustomLangError::type_err(format!("every() requires array, got {}", v.type_name()))),
                }
            }
            "some" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => { let callback = args[1].clone(); let items = arr.borrow().clone(); for item in items { if self.call_value(callback.clone(), vec![item], None, pos)?.is_truthy() { return Ok(Value::Bool(true)); } } Ok(Value::Bool(false)) }
                    v => Err(CustomLangError::type_err(format!("some() requires array, got {}", v.type_name()))),
                }
            }
            "range" => match argc {
                1 => match args[0] { Value::Number(n) => Ok(Value::make_array((0..(n as i64)).map(|i| Value::Number(i as f64)).collect())), _ => Err(CustomLangError::type_err("range() argument must be number")) },
                2 => match (&args[0], &args[1]) { (Value::Number(s), Value::Number(e)) => Ok(Value::make_array((*s as i64..*e as i64).map(|i| Value::Number(i as f64)).collect())), _ => Err(CustomLangError::type_err("range() arguments must be numbers")) },
                3 => match (&args[0], &args[1], &args[2]) {
                    (Value::Number(start), Value::Number(end), Value::Number(step)) => {
                        if *step == 0.0 { return Err(CustomLangError::runtime("range() step cannot be zero")); }
                        let mut r = Vec::new(); let mut i = *start;
                        while if *step > 0.0 { i < *end } else { i > *end } { r.push(Value::Number(i)); i += step; }
                        Ok(Value::make_array(r))
                    }
                    _ => Err(CustomLangError::type_err("range() arguments must be numbers")),
                },
                _ => err_argc("1, 2, or 3"),
            },
            "split" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Str(s), Value::Str(sep)) => Ok(Value::make_array(s.split(sep.as_str()).map(|p| Value::Str(p.to_string())).collect())), _ => Err(CustomLangError::type_err("split() requires (string, string)")) } }
            "join" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Array(arr), Value::Str(sep)) => Ok(Value::Str(arr.borrow().iter().map(|v| v.to_string()).collect::<Vec<_>>().join(sep))), _ => Err(CustomLangError::type_err("join() requires (array, string)")) } }
            "substring" => {
                if !(2..=3).contains(&argc) { return err_argc("2 or 3"); }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Number(start)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let len = chars.len();
                        let s_idx = (*start as usize).min(len);
                        let e_idx = if argc == 3 { match args[2] { Value::Number(n) => (n as usize).min(len), _ => return Err(CustomLangError::type_err("substring() end must be number")) } } else { len };
                        Ok(Value::Str(chars[s_idx..e_idx.max(s_idx)].iter().collect()))
                    }
                    _ => Err(CustomLangError::type_err("substring() requires (string, number[, number])")),
                }
            }
            "to_upper" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(s.to_uppercase())), v => Err(CustomLangError::type_err(format!("to_upper() requires string, got {}", v.type_name()))) } }
            "to_lower" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(s.to_lowercase())), v => Err(CustomLangError::type_err(format!("to_lower() requires string, got {}", v.type_name()))) } }
            "trim" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(s.trim().to_string())), v => Err(CustomLangError::type_err(format!("trim() requires string, got {}", v.type_name()))) } }
            "trim_start" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(s.trim_start().to_string())), _ => Err(CustomLangError::type_err("trim_start() requires string")) } }
            "trim_end" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(s.trim_end().to_string())), _ => Err(CustomLangError::type_err("trim_end() requires string")) } }
            "starts_with" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.starts_with(p.as_str()))), _ => Err(CustomLangError::type_err("starts_with() requires (string, string)")) } }
            "ends_with" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Str(s), Value::Str(p)) => Ok(Value::Bool(s.ends_with(p.as_str()))), _ => Err(CustomLangError::type_err("ends_with() requires (string, string)")) } }
            "contains" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(sub.as_str()))), _ => Err(CustomLangError::type_err("contains() requires (string, string)")) } }
            "replace" => { if argc != 3 { return err_argc("3"); } match (&args[0], &args[1], &args[2]) { (Value::Str(s), Value::Str(from), Value::Str(to)) => Ok(Value::Str(s.replace(from.as_str(), to))), _ => Err(CustomLangError::type_err("replace() requires (string, string, string)")) } }
            "char_at" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Str(s), Value::Number(n)) => Ok(s.chars().nth(*n as usize).map(|c| Value::Str(c.to_string())).unwrap_or(Value::Str(String::new()))), _ => Err(CustomLangError::type_err("char_at() requires (string, number)")) } }
            "char_code" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Number(s.chars().next().map(|c| c as u32 as f64).unwrap_or(0.0))), _ => Err(CustomLangError::type_err("char_code() requires string")) } }
            "format" => {
                if argc < 1 { return err_argc("1+"); }
                match &args[0] {
                    Value::Str(template) => {
                        let mut result = template.clone();
                        for (i, arg) in args[1..].iter().enumerate() {
                            result = result.replace(&format!("{{{i}}}"), &arg.to_string());
                            result = result.replacen("{}", &arg.to_string(), 1);
                        }
                        Ok(Value::Str(result))
                    }
                    _ => Err(CustomLangError::type_err("format() first argument must be string")),
                }
            }
            "repeat" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Str(s), Value::Number(n)) => Ok(Value::Str(s.repeat(*n as usize))), _ => Err(CustomLangError::type_err("repeat() requires (string, number)")) } }
            "pad_start" => {
                if !(2..=3).contains(&argc) { return err_argc("2 or 3"); }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Number(len)) => {
                        let pad_char = if argc == 3 { match &args[2] { Value::Str(p) => p.chars().next().unwrap_or(' '), _ => ' ' } } else { ' ' };
                        let target = *len as usize;
                        let cur_len = s.chars().count();
                        if cur_len >= target { Ok(Value::Str(s.clone())) } else {
                            let pad: String = std::iter::repeat(pad_char).take(target - cur_len).collect();
                            Ok(Value::Str(format!("{pad}{s}")))
                        }
                    }
                    _ => Err(CustomLangError::type_err("pad_start() requires (string, number[, string])")),
                }
            }
            "pad_end" => {
                if !(2..=3).contains(&argc) { return err_argc("2 or 3"); }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Number(len)) => {
                        let pad_char = if argc == 3 { match &args[2] { Value::Str(p) => p.chars().next().unwrap_or(' '), _ => ' ' } } else { ' ' };
                        let target = *len as usize;
                        let cur_len = s.chars().count();
                        if cur_len >= target { Ok(Value::Str(s.clone())) } else {
                            let pad: String = std::iter::repeat(pad_char).take(target - cur_len).collect();
                            Ok(Value::Str(format!("{s}{pad}")))
                        }
                    }
                    _ => Err(CustomLangError::type_err("pad_end() requires (string, number[, string])")),
                }
            }
            "keys" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(obj) => Ok(Value::make_array(obj.borrow().keys().map(|k| Value::Str(k.clone())).collect())), _ => Err(CustomLangError::type_err("keys() requires object")) } }
            "values" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(obj) => Ok(Value::make_array(obj.borrow().values().cloned().collect())), _ => Err(CustomLangError::type_err("values() requires object")) } }
            "entries" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(obj) => Ok(Value::make_array(obj.borrow().iter().map(|(k, v)| Value::make_array(vec![Value::Str(k.clone()), v.clone()])).collect())), _ => Err(CustomLangError::type_err("entries() requires object")) } }
            "has_key" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Object(obj), Value::Str(key)) => Ok(Value::Bool(obj.borrow().contains_key(key.as_str()))), _ => Err(CustomLangError::type_err("has_key() requires (object, string)")) } }
            "delete_key" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Object(obj), Value::Str(key)) => { obj.borrow_mut().remove(key.as_str()); Ok(Value::Null) }, _ => Err(CustomLangError::type_err("delete_key() requires (object, string)")) } }
            "read_file" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(path) => std::fs::read_to_string(path).map(Value::Str).map_err(|e| CustomLangError::io_err(format!("read_file('{path}'): {e}"))), _ => Err(CustomLangError::type_err("read_file() requires string path")) } }
            "write_file" => { if argc != 2 { return err_argc("2"); } match (&args[0], &args[1]) { (Value::Str(path), content) => { std::fs::write(path, content.to_string()).map_err(|e| CustomLangError::io_err(format!("write_file('{path}'): {e}")))?; Ok(Value::Bool(true)) }, _ => Err(CustomLangError::type_err("write_file() requires (string, value)")) } }
            "append_file" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Str(path), content) => {
                        use std::io::Write as IoWrite;
                        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)
                            .map_err(|e| CustomLangError::io_err(format!("append_file('{path}'): {e}")))?;
                        write!(file, "{}", content).map_err(|e| CustomLangError::io_err(format!("append_file('{path}'): {e}")))?;
                        Ok(Value::Bool(true))
                    }
                    _ => Err(CustomLangError::type_err("append_file() requires (string, value)")),
                }
            }
            "assert" => {
                if !(1..=2).contains(&argc) { return err_argc("1 or 2"); }
                if !args[0].is_truthy() {
                    let msg = if argc == 2 { args[1].to_string() } else { "assertion failed".to_string() };
                    return Err(CustomLangError::runtime(msg));
                }
                Ok(Value::Null)
            }
            "exit" => {
                let code = if argc == 1 { match args[0] { Value::Number(n) => n as i32, _ => 0 } } else { 0 };
                std::process::exit(code);
            }
            "now" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0);
                Ok(Value::Number(ms))
            }
            "flat" => {
                if argc != 1 { return err_argc("1"); }
                fn flatten(v: &Value, depth: usize) -> Vec<Value> {
                    match v {
                        Value::Array(arr) if depth > 0 => arr.borrow().iter().flat_map(|x| flatten(x, depth - 1)).collect(),
                        _ => vec![v.clone()],
                    }
                }
                match &args[0] {
                    Value::Array(_) => Ok(Value::make_array(flatten(&args[0], 1))),
                    v => Err(CustomLangError::type_err(format!("flat() requires array, got {}", v.type_name()))),
                }
            }
            "flat_map" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let cb = args[1].clone();
                        let items = arr.borrow().clone();
                        let mut result = Vec::new();
                        for item in items {
                            let mapped = self.call_value(cb.clone(), vec![item], None, pos)?;
                            match mapped { Value::Array(a) => result.extend(a.borrow().clone()), v => result.push(v) }
                        }
                        Ok(Value::make_array(result))
                    }
                    v => Err(CustomLangError::type_err(format!("flat_map() requires array, got {}", v.type_name()))),
                }
            }
            "zip" => {
                if argc < 2 { return err_argc("2+"); }
                let arrays: Vec<Vec<Value>> = args.iter().map(|a| match a { Value::Array(arr) => Ok(arr.borrow().clone()), _ => Err(CustomLangError::type_err("zip() requires arrays")) }).collect::<Result<_>>()?;
                let len = arrays.iter().map(|a| a.len()).min().unwrap_or(0);
                let result: Vec<Value> = (0..len).map(|i| Value::make_array(arrays.iter().map(|a| a[i].clone()).collect())).collect();
                Ok(Value::make_array(result))
            }
            "chunk" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let size = *n as usize;
                        if size == 0 { return Err(CustomLangError::runtime("chunk() size must be > 0")); }
                        let items = arr.borrow().clone();
                        let result: Vec<Value> = items.chunks(size).map(|c| Value::make_array(c.to_vec())).collect();
                        Ok(Value::make_array(result))
                    }
                    _ => Err(CustomLangError::type_err("chunk() requires (array, number)")),
                }
            }
            "unique" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let items = arr.borrow().clone();
                        let mut seen = Vec::new();
                        let mut result = Vec::new();
                        for item in items { if !seen.iter().any(|s: &Value| s.equals(&item)) { seen.push(item.clone()); result.push(item); } }
                        Ok(Value::make_array(result))
                    }
                    v => Err(CustomLangError::type_err(format!("unique() requires array, got {}", v.type_name()))),
                }
            }
            "count" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let cb = args[1].clone();
                        let items = arr.borrow().clone();
                        let mut n = 0;
                        for item in items { if self.call_value(cb.clone(), vec![item], None, pos)?.is_truthy() { n += 1; } }
                        Ok(Value::Number(n as f64))
                    }
                    v => Err(CustomLangError::type_err(format!("count() requires array, got {}", v.type_name()))),
                }
            }
            "sum" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let total = arr.borrow().iter().map(|v| match v { Value::Number(n) => *n, _ => 0.0 }).sum::<f64>();
                        Ok(Value::Number(total))
                    }
                    _ => Err(CustomLangError::type_err("sum() requires array")),
                }
            }
            "average" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let items = arr.borrow();
                        if items.is_empty() { return Ok(Value::Number(0.0)); }
                        let total: f64 = items.iter().map(|v| match v { Value::Number(n) => *n, _ => 0.0 }).sum();
                        Ok(Value::Number(total / items.len() as f64))
                    }
                    _ => Err(CustomLangError::type_err("average() requires array")),
                }
            }
            // ── JSON ──────────────────────────────────────────────────────────
            "json_parse" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Str(s) => parse_json(s).map_err(|e| CustomLangError::runtime(format!("json_parse(): {e}"))),
                    _ => Err(CustomLangError::type_err("json_parse() requires string")),
                }
            }
            "json_stringify" => {
                if !(1..=2).contains(&argc) { return err_argc("1 or 2"); }
                let indent = if argc == 2 { match &args[1] { Value::Number(n) => Some(*n as usize), _ => None } } else { None };
                Ok(Value::Str(stringify_json(&args[0], indent, 0)))
            }
            "json_is_valid" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] { Value::Str(s) => Ok(Value::Bool(parse_json(s).is_ok())), _ => Ok(Value::Bool(false)) }
            }
            // ── Random ────────────────────────────────────────────────────────
            "random_float" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(12345);
                let r = lcg_rand(seed) as f64 / u64::MAX as f64;
                if argc == 2 {
                    if let (Value::Number(min), Value::Number(max)) = (&args[0], &args[1]) {
                        return Ok(Value::Number(min + r * (max - min)));
                    }
                }
                Ok(Value::Number(r))
            }
            "random_int" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Number(min), Value::Number(max)) => {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(42);
                        let range = (*max as i64 - *min as i64 + 1).max(1) as u64;
                        let r = (lcg_rand(seed) % range) as i64 + *min as i64;
                        Ok(Value::Number(r as f64))
                    }
                    _ => Err(CustomLangError::type_err("random_int() requires (number, number)")),
                }
            }
            "random_bool" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(42);
                Ok(Value::Bool(lcg_rand(seed) % 2 == 0))
            }
            "random_choice" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let items = arr.borrow();
                        if items.is_empty() { return Ok(Value::Null); }
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(42);
                        let idx = (lcg_rand(seed) % items.len() as u64) as usize;
                        Ok(items[idx].clone())
                    }
                    _ => Err(CustomLangError::type_err("random_choice() requires array")),
                }
            }
            "random_shuffle" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut items = arr.borrow().clone();
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let mut seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(42) as u64;
                        for i in (1..items.len()).rev() {
                            seed = lcg_rand(seed as u32);
                            let j = (seed % (i + 1) as u64) as usize;
                            items.swap(i, j);
                        }
                        Ok(Value::make_array(items))
                    }
                    _ => Err(CustomLangError::type_err("random_shuffle() requires array")),
                }
            }
            // ── Math extended ─────────────────────────────────────────────────
            "math_clamp" => {
                if argc != 3 { return err_argc("3"); }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Number(v), Value::Number(min), Value::Number(max)) => Ok(Value::Number(v.clamp(*min, *max))),
                    _ => Err(CustomLangError::type_err("clamp() requires (number, number, number)")),
                }
            }
            "math_sign" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] { Value::Number(n) => Ok(Value::Number(if *n > 0.0 { 1.0 } else if *n < 0.0 { -1.0 } else { 0.0 })), _ => Err(CustomLangError::type_err("sign() requires number")) }
            }
            "math_hypot" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) { (Value::Number(x), Value::Number(y)) => Ok(Value::Number(x.hypot(*y))), _ => Err(CustomLangError::type_err("hypot() requires (number, number)")) }
            }
            "math_gcd" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Number(a), Value::Number(b)) => {
                        let (mut a, mut b) = (*a as u64, *b as u64);
                        while b != 0 { let t = b; b = a % b; a = t; }
                        Ok(Value::Number(a as f64))
                    }
                    _ => Err(CustomLangError::type_err("gcd() requires (number, number)")),
                }
            }
            "math_lcm" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Number(a), Value::Number(b)) => {
                        let (mut ga, mut gb) = (*a as u64, *b as u64);
                        let orig_a = ga; let orig_b = gb;
                        while gb != 0 { let t = gb; gb = ga % gb; ga = t; }
                        Ok(Value::Number((orig_a / ga * orig_b) as f64))
                    }
                    _ => Err(CustomLangError::type_err("lcm() requires (number, number)")),
                }
            }
            "math_factorial" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Number(n) => {
                        if *n < 0.0 { return Err(CustomLangError::runtime("factorial() requires non-negative number")); }
                        let mut result = 1u64;
                        for i in 1..=(*n as u64) { result = result.saturating_mul(i); }
                        Ok(Value::Number(result as f64))
                    }
                    _ => Err(CustomLangError::type_err("factorial() requires number")),
                }
            }
            "math_is_nan" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Number(n) => Ok(Value::Bool(n.is_nan())), _ => Ok(Value::Bool(false)) } }
            "math_is_finite" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Number(n) => Ok(Value::Bool(n.is_finite())), _ => Ok(Value::Bool(false)) } }

            _ => Err(CustomLangError::runtime(format!("unknown builtin function '{name}'")).with_pos(pos)),
        }
    }

    fn extract_numbers(args: &[Value], fn_name: &str, pos: &Position) -> Result<Vec<f64>> {
        args.iter().map(|v| match v {
            Value::Number(n) => Ok(*n),
            _ => Err(CustomLangError::type_err(format!("{fn_name}() requires numbers")).with_pos(pos)),
        }).collect()
    }
}

impl Default for Interpreter {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────── Extension trait ─────────────────────────────

trait ErrorExt {
    fn with_pos(self, pos: &Position) -> Self;
}

impl ErrorExt for CustomLangError {
    fn with_pos(self, _pos: &Position) -> Self { self }
}

// ─────────────────────────────── JSON codec ──────────────────────────────────

fn parse_json(s: &str) -> std::result::Result<Value, String> {
    let s = s.trim();
    if s.is_empty() { return Err("empty input".to_string()); }
    match s.chars().next().unwrap() {
        '"' => {
            let inner = &s[1..s.len()-1];
            Ok(Value::Str(inner.replace("\\n", "\n").replace("\\t", "\t").replace("\\\"", "\"")))
        }
        '[' => {
            if s == "[]" { return Ok(Value::make_array(vec![])); }
            let inner = &s[1..s.len()-1].trim();
            let parts = split_json_array(inner);
            let vals: std::result::Result<Vec<Value>, _> = parts.iter().map(|p| parse_json(p.trim())).collect();
            Ok(Value::make_array(vals?))
        }
        '{' => {
            if s == "{}" { return Ok(Value::make_object(HashMap::new())); }
            let inner = &s[1..s.len()-1].trim();
            let parts = split_json_array(inner);
            let mut map = HashMap::new();
            for part in parts {
                let part = part.trim();
                if let Some(colon) = find_json_colon(part) {
                    let key_str = part[..colon].trim();
                    let key = if key_str.starts_with('"') { key_str[1..key_str.len()-1].to_string() } else { key_str.to_string() };
                    let val = parse_json(part[colon+1..].trim())?;
                    map.insert(key, val);
                }
            }
            Ok(Value::make_object(map))
        }
        't' if s == "true" => Ok(Value::Bool(true)),
        'f' if s == "false" => Ok(Value::Bool(false)),
        'n' if s == "null" => Ok(Value::Null),
        _ => s.trim().parse::<f64>().map(Value::Number).map_err(|_| format!("invalid JSON: {s}")),
    }
}

fn split_json_array(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut cur = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' && (i == 0 || chars[i-1] != '\\') { in_str = !in_str; }
        if !in_str {
            match c {
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                ',' if depth == 0 => { parts.push(cur.trim().to_string()); cur.clear(); i += 1; continue; }
                _ => {}
            }
        }
        cur.push(c);
        i += 1;
    }
    if !cur.trim().is_empty() { parts.push(cur.trim().to_string()); }
    parts
}

fn find_json_colon(s: &str) -> Option<usize> {
    let mut in_str = false;
    for (i, c) in s.char_indices() {
        if c == '"' { in_str = !in_str; }
        if c == ':' && !in_str { return Some(i); }
    }
    None
}

fn stringify_json(v: &Value, indent: Option<usize>, depth: usize) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", *n as i64) } else { format!("{n}") }
        }
        Value::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t")),
        Value::Array(arr) => {
            let items = arr.borrow();
            if items.is_empty() { return "[]".to_string(); }
            if let Some(ind) = indent {
                let inner_indent = " ".repeat(ind * (depth + 1));
                let outer_indent = " ".repeat(ind * depth);
                let parts: Vec<String> = items.iter().map(|x| format!("{}{}", inner_indent, stringify_json(x, indent, depth + 1))).collect();
                format!("[\n{}\n{}]", parts.join(",\n"), outer_indent)
            } else {
                let parts: Vec<String> = items.iter().map(|x| stringify_json(x, None, 0)).collect();
                format!("[{}]", parts.join(","))
            }
        }
        Value::Object(obj) => {
            let obj = obj.borrow();
            if obj.is_empty() { return "{}".to_string(); }
            let mut pairs: Vec<(&String, &Value)> = obj.iter().collect();
            pairs.sort_by_key(|(k, _)| k.as_str());
            if let Some(ind) = indent {
                let inner_indent = " ".repeat(ind * (depth + 1));
                let outer_indent = " ".repeat(ind * depth);
                let parts: Vec<String> = pairs.iter().map(|(k, val)| format!("{}\"{}\":{}", inner_indent, k, stringify_json(val, indent, depth + 1))).collect();
                format!("{{\n{}\n{}}}", parts.join(",\n"), outer_indent)
            } else {
                let parts: Vec<String> = pairs.iter().map(|(k, val)| format!("\"{}\":{}", k, stringify_json(val, None, 0))).collect();
                format!("{{{}}}", parts.join(","))
            }
        }
        _ => "null".to_string(),
    }
}

// ─────────────────────────────── LCG random ──────────────────────────────────

fn lcg_rand(seed: u32) -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut s = STATE.load(Ordering::Relaxed);
    if s == 0 { s = seed as u64 ^ 6364136223846793005; }
    s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    STATE.store(s, Ordering::Relaxed);
    s
}
