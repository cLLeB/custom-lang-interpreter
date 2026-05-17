use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;

use crate::ast::*;
use crate::env::{Env, EnvRef};
use crate::error::{CustomLangError, Result};

const MAX_CALL_DEPTH: usize = 500;

/// Control-flow signals propagated through the execution stack
#[derive(Debug, Clone)]
pub enum Signal {
    None,
    /// Value produced by an expression statement (used by REPL to show results)
    ExprValue(Value),
    Return(Value),
    Break,
    Continue,
}

pub struct Interpreter {
    pub env: EnvRef,
    call_depth: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        let env = Env::root();
        Self::register_builtins(&env);
        Self { env, call_depth: 0 }
    }

    // ─── public entry point ───────────────────────────────────────────────

    pub fn interpret(&mut self, program: &Program) -> Result<()> {
        for stmt in &program.stmts {
            match self.exec_stmt(stmt)? {
                Signal::Return(_) => {
                    return Err(CustomLangError::runtime("cannot use 'return' at top level"));
                }
                Signal::Break | Signal::Continue => {
                    return Err(CustomLangError::runtime("'break'/'continue' outside loop"));
                }
                Signal::None | Signal::ExprValue(_) => {}
            }
        }
        Ok(())
    }

    /// Execute a single statement and return a value (used by REPL)
    pub fn exec_repl(&mut self, program: &Program) -> Result<Option<Value>> {
        let mut last = None;
        for stmt in &program.stmts {
            match self.exec_stmt(stmt)? {
                Signal::ExprValue(v) => {
                    last = Some(v);
                }
                Signal::None => {
                    // non-expression statement — no value to show
                }
                Signal::Return(v) => {
                    last = Some(v);
                }
                Signal::Break | Signal::Continue => {}
            }
        }
        Ok(last)
    }

    // ─── statements ───────────────────────────────────────────────────────

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
            Stmt::If {
                cond,
                then_b,
                else_b,
                ..
            } => {
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
                    let c = self.eval_expr(cond)?;
                    if !c.is_truthy() {
                        break;
                    }
                    match self.exec_stmt(body)? {
                        Signal::Return(v) => return Ok(Signal::Return(v)),
                        Signal::Break => break,
                        Signal::Continue => continue,
                        Signal::None | Signal::ExprValue(_) => {}
                    }
                }
                Ok(Signal::None)
            }
            Stmt::For {
                init,
                cond,
                update,
                body,
                ..
            } => {
                // Create a new scope for the entire for loop (so `let i` is scoped)
                let loop_env = Env::child(&self.env);
                let outer = std::mem::replace(&mut self.env, loop_env);

                if let Some(i) = init {
                    self.exec_stmt(i)?;
                }
                let result = loop {
                    if let Some(c) = cond {
                        if !self.eval_expr(c)?.is_truthy() {
                            break Ok(Signal::None);
                        }
                    }
                    match self.exec_stmt(body) {
                        Err(e) => break Err(e),
                        Ok(Signal::Return(v)) => break Ok(Signal::Return(v)),
                        Ok(Signal::Break) => break Ok(Signal::None),
                        Ok(Signal::Continue) | Ok(Signal::None) | Ok(Signal::ExprValue(_)) => {}
                    }
                    if let Some(u) = update {
                        if let Err(e) = self.eval_expr(u) {
                            break Err(e);
                        }
                    }
                };
                self.env = outer;
                result
            }
            Stmt::ForIn {
                var, iter, body, ..
            } => {
                let iter_val = self.eval_expr(iter)?;
                let items: Vec<Value> = match &iter_val {
                    Value::Array(arr) => arr.borrow().clone(),
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    Value::Object(obj) => {
                        obj.borrow().keys().map(|k| Value::Str(k.clone())).collect()
                    }
                    _ => {
                        return Err(CustomLangError::type_err(format!(
                            "cannot iterate over {}",
                            iter_val.type_name()
                        )))
                    }
                };

                let loop_env = Env::child(&self.env);
                let outer = std::mem::replace(&mut self.env, loop_env);
                let mut result = Ok(Signal::None);

                for item in items {
                    Env::define(&self.env, var, item);
                    match self.exec_stmt(body) {
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                        Ok(Signal::Return(v)) => {
                            result = Ok(Signal::Return(v));
                            break;
                        }
                        Ok(Signal::Break) => break,
                        Ok(Signal::Continue) | Ok(Signal::None) | Ok(Signal::ExprValue(_)) => {}
                    }
                }

                self.env = outer;
                result
            }
            Stmt::Function {
                name, params, body, ..
            } => {
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
                let v = match value {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Null,
                };
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
                    return Err(CustomLangError::runtime(format!(
                        "cannot export undefined name '{name}'"
                    )));
                }
                Ok(Signal::None)
            }
            Stmt::Class {
                name,
                super_name,
                methods,
                ..
            } => {
                self.exec_class(name, super_name.as_deref(), methods)?;
                Ok(Signal::None)
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
                s => {
                    signal = s;
                    break;
                }
            }
        }
        self.env = outer;
        Ok(signal)
    }

    // ─── expressions ──────────────────────────────────────────────────────

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Literal { value, .. } => Ok(value.clone()),

            Expr::Var { name, pos } => Env::get(&self.env, name).ok_or_else(|| {
                let names = Env::all_names(&self.env);
                let hint = CustomLangError::find_similar(name, &names)
                    .map(|s| format!("did you mean '{s}'?"));
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

            Expr::CompoundAssign {
                name,
                op,
                value,
                pos,
            } => {
                let current = Env::get(&self.env, name)
                    .ok_or_else(|| CustomLangError::undef_var(name, None).with_pos(pos))?;
                let rhs = self.eval_expr(value)?;
                let new_val = self.apply_binop(&current, &op.to_binary(), &rhs, pos)?;
                if !Env::set(&self.env, name, new_val.clone()) {
                    return Err(CustomLangError::undef_var(name, None).with_pos(pos));
                }
                Ok(new_val)
            }

            Expr::IndexAssign {
                object,
                index,
                value,
                pos,
            } => {
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
                            Err(CustomLangError::runtime(format!(
                                "array index {idx} out of bounds (length {})",
                                arr.len()
                            ))
                            .with_pos(pos))
                        }
                    }
                    (Value::Object(obj), Value::Str(key)) => {
                        obj.borrow_mut().insert(key.clone(), new_val.clone());
                        Ok(new_val)
                    }
                    _ => Err(CustomLangError::type_err(format!(
                        "cannot index-assign {} with {}",
                        obj_val.type_name(),
                        idx_val.type_name()
                    ))
                    .with_pos(pos)),
                }
            }

            Expr::PropAssign {
                object,
                prop,
                value,
                pos,
            } => {
                let obj_val = self.eval_expr(object)?;
                let new_val = self.eval_expr(value)?;
                match &obj_val {
                    Value::Object(obj) => {
                        obj.borrow_mut().insert(prop.clone(), new_val.clone());
                        Ok(new_val)
                    }
                    Value::Instance(inst) => {
                        inst.borrow_mut()
                            .fields
                            .insert(prop.clone(), new_val.clone());
                        Ok(new_val)
                    }
                    _ => Err(CustomLangError::type_err(format!(
                        "cannot set property '{}' on {}",
                        prop,
                        obj_val.type_name()
                    ))
                    .with_pos(pos)),
                }
            }

            Expr::Binary {
                left,
                op,
                right,
                pos,
            } => {
                // Short-circuit evaluation for && and ||
                match op {
                    BinaryOp::And => {
                        let l = self.eval_expr(left)?;
                        if !l.is_truthy() {
                            return Ok(l);
                        }
                        return self.eval_expr(right);
                    }
                    BinaryOp::Or => {
                        let l = self.eval_expr(left)?;
                        if l.is_truthy() {
                            return Ok(l);
                        }
                        return self.eval_expr(right);
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
                        _ => Err(CustomLangError::type_err(format!(
                            "unary '-' requires number, got {}",
                            v.type_name()
                        ))
                        .with_pos(pos)),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!v.is_truthy())),
                }
            }

            Expr::Call { callee, args, pos } => {
                // Special handling: method calls need the receiver for 'this'
                match callee.as_ref() {
                    Expr::Prop { object, name, .. } => {
                        let receiver = self.eval_expr(object)?;
                        let method = self.get_method(&receiver, name, pos)?;
                        let arg_vals = self.eval_args(args)?;
                        self.call_with_this(method, Some(receiver), arg_vals, pos)
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

            Expr::Array { elements, .. } => {
                let vals: Result<Vec<Value>> = elements.iter().map(|e| self.eval_expr(e)).collect();
                Ok(Value::make_array(vals?))
            }

            Expr::Object { pairs, .. } => {
                let mut map = HashMap::new();
                for (k, v) in pairs {
                    map.insert(k.clone(), self.eval_expr(v)?);
                }
                Ok(Value::make_object(map))
            }

            Expr::New { class, args, pos } => {
                let class_val = Env::get(&self.env, class).ok_or_else(|| {
                    let names: Vec<String> = Env::all_names(&self.env);
                    let hint = CustomLangError::find_similar(class, &names);
                    CustomLangError::undef_var(class.clone(), hint).with_pos(pos)
                })?;
                let arg_vals = self.eval_args(args)?;
                self.instantiate(class_val, arg_vals, pos)
            }

            Expr::This { pos } => Env::get(&self.env, "this").ok_or_else(|| {
                CustomLangError::runtime("'this' used outside of a class method").with_pos(pos)
            }),

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

    // ─── operators ────────────────────────────────────────────────────────

    fn apply_binop(&self, l: &Value, op: &BinaryOp, r: &Value, pos: &Position) -> Result<Value> {
        match op {
            BinaryOp::Add => self.op_add(l, r, pos),
            BinaryOp::Subtract => self.numeric_op(l, r, op, pos, |a, b| a - b),
            BinaryOp::Multiply => self.numeric_op(l, r, op, pos, |a, b| a * b),
            BinaryOp::Divide => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    if *b == 0.0 {
                        return Err(CustomLangError::DivisionByZero.with_pos(pos));
                    }
                    Ok(Value::Number(a / b))
                } else {
                    Err(self.type_err_binop("division", l, r, pos))
                }
            }
            BinaryOp::Modulo => {
                if let (Value::Number(a), Value::Number(b)) = (l, r) {
                    if *b == 0.0 {
                        return Err(CustomLangError::DivisionByZero.with_pos(pos));
                    }
                    Ok(Value::Number(a % b))
                } else {
                    Err(self.type_err_binop("modulo", l, r, pos))
                }
            }
            BinaryOp::Equal => Ok(Value::Bool(l.equals(r))),
            BinaryOp::NotEqual => Ok(Value::Bool(!l.equals(r))),
            BinaryOp::Less => self.compare_op(l, r, op, pos, |o| o.is_lt()),
            BinaryOp::LessEqual => self.compare_op(l, r, op, pos, |o| o.is_le()),
            BinaryOp::Greater => self.compare_op(l, r, op, pos, |o| o.is_gt()),
            BinaryOp::GreaterEqual => self.compare_op(l, r, op, pos, |o| o.is_ge()),
            // Short-circuit handled above; shouldn't reach here
            BinaryOp::And => Ok(if l.is_truthy() { r.clone() } else { l.clone() }),
            BinaryOp::Or => Ok(if l.is_truthy() { l.clone() } else { r.clone() }),
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
            // String coercion — any type can be added to a string
            (Value::Str(a), other) | (other, Value::Str(a)) if matches!(l, Value::Str(_)) => {
                Ok(Value::Str(format!("{a}{other}")))
            }
            (other, Value::Str(b)) => Ok(Value::Str(format!("{other}{b}"))),
            _ => Err(self.type_err_binop("addition", l, r, pos)),
        }
    }

    fn numeric_op<F>(
        &self,
        l: &Value,
        r: &Value,
        op: &BinaryOp,
        pos: &Position,
        f: F,
    ) -> Result<Value>
    where
        F: Fn(f64, f64) -> f64,
    {
        match (l, r) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(f(*a, *b))),
            _ => Err(self.type_err_binop(&format!("{op:?}"), l, r, pos)),
        }
    }

    fn compare_op<F>(
        &self,
        l: &Value,
        r: &Value,
        op: &BinaryOp,
        pos: &Position,
        f: F,
    ) -> Result<Value>
    where
        F: Fn(std::cmp::Ordering) -> bool,
    {
        let ord = match (l, r) {
            (Value::Number(a), Value::Number(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Value::Str(a), Value::Str(b)) => a.cmp(b),
            _ => return Err(self.type_err_binop(&format!("{op:?}"), l, r, pos)),
        };
        Ok(Value::Bool(f(ord)))
    }

    fn type_err_binop(&self, op: &str, l: &Value, r: &Value, pos: &Position) -> CustomLangError {
        CustomLangError::type_err(format!(
            "cannot apply {op} to {} and {}",
            l.type_name(),
            r.type_name()
        ))
        .with_pos(pos)
    }

    // ─── property / index access ──────────────────────────────────────────

    fn eval_index(&self, obj: &Value, idx: &Value, pos: &Position) -> Result<Value> {
        match (obj, idx) {
            (Value::Array(arr), Value::Number(n)) => {
                let i = *n as usize;
                let arr = arr.borrow();
                arr.get(i).cloned().ok_or_else(|| {
                    CustomLangError::runtime(format!(
                        "array index {i} out of bounds (length {})",
                        arr.len()
                    ))
                    .with_pos(pos)
                })
            }
            (Value::Str(s), Value::Number(n)) => {
                let i = *n as usize;
                s.chars()
                    .nth(i)
                    .map(|c| Value::Str(c.to_string()))
                    .ok_or_else(|| {
                        CustomLangError::runtime(format!(
                            "string index {i} out of bounds (length {})",
                            s.chars().count()
                        ))
                        .with_pos(pos)
                    })
            }
            (Value::Object(obj), Value::Str(key)) => {
                Ok(obj.borrow().get(key).cloned().unwrap_or(Value::Null))
            }
            _ => Err(CustomLangError::type_err(format!(
                "cannot index {} with {}",
                obj.type_name(),
                idx.type_name()
            ))
            .with_pos(pos)),
        }
    }

    fn get_property(&self, obj: &Value, name: &str, pos: &Position) -> Result<Value> {
        match obj {
            Value::Instance(inst) => {
                let inst_b = inst.borrow();
                // Check fields first
                if let Some(v) = inst_b.fields.get(name) {
                    return Ok(v.clone());
                }
                // Then methods
                if let Some(m) = inst_b.class.methods.get(name) {
                    return Ok(Value::Function(Rc::clone(m)));
                }
                // Superclass methods
                let mut super_cls = inst_b.class.superclass.clone();
                while let Some(sc) = super_cls {
                    if let Some(m) = sc.methods.get(name) {
                        return Ok(Value::Function(Rc::clone(m)));
                    }
                    super_cls = sc.superclass.clone();
                }
                Ok(Value::Null)
            }
            Value::Object(obj) => Ok(obj.borrow().get(name).cloned().unwrap_or(Value::Null)),
            Value::Array(arr) => {
                // Array length property
                if name == "length" {
                    return Ok(Value::Number(arr.borrow().len() as f64));
                }
                Ok(Value::Null)
            }
            Value::Str(s) => {
                if name == "length" {
                    return Ok(Value::Number(s.chars().count() as f64));
                }
                Ok(Value::Null)
            }
            _ => Err(CustomLangError::type_err(format!(
                "cannot access property '{}' on {}",
                name,
                obj.type_name()
            ))
            .with_pos(pos)),
        }
    }

    fn get_method(&self, obj: &Value, name: &str, pos: &Position) -> Result<Value> {
        match obj {
            Value::Instance(inst) => {
                let inst_b = inst.borrow();
                if let Some(m) = inst_b.class.methods.get(name) {
                    return Ok(Value::Function(Rc::clone(m)));
                }
                let mut super_cls = inst_b.class.superclass.clone();
                while let Some(sc) = super_cls {
                    if let Some(m) = sc.methods.get(name) {
                        return Ok(Value::Function(Rc::clone(m)));
                    }
                    super_cls = sc.superclass.clone();
                }
                // Check fields too (function stored in field)
                if let Some(v) = inst_b.fields.get(name) {
                    return Ok(v.clone());
                }
                Err(CustomLangError::runtime(format!(
                    "instance of '{}' has no method '{name}'",
                    inst_b.class.name
                ))
                .with_pos(pos))
            }
            Value::Object(obj) => obj.borrow().get(name).cloned().ok_or_else(|| {
                CustomLangError::runtime(format!("object has no property '{name}'")).with_pos(pos)
            }),
            _ => Err(CustomLangError::type_err(format!(
                "cannot call method '{name}' on {}",
                obj.type_name()
            ))
            .with_pos(pos)),
        }
    }

    // ─── function calls ───────────────────────────────────────────────────

    fn eval_args(&mut self, args: &[Expr]) -> Result<Vec<Value>> {
        args.iter().map(|a| self.eval_expr(a)).collect()
    }

    pub fn call_value(
        &mut self,
        func: Value,
        args: Vec<Value>,
        this: Option<Value>,
        pos: &Position,
    ) -> Result<Value> {
        match func {
            Value::Function(fd) => self.call_fn(&fd, args, this, pos),
            Value::Builtin(name) => self.call_builtin(&name, args, pos),
            Value::Class(cls) => {
                // Calling a class directly creates an instance
                self.instantiate(Value::Class(cls), args, pos)
            }
            _ => Err(CustomLangError::type_err(format!(
                "cannot call value of type {}",
                func.type_name()
            ))
            .with_pos(pos)),
        }
    }

    fn call_with_this(
        &mut self,
        func: Value,
        this: Option<Value>,
        args: Vec<Value>,
        pos: &Position,
    ) -> Result<Value> {
        self.call_value(func, args, this, pos)
    }

    fn call_fn(
        &mut self,
        fd: &Rc<FnData>,
        args: Vec<Value>,
        this: Option<Value>,
        pos: &Position,
    ) -> Result<Value> {
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(CustomLangError::StackOverflow.with_pos(pos));
        }
        if args.len() != fd.params.len() {
            return Err(CustomLangError::runtime(format!(
                "function '{}' expects {} arguments, got {}",
                fd.name,
                fd.params.len(),
                args.len()
            ))
            .with_pos(pos));
        }

        let fn_env = Env::child(&fd.closure);
        // Bind 'this' if provided
        if let Some(t) = this {
            Env::define(&fn_env, "this", t);
        }
        // Bind the function itself for recursion
        Env::define(&fn_env, &fd.name, Value::Function(Rc::clone(fd)));
        // Bind parameters
        for (param, val) in fd.params.iter().zip(args) {
            Env::define(&fn_env, param, val);
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

    // ─── class / instance ─────────────────────────────────────────────────

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
        for method in methods {
            if let Stmt::Function {
                name: mn,
                params,
                body,
                ..
            } = method
            {
                let fd = Rc::new(FnData {
                    name: mn.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.env),
                });
                method_map.insert(mn.clone(), fd);
            }
        }

        let cls = Rc::new(ClassData {
            name: name.to_string(),
            methods: method_map,
            superclass,
        });
        Env::define(&self.env, name, Value::Class(cls));
        Ok(())
    }

    fn instantiate(&mut self, class_val: Value, args: Vec<Value>, pos: &Position) -> Result<Value> {
        let cls = match class_val {
            Value::Class(c) => c,
            _ => {
                return Err(CustomLangError::type_err(format!(
                    "cannot instantiate {}",
                    class_val.type_name()
                ))
                .with_pos(pos))
            }
        };

        let inst = Rc::new(RefCell::new(InstanceData {
            class: Rc::clone(&cls),
            fields: HashMap::new(),
        }));
        let inst_val = Value::Instance(Rc::clone(&inst));

        // Call constructor if present
        if let Some(init_fd) = cls.methods.get("init") {
            self.call_fn(init_fd, args, Some(inst_val.clone()), pos)?;
            // After constructor runs, copy fields set via 'this' back
            // (they're stored in the instance via PropAssign which updates inst directly)
        }

        Ok(inst_val)
    }

    // ─── pattern matching ─────────────────────────────────────────────────

    fn match_pattern(
        &self,
        pattern: &Pattern,
        value: &Value,
    ) -> Result<Option<Vec<(String, Value)>>> {
        Ok(match pattern {
            Pattern::Number(n) => {
                if let Value::Number(v) = value {
                    if (v - n).abs() < f64::EPSILON {
                        Some(vec![])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Pattern::Str(s) => {
                if let Value::Str(v) = value {
                    if v == s {
                        Some(vec![])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Pattern::Bool(b) => {
                if let Value::Bool(v) = value {
                    if v == b {
                        Some(vec![])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Pattern::Null => {
                if matches!(value, Value::Null) {
                    Some(vec![])
                } else {
                    None
                }
            }
            Pattern::Wildcard => Some(vec![]),
            Pattern::Binding(name) => Some(vec![(name.clone(), value.clone())]),
            Pattern::Array(pats) => {
                if let Value::Array(arr) = value {
                    let arr = arr.borrow();
                    if pats.len() != arr.len() {
                        return Ok(None);
                    }
                    let mut bindings = Vec::new();
                    for (p, v) in pats.iter().zip(arr.iter()) {
                        match self.match_pattern(p, v)? {
                            Some(mut b) => bindings.append(&mut b),
                            None => return Ok(None),
                        }
                    }
                    Some(bindings)
                } else {
                    None
                }
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
                } else {
                    None
                }
            }
        })
    }

    // ─── import ───────────────────────────────────────────────────────────

    fn exec_import(&mut self, path: &str, alias: Option<&str>) -> Result<()> {
        let file_path = if path.ends_with(".cl") {
            path.to_string()
        } else {
            format!("{path}.cl")
        };

        let source = std::fs::read_to_string(&file_path).map_err(|e| {
            CustomLangError::io_err(format!("cannot read module '{file_path}': {e}"))
        })?;

        let tokens = crate::lexer::Lexer::new(&source)
            .tokenize()
            .map_err(|e| CustomLangError::runtime(format!("error in module '{file_path}': {e}")))?;
        let program = crate::parser::Parser::new(tokens)
            .parse()
            .map_err(|e| CustomLangError::runtime(format!("error in module '{file_path}': {e}")))?;

        // Execute module in its own environment
        let mod_env = Env::root();
        Self::register_builtins(&mod_env);
        let mut mod_interp = Interpreter {
            env: mod_env,
            call_depth: self.call_depth,
        };
        mod_interp.interpret(&program)?;

        // Import names into current environment
        let mod_name = alias.unwrap_or_else(|| {
            std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(path)
        });

        if alias.is_some() {
            // Create a namespace object
            let mut ns = HashMap::new();
            let inner = mod_interp.env.borrow();
            // Can't easily introspect Rc<RefCell<Env>>, so iterate direct vars
            drop(inner);
            // For now, merge all top-level names with prefix
            let names = Env::all_names(&mod_interp.env);
            for name in names {
                if let Some(v) = Env::get(&mod_interp.env, &name) {
                    ns.insert(name, v);
                }
            }
            Env::define(&self.env, mod_name, Value::make_object(ns));
        } else {
            // Flat import
            let names = Env::all_names(&mod_interp.env);
            for name in names {
                if let Some(v) = Env::get(&mod_interp.env, &name) {
                    Env::define(&self.env, &name, v);
                }
            }
        }
        Ok(())
    }

    // ─── builtins ─────────────────────────────────────────────────────────

    fn register_builtins(env: &EnvRef) {
        let builtins = [
            "print",
            "println",
            "input",
            "len",
            "type",
            "abs",
            "sqrt",
            "pow",
            "min",
            "max",
            "floor",
            "ceil",
            "round",
            "log",
            "sin",
            "cos",
            "tan",
            "to_string",
            "to_number",
            "to_bool",
            "push",
            "pop",
            "shift",
            "unshift",
            "first",
            "last",
            "sort",
            "reverse",
            "slice",
            "includes",
            "find",
            "index_of",
            "filter",
            "map",
            "reduce",
            "for_each",
            "every",
            "some",
            "split",
            "join",
            "substring",
            "to_upper",
            "to_lower",
            "trim",
            "trim_start",
            "trim_end",
            "starts_with",
            "ends_with",
            "contains",
            "replace",
            "char_at",
            "char_code",
            "format",
            "read_file",
            "write_file",
            "append_file",
            "keys",
            "values",
            "entries",
            "has_key",
            "delete_key",
            "parse_int",
            "parse_float",
            "is_number",
            "is_string",
            "is_bool",
            "is_null",
            "is_array",
            "is_object",
            "assert",
            "exit",
            "now",
            "range",
        ];
        for name in builtins {
            Env::define(env, name, Value::Builtin(name.to_string()));
        }
    }

    fn call_builtin(&mut self, name: &str, args: Vec<Value>, pos: &Position) -> Result<Value> {
        let argc = args.len();
        let err_argc = |expected: &str| {
            Err(CustomLangError::runtime(format!(
                "{name}() expects {expected} argument(s), got {argc}"
            ))
            .with_pos(pos))
        };

        match name {
            // ── I/O ──────────────────────────────────────────────────────
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
                if !args.is_empty() {
                    print!("{}", args[0]);
                    let _ = io::stdout().flush();
                }
                let mut line = String::new();
                io::stdin()
                    .read_line(&mut line)
                    .map_err(|e| CustomLangError::io_err(format!("failed to read input: {e}")))?;
                Ok(Value::Str(line.trim_end_matches(['\n', '\r']).to_string()))
            }

            // ── Type / conversion ─────────────────────────────────────────
            "type" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Str(args[0].type_name().to_string()))
            }
            "to_string" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Str(args[0].to_string()))
            }
            "to_number" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Number(n) => Ok(Value::Number(*n)),
                    Value::Str(s) => s.trim().parse::<f64>().map(Value::Number).map_err(|_| {
                        CustomLangError::type_err(format!("cannot convert '{s}' to number"))
                    }),
                    Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
                    Value::Null => Ok(Value::Number(0.0)),
                    v => Err(CustomLangError::type_err(format!(
                        "cannot convert {} to number",
                        v.type_name()
                    ))),
                }
            }
            "to_bool" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Bool(args[0].is_truthy()))
            }
            "parse_int" => {
                if !(1..=2).contains(&argc) {
                    return err_argc("1 or 2");
                }
                let s = match &args[0] {
                    Value::Str(s) => s.trim().to_string(),
                    v => v.to_string(),
                };
                let radix = if argc == 2 {
                    match &args[1] {
                        Value::Number(n) => *n as u32,
                        _ => 10,
                    }
                } else {
                    10
                };
                i64::from_str_radix(&s, radix)
                    .map(|n| Value::Number(n as f64))
                    .map_err(|_| CustomLangError::runtime(format!("cannot parse '{s}' as integer")))
            }
            "parse_float" => {
                if argc != 1 {
                    return err_argc("1");
                }
                let s = args[0].to_string();
                s.trim()
                    .parse::<f64>()
                    .map(Value::Number)
                    .map_err(|_| CustomLangError::runtime(format!("cannot parse '{s}' as float")))
            }

            // ── Type predicates ───────────────────────────────────────────
            "is_number" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Bool(matches!(args[0], Value::Number(_))))
            }
            "is_string" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Bool(matches!(args[0], Value::Str(_))))
            }
            "is_bool" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Bool(matches!(args[0], Value::Bool(_))))
            }
            "is_null" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Bool(matches!(args[0], Value::Null)))
            }
            "is_array" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Bool(matches!(args[0], Value::Array(_))))
            }
            "is_object" => {
                if argc != 1 {
                    return err_argc("1");
                }
                Ok(Value::Bool(matches!(args[0], Value::Object(_))))
            }

            // ── General ───────────────────────────────────────────────────
            "len" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Number(s.chars().count() as f64)),
                    Value::Array(a) => Ok(Value::Number(a.borrow().len() as f64)),
                    Value::Object(o) => Ok(Value::Number(o.borrow().len() as f64)),
                    v => Err(CustomLangError::type_err(format!(
                        "len() not supported for {}",
                        v.type_name()
                    ))),
                }
            }

            // ── Math ──────────────────────────────────────────────────────
            "abs" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match args[0] {
                    Value::Number(n) => Ok(Value::Number(n.abs())),
                    ref v => Err(CustomLangError::type_err(format!(
                        "abs() requires number, got {}",
                        v.type_name()
                    ))),
                }
            }
            "sqrt" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match args[0] {
                    Value::Number(n) if n >= 0.0 => Ok(Value::Number(n.sqrt())),
                    Value::Number(_) => Err(CustomLangError::runtime("sqrt() of negative number")),
                    ref v => Err(CustomLangError::type_err(format!(
                        "sqrt() requires number, got {}",
                        v.type_name()
                    ))),
                }
            }
            "pow" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Number(b), Value::Number(e)) => Ok(Value::Number(b.powf(*e))),
                    _ => Err(CustomLangError::type_err("pow() requires two numbers")),
                }
            }
            "min" => {
                if argc < 1 {
                    return err_argc("1+");
                }
                if argc == 2 {
                    if let (Value::Number(a), Value::Number(b)) = (&args[0], &args[1]) {
                        return Ok(Value::Number(a.min(*b)));
                    }
                }
                // Multi-arg or array min
                let nums = Self::extract_numbers(&args, "min", pos)?;
                Ok(Value::Number(
                    nums.into_iter().fold(f64::INFINITY, f64::min),
                ))
            }
            "max" => {
                if argc < 1 {
                    return err_argc("1+");
                }
                if argc == 2 {
                    if let (Value::Number(a), Value::Number(b)) = (&args[0], &args[1]) {
                        return Ok(Value::Number(a.max(*b)));
                    }
                }
                let nums = Self::extract_numbers(&args, "max", pos)?;
                Ok(Value::Number(
                    nums.into_iter().fold(f64::NEG_INFINITY, f64::max),
                ))
            }
            "floor" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match args[0] {
                    Value::Number(n) => Ok(Value::Number(n.floor())),
                    ref v => Err(CustomLangError::type_err(format!(
                        "floor() requires number, got {}",
                        v.type_name()
                    ))),
                }
            }
            "ceil" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match args[0] {
                    Value::Number(n) => Ok(Value::Number(n.ceil())),
                    ref v => Err(CustomLangError::type_err(format!(
                        "ceil() requires number, got {}",
                        v.type_name()
                    ))),
                }
            }
            "round" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match args[0] {
                    Value::Number(n) => Ok(Value::Number(n.round())),
                    ref v => Err(CustomLangError::type_err(format!(
                        "round() requires number, got {}",
                        v.type_name()
                    ))),
                }
            }
            "log" => {
                if !(1..=2).contains(&argc) {
                    return err_argc("1 or 2");
                }
                match args[0] {
                    Value::Number(n) => {
                        let result = if argc == 2 {
                            match args[1] {
                                Value::Number(base) => n.log(base),
                                _ => {
                                    return Err(CustomLangError::type_err(
                                        "log() base must be a number",
                                    ))
                                }
                            }
                        } else {
                            n.ln()
                        };
                        Ok(Value::Number(result))
                    }
                    ref v => Err(CustomLangError::type_err(format!(
                        "log() requires number, got {}",
                        v.type_name()
                    ))),
                }
            }
            "sin" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match args[0] {
                    Value::Number(n) => Ok(Value::Number(n.sin())),
                    _ => Err(CustomLangError::type_err("sin() requires number")),
                }
            }
            "cos" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match args[0] {
                    Value::Number(n) => Ok(Value::Number(n.cos())),
                    _ => Err(CustomLangError::type_err("cos() requires number")),
                }
            }
            "tan" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match args[0] {
                    Value::Number(n) => Ok(Value::Number(n.tan())),
                    _ => Err(CustomLangError::type_err("tan() requires number")),
                }
            }

            // ── Array ─────────────────────────────────────────────────────
            "push" => {
                if argc < 2 {
                    return err_argc("2+");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        for v in &args[1..] {
                            arr.borrow_mut().push(v.clone());
                        }
                        Ok(args[0].clone()) // return the array (already mutated in place)
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "push() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "pop" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Array(arr) => Ok(arr.borrow_mut().pop().unwrap_or(Value::Null)),
                    v => Err(CustomLangError::type_err(format!(
                        "pop() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "shift" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut a = arr.borrow_mut();
                        if a.is_empty() {
                            Ok(Value::Null)
                        } else {
                            Ok(a.remove(0))
                        }
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "shift() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "unshift" => {
                if argc < 2 {
                    return err_argc("2+");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        for (i, v) in args[1..].iter().enumerate() {
                            arr.borrow_mut().insert(i, v.clone());
                        }
                        Ok(args[0].clone()) // return the array
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "unshift() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "first" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Array(arr) => Ok(arr.borrow().first().cloned().unwrap_or(Value::Null)),
                    v => Err(CustomLangError::type_err(format!(
                        "first() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "last" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Array(arr) => Ok(arr.borrow().last().cloned().unwrap_or(Value::Null)),
                    v => Err(CustomLangError::type_err(format!(
                        "last() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "sort" => {
                if !(1..=2).contains(&argc) {
                    return err_argc("1 or 2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut a = arr.borrow_mut();
                        if argc == 2 {
                            // Custom comparator
                            let cmp_fn = args[1].clone();
                            let mut error: Option<CustomLangError> = None;
                            a.sort_by(|x, y| {
                                if error.is_some() {
                                    return std::cmp::Ordering::Equal;
                                }
                                match self.call_value(
                                    cmp_fn.clone(),
                                    vec![x.clone(), y.clone()],
                                    None,
                                    pos,
                                ) {
                                    Ok(Value::Number(n)) => {
                                        if n < 0.0 {
                                            std::cmp::Ordering::Less
                                        } else if n > 0.0 {
                                            std::cmp::Ordering::Greater
                                        } else {
                                            std::cmp::Ordering::Equal
                                        }
                                    }
                                    Ok(_) => std::cmp::Ordering::Equal,
                                    Err(e) => {
                                        error = Some(e);
                                        std::cmp::Ordering::Equal
                                    }
                                }
                            });
                            if let Some(e) = error {
                                return Err(e);
                            }
                        } else {
                            a.sort_by(|x, y| match (x, y) {
                                (Value::Number(a), Value::Number(b)) => {
                                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                                }
                                (Value::Str(a), Value::Str(b)) => a.cmp(b),
                                _ => std::cmp::Ordering::Equal,
                            });
                        }
                        drop(a);
                        Ok(args[0].clone())
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "sort() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "reverse" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        arr.borrow_mut().reverse();
                        Ok(args[0].clone())
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "reverse() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "slice" => {
                if !(2..=3).contains(&argc) {
                    return err_argc("2 or 3");
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::Number(start)) => {
                        let arr = arr.borrow();
                        let len = arr.len();
                        let s = (*start as isize).rem_euclid(len as isize) as usize;
                        let e = if argc == 3 {
                            match args[2] {
                                Value::Number(n) => (n as isize).rem_euclid(len as isize) as usize,
                                _ => {
                                    return Err(CustomLangError::type_err(
                                        "slice() end must be number",
                                    ))
                                }
                            }
                        } else {
                            len
                        };
                        let sliced: Vec<Value> = arr[s.min(len)..e.min(len)].to_vec();
                        Ok(Value::make_array(sliced))
                    }
                    _ => Err(CustomLangError::type_err(
                        "slice() requires (array, number[, number])",
                    )),
                }
            }
            "includes" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let found = arr.borrow().iter().any(|v| v.equals(&args[1]));
                        Ok(Value::Bool(found))
                    }
                    Value::Str(s) => {
                        if let Value::Str(sub) = &args[1] {
                            Ok(Value::Bool(s.contains(sub.as_str())))
                        } else {
                            Ok(Value::Bool(false))
                        }
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "includes() requires array or string, got {}",
                        v.type_name()
                    ))),
                }
            }
            "find" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let needle = args[1].clone();
                        let arr = arr.borrow().clone();
                        match &needle {
                            Value::Function(_) | Value::Builtin(_) => {
                                // predicate mode: find(arr, fn)
                                for item in arr {
                                    let result = self.call_value(needle.clone(), vec![item.clone()], None, pos)?;
                                    if result.is_truthy() { return Ok(item); }
                                }
                            }
                            _ => {
                                // value mode: find(arr, value) → first element equal to value
                                for item in arr {
                                    if item.equals(&needle) { return Ok(item); }
                                }
                            }
                        }
                        Ok(Value::Null)
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "find() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "index_of" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let arr = arr.borrow();
                        let idx = arr.iter().position(|v| v.equals(&args[1]));
                        Ok(Value::Number(idx.map(|i| i as f64).unwrap_or(-1.0)))
                    }
                    Value::Str(s) => {
                        if let Value::Str(sub) = &args[1] {
                            Ok(Value::Number(
                                s.find(sub.as_str()).map(|i| i as f64).unwrap_or(-1.0),
                            ))
                        } else {
                            Ok(Value::Number(-1.0))
                        }
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "index_of() requires array or string, got {}",
                        v.type_name()
                    ))),
                }
            }
            "filter" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        let mut result = Vec::new();
                        for item in items {
                            let pass =
                                self.call_value(callback.clone(), vec![item.clone()], None, pos)?;
                            if pass.is_truthy() {
                                result.push(item);
                            }
                        }
                        Ok(Value::make_array(result))
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "filter() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "map" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        let mut result = Vec::new();
                        for item in items {
                            let mapped =
                                self.call_value(callback.clone(), vec![item], None, pos)?;
                            result.push(mapped);
                        }
                        Ok(Value::make_array(result))
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "map() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "reduce" => {
                if !(2..=3).contains(&argc) {
                    return err_argc("2 or 3");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        let mut acc = if argc == 3 {
                            args[2].clone()
                        } else {
                            items.first().cloned().unwrap_or(Value::Null)
                        };
                        let start = if argc == 3 { 0 } else { 1 };
                        for item in items[start..].iter() {
                            acc = self.call_value(
                                callback.clone(),
                                vec![acc, item.clone()],
                                None,
                                pos,
                            )?;
                        }
                        Ok(acc)
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "reduce() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "for_each" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        for item in items {
                            self.call_value(callback.clone(), vec![item], None, pos)?;
                        }
                        Ok(Value::Null)
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "for_each() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "every" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        for item in items {
                            if !self
                                .call_value(callback.clone(), vec![item], None, pos)?
                                .is_truthy()
                            {
                                return Ok(Value::Bool(false));
                            }
                        }
                        Ok(Value::Bool(true))
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "every() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }
            "some" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let callback = args[1].clone();
                        let items = arr.borrow().clone();
                        for item in items {
                            if self
                                .call_value(callback.clone(), vec![item], None, pos)?
                                .is_truthy()
                            {
                                return Ok(Value::Bool(true));
                            }
                        }
                        Ok(Value::Bool(false))
                    }
                    v => Err(CustomLangError::type_err(format!(
                        "some() requires array, got {}",
                        v.type_name()
                    ))),
                }
            }

            // ── Range ─────────────────────────────────────────────────────
            "range" => match argc {
                1 => match args[0] {
                    Value::Number(n) => {
                        let r: Vec<Value> =
                            (0..(n as i64)).map(|i| Value::Number(i as f64)).collect();
                        Ok(Value::make_array(r))
                    }
                    _ => Err(CustomLangError::type_err("range() argument must be number")),
                },
                2 => match (&args[0], &args[1]) {
                    (Value::Number(start), Value::Number(end)) => {
                        let r: Vec<Value> = (*start as i64..*end as i64)
                            .map(|i| Value::Number(i as f64))
                            .collect();
                        Ok(Value::make_array(r))
                    }
                    _ => Err(CustomLangError::type_err(
                        "range() arguments must be numbers",
                    )),
                },
                3 => match (&args[0], &args[1], &args[2]) {
                    (Value::Number(start), Value::Number(end), Value::Number(step)) => {
                        if *step == 0.0 {
                            return Err(CustomLangError::runtime("range() step cannot be zero"));
                        }
                        let mut r = Vec::new();
                        let mut i = *start;
                        while if *step > 0.0 { i < *end } else { i > *end } {
                            r.push(Value::Number(i));
                            i += step;
                        }
                        Ok(Value::make_array(r))
                    }
                    _ => Err(CustomLangError::type_err(
                        "range() arguments must be numbers",
                    )),
                },
                _ => err_argc("1, 2, or 3"),
            },

            // ── String ────────────────────────────────────────────────────
            "split" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(sep)) => {
                        let parts: Vec<Value> = s
                            .split(sep.as_str())
                            .map(|p| Value::Str(p.to_string()))
                            .collect();
                        Ok(Value::make_array(parts))
                    }
                    _ => Err(CustomLangError::type_err(
                        "split() requires (string, string)",
                    )),
                }
            }
            "join" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::Str(sep)) => {
                        let parts: Vec<String> =
                            arr.borrow().iter().map(|v| v.to_string()).collect();
                        Ok(Value::Str(parts.join(sep)))
                    }
                    _ => Err(CustomLangError::type_err("join() requires (array, string)")),
                }
            }
            "substring" => {
                if !(2..=3).contains(&argc) {
                    return err_argc("2 or 3");
                }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Number(start)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let len = chars.len();
                        let s_idx = (*start as usize).min(len);
                        let e_idx = if argc == 3 {
                            match args[2] {
                                Value::Number(n) => (n as usize).min(len),
                                _ => {
                                    return Err(CustomLangError::type_err(
                                        "substring() end must be number",
                                    ))
                                }
                            }
                        } else {
                            len
                        };
                        Ok(Value::Str(chars[s_idx..e_idx.max(s_idx)].iter().collect()))
                    }
                    _ => Err(CustomLangError::type_err(
                        "substring() requires (string, number[, number])",
                    )),
                }
            }
            "to_upper" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
                    v => Err(CustomLangError::type_err(format!(
                        "to_upper() requires string, got {}",
                        v.type_name()
                    ))),
                }
            }
            "to_lower" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
                    v => Err(CustomLangError::type_err(format!(
                        "to_lower() requires string, got {}",
                        v.type_name()
                    ))),
                }
            }
            "trim" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
                    v => Err(CustomLangError::type_err(format!(
                        "trim() requires string, got {}",
                        v.type_name()
                    ))),
                }
            }
            "trim_start" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Str(s.trim_start().to_string())),
                    _ => Err(CustomLangError::type_err("trim_start() requires string")),
                }
            }
            "trim_end" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Str(s.trim_end().to_string())),
                    _ => Err(CustomLangError::type_err("trim_end() requires string")),
                }
            }
            "starts_with" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(prefix)) => {
                        Ok(Value::Bool(s.starts_with(prefix.as_str())))
                    }
                    _ => Err(CustomLangError::type_err(
                        "starts_with() requires (string, string)",
                    )),
                }
            }
            "ends_with" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(suffix)) => {
                        Ok(Value::Bool(s.ends_with(suffix.as_str())))
                    }
                    _ => Err(CustomLangError::type_err(
                        "ends_with() requires (string, string)",
                    )),
                }
            }
            "contains" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(sub.as_str()))),
                    _ => Err(CustomLangError::type_err(
                        "contains() requires (string, string)",
                    )),
                }
            }
            "replace" => {
                if argc != 3 {
                    return err_argc("3");
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Str(s), Value::Str(from), Value::Str(to)) => {
                        Ok(Value::Str(s.replace(from.as_str(), to)))
                    }
                    _ => Err(CustomLangError::type_err(
                        "replace() requires (string, string, string)",
                    )),
                }
            }
            "char_at" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Number(n)) => Ok(s
                        .chars()
                        .nth(*n as usize)
                        .map(|c| Value::Str(c.to_string()))
                        .unwrap_or(Value::Str(String::new()))),
                    _ => Err(CustomLangError::type_err(
                        "char_at() requires (string, number)",
                    )),
                }
            }
            "char_code" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Number(
                        s.chars().next().map(|c| c as u32 as f64).unwrap_or(0.0),
                    )),
                    _ => Err(CustomLangError::type_err("char_code() requires string")),
                }
            }
            "format" => {
                if argc < 1 {
                    return err_argc("1+");
                }
                match &args[0] {
                    Value::Str(template) => {
                        let mut result = template.clone();
                        for (i, arg) in args[1..].iter().enumerate() {
                            result = result.replace(&format!("{{{i}}}"), &arg.to_string());
                            result = result.replacen("{}", &arg.to_string(), 1);
                        }
                        Ok(Value::Str(result))
                    }
                    _ => Err(CustomLangError::type_err(
                        "format() first argument must be string",
                    )),
                }
            }

            // ── Object ────────────────────────────────────────────────────
            "keys" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Object(obj) => {
                        let keys: Vec<Value> =
                            obj.borrow().keys().map(|k| Value::Str(k.clone())).collect();
                        Ok(Value::make_array(keys))
                    }
                    _ => Err(CustomLangError::type_err("keys() requires object")),
                }
            }
            "values" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Object(obj) => {
                        let vals: Vec<Value> = obj.borrow().values().cloned().collect();
                        Ok(Value::make_array(vals))
                    }
                    _ => Err(CustomLangError::type_err("values() requires object")),
                }
            }
            "entries" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Object(obj) => {
                        let entries: Vec<Value> = obj
                            .borrow()
                            .iter()
                            .map(|(k, v)| Value::make_array(vec![Value::Str(k.clone()), v.clone()]))
                            .collect();
                        Ok(Value::make_array(entries))
                    }
                    _ => Err(CustomLangError::type_err("entries() requires object")),
                }
            }
            "has_key" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Object(obj), Value::Str(key)) => {
                        Ok(Value::Bool(obj.borrow().contains_key(key.as_str())))
                    }
                    _ => Err(CustomLangError::type_err(
                        "has_key() requires (object, string)",
                    )),
                }
            }
            "delete_key" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Object(obj), Value::Str(key)) => {
                        obj.borrow_mut().remove(key.as_str());
                        Ok(Value::Null)
                    }
                    _ => Err(CustomLangError::type_err(
                        "delete_key() requires (object, string)",
                    )),
                }
            }

            // ── File I/O ──────────────────────────────────────────────────
            "read_file" => {
                if argc != 1 {
                    return err_argc("1");
                }
                match &args[0] {
                    Value::Str(path) => std::fs::read_to_string(path)
                        .map(Value::Str)
                        .map_err(|e| CustomLangError::io_err(format!("read_file('{path}'): {e}"))),
                    _ => Err(CustomLangError::type_err(
                        "read_file() requires string path",
                    )),
                }
            }
            "write_file" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Str(path), content) => {
                        std::fs::write(path, content.to_string()).map_err(|e| {
                            CustomLangError::io_err(format!("write_file('{path}'): {e}"))
                        })?;
                        Ok(Value::Bool(true))
                    }
                    _ => Err(CustomLangError::type_err(
                        "write_file() requires (string, value)",
                    )),
                }
            }
            "append_file" => {
                if argc != 2 {
                    return err_argc("2");
                }
                match (&args[0], &args[1]) {
                    (Value::Str(path), content) => {
                        use std::io::Write as IoWrite;
                        let mut file = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .map_err(|e| {
                                CustomLangError::io_err(format!("append_file('{path}'): {e}"))
                            })?;
                        write!(file, "{}", content).map_err(|e| {
                            CustomLangError::io_err(format!("append_file('{path}'): {e}"))
                        })?;
                        Ok(Value::Bool(true))
                    }
                    _ => Err(CustomLangError::type_err(
                        "append_file() requires (string, value)",
                    )),
                }
            }

            // ── Misc ──────────────────────────────────────────────────────
            "assert" => {
                if !(1..=2).contains(&argc) {
                    return err_argc("1 or 2");
                }
                if !args[0].is_truthy() {
                    let msg = if argc == 2 {
                        args[1].to_string()
                    } else {
                        "assertion failed".to_string()
                    };
                    return Err(CustomLangError::runtime(msg));
                }
                Ok(Value::Null)
            }
            "exit" => {
                let code = if argc == 1 {
                    match args[0] {
                        Value::Number(n) => n as i32,
                        _ => 0,
                    }
                } else {
                    0
                };
                std::process::exit(code);
            }
            "now" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as f64)
                    .unwrap_or(0.0);
                Ok(Value::Number(ms))
            }

            _ => Err(
                CustomLangError::runtime(format!("unknown builtin function '{name}'"))
                    .with_pos(pos),
            ),
        }
    }

    // ─── helpers ──────────────────────────────────────────────────────────

    fn extract_numbers(args: &[Value], fn_name: &str, pos: &Position) -> Result<Vec<f64>> {
        args.iter()
            .map(|v| match v {
                Value::Number(n) => Ok(*n),
                _ => Err(
                    CustomLangError::type_err(format!("{fn_name}() requires numbers"))
                        .with_pos(pos),
                ),
            })
            .collect()
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────── Extension trait ─────────────────────────────

trait ErrorExt {
    fn with_pos(self, pos: &Position) -> Self;
}

impl ErrorExt for CustomLangError {
    fn with_pos(self, _pos: &Position) -> Self {
        // Positions are already in error messages from lex/parse;
        // for runtime errors we just keep the message as-is.
        self
    }
}
