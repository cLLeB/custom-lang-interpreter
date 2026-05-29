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
    BreakLabel(String),
    ContinueLabel(String),
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
                Signal::Break | Signal::Continue | Signal::BreakLabel(_) | Signal::ContinueLabel(_) => return Err(CustomLangError::runtime("'break'/'continue' outside loop")),
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
                Signal::None | Signal::Break | Signal::Continue | Signal::BreakLabel(_) | Signal::ContinueLabel(_) => {}
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
                        Signal::Continue | Signal::ContinueLabel(_) => continue,
                        s @ Signal::BreakLabel(_) => return Ok(s),
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
                        Signal::Continue | Signal::ContinueLabel(_) => {}
                        s @ Signal::BreakLabel(_) => return Ok(s),
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
                        Ok(s @ Signal::BreakLabel(_)) => break Ok(s),
                        Ok(Signal::Continue) | Ok(Signal::ContinueLabel(_)) | Ok(Signal::None) | Ok(Signal::ExprValue(_)) => {}
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
                        Ok(s @ Signal::BreakLabel(_)) => { result = Ok(s); break; }
                        Ok(Signal::Continue) | Ok(Signal::ContinueLabel(_)) | Ok(Signal::None) | Ok(Signal::ExprValue(_)) => {}
                    }
                }
                self.env = outer;
                result
            }
            Stmt::Function { name, params, body, is_static: _, is_generator, is_async, .. } => {
                let fd = Rc::new(FnData {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.env),
                    is_generator: *is_generator,
                    is_async: *is_async,
                });
                Env::define(&self.env, name, Value::Function(fd));
                Ok(Signal::None)
            }
            Stmt::LetDestructArray { elems, init, pos } => {
                let val = self.eval_expr(init)?;
                let arr = match &val {
                    Value::Array(a) => a.borrow().clone(),
                    _ => return Err(CustomLangError::type_err(format!("array destructuring requires array, got {}", val.type_name())).with_pos(pos)),
                };
                let mut idx = 0usize;
                for elem in elems {
                    match elem {
                        DestructElem::Skip => { idx += 1; }
                        DestructElem::Bind { name, default } => {
                            let v = if idx < arr.len() { arr[idx].clone() } else if let Some(d) = default { self.eval_expr(d)? } else { Value::Null };
                            Env::define(&self.env, name, v);
                            idx += 1;
                        }
                        DestructElem::Rest(name) => {
                            let rest = Value::make_array(arr[idx..].to_vec());
                            Env::define(&self.env, name, rest);
                            break;
                        }
                    }
                }
                Ok(Signal::None)
            }
            Stmt::LetDestructObject { fields, init, pos } => {
                let val = self.eval_expr(init)?;
                let obj = match &val {
                    Value::Object(o) => o.borrow().clone(),
                    Value::Instance(i) => i.borrow().fields.clone(),
                    _ => return Err(CustomLangError::type_err(format!("object destructuring requires object, got {}", val.type_name())).with_pos(pos)),
                };
                for field in fields {
                    let bind_name = field.alias.as_ref().unwrap_or(&field.key);
                    let v = if let Some(v) = obj.get(&field.key) {
                        v.clone()
                    } else if let Some(d) = &field.default {
                        self.eval_expr(d)?
                    } else {
                        Value::Null
                    };
                    Env::define(&self.env, bind_name, v);
                }
                Ok(Signal::None)
            }
            Stmt::ForOf { var, iter, body, .. } => {
                // for-of iterates array elements (same as for-in for arrays)
                let iter_val = self.eval_expr(iter)?;
                let items: Vec<Value> = match &iter_val {
                    Value::Array(arr) => arr.borrow().clone(),
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    Value::Generator(g) => {
                        let mut items = Vec::new();
                        loop {
                            let v = { let mut gs = g.borrow_mut(); if gs.done || gs.index >= gs.values.len() { break; } let v = gs.values[gs.index].clone(); gs.index += 1; v };
                            items.push(v);
                        }
                        items
                    }
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
                        Ok(s @ Signal::BreakLabel(_)) => { result = Ok(s); break; }
                        Ok(Signal::Continue) | Ok(Signal::ContinueLabel(_)) | Ok(Signal::None) | Ok(Signal::ExprValue(_)) => {}
                    }
                }
                self.env = outer;
                result
            }
            Stmt::Labeled { label, body, .. } => {
                match self.exec_stmt(body)? {
                    Signal::BreakLabel(l) if &l == label => Ok(Signal::None),
                    other => Ok(other),
                }
            }
            Stmt::Enum { name, variants, .. } => {
                let mut map = std::collections::HashMap::new();
                for (vname, val_expr) in variants {
                    let v = if let Some(e) = val_expr { self.eval_expr(e)? } else { Value::Null };
                    map.insert(vname.clone(), v);
                }
                Env::define(&self.env, name, Value::make_object(map));
                Ok(Signal::None)
            }
            Stmt::TypeAlias { .. } | Stmt::Interface { .. } => Ok(Signal::None),
            Stmt::Decorator { name, target, .. } => {
                // Execute the target statement first
                self.exec_stmt(target)?;
                // Then apply decorator if it resolves to a function
                if let Stmt::Function { name: fn_name, .. } = target.as_ref() {
                    if let Some(decorator_fn) = Env::get(&self.env, name) {
                        if let Some(fn_val) = Env::get(&self.env, fn_name) {
                            let pos = Position::default();
                            let decorated = self.call_value(decorator_fn, vec![fn_val], None, &pos)?;
                            Env::define(&self.env, fn_name, decorated);
                        }
                    }
                }
                Ok(Signal::None)
            }
            Stmt::Return { value, .. } => {
                let v = match value { Some(e) => self.eval_expr(e)?, None => Value::Null };
                Ok(Signal::Return(v))
            }
            Stmt::Break { label, .. } => {
                if let Some(l) = label { Ok(Signal::BreakLabel(l.clone())) } else { Ok(Signal::Break) }
            }
            Stmt::Continue { label, .. } => {
                if let Some(l) = label { Ok(Signal::ContinueLabel(l.clone())) } else { Ok(Signal::Continue) }
            }
            Stmt::Print { expr, .. } => {
                let v = self.eval_expr(expr)?;
                println!("{v}");
                Ok(Signal::None)
            }
            Stmt::Import { path, alias, names, .. } => {
                self.exec_import_with_names(path, alias.as_deref(), names)?;
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
        // Note: BreakLabel/ContinueLabel propagate upward through blocks
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
                    Value::Class(cls) => {
                        // Set static field — use Rc::ptr_eq workaround via unsafe
                        // We can't mutate ClassData through Rc, so we update via env
                        if let Expr::Var { name: class_name, .. } = object.as_ref() {
                            if let Some(Value::Class(cls_ref)) = Env::get(&self.env, class_name) {
                                unsafe {
                                    let cls_ptr = Rc::as_ptr(&cls_ref) as *mut ClassData;
                                    (*cls_ptr).static_fields.insert(prop.clone(), new_val.clone());
                                }
                                return Ok(new_val);
                            }
                        }
                        // Fallback: try direct mutation if we have unique access
                        let _ = cls; // suppress warning
                        Err(CustomLangError::type_err(format!("cannot set static field '{}' on class", prop)).with_pos(pos))
                    }
                    Value::Object(obj) => { obj.borrow_mut().insert(prop.clone(), new_val.clone()); Ok(new_val) }
                    Value::Instance(inst) => {
                        // Check setter
                        let setter = inst.borrow().class.setters.get(prop).cloned();
                        if let Some(s) = setter {
                            self.call_fn(&s, vec![new_val.clone()], Some(obj_val.clone()), pos)?;
                            return Ok(new_val);
                        }
                        inst.borrow_mut().fields.insert(prop.clone(), new_val.clone());
                        Ok(new_val)
                    }
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
                        // Special case: generator.next() / generator.to_array()
                        if let Value::Generator(_) = &receiver {
                            let method_name = name.as_str();
                            let arg_vals = self.eval_args(args)?;
                            let mut call_args = vec![receiver];
                            call_args.extend(arg_vals);
                            return self.call_builtin(match method_name { "next" => "gen_next", "to_array" => "gen_to_array", _ => return Err(CustomLangError::runtime(format!("generator has no method '{method_name}'")).with_pos(pos)) }, call_args, pos);
                        }
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
                    is_generator: false,
                    is_async: false,
                });
                Ok(Value::Function(fd))
            }

            Expr::Yield { value, pos } => {
                // yield is handled in generator execution context
                let v = if let Some(e) = value { self.eval_expr(e)? } else { Value::Null };
                self.thrown_value = Some(v.clone());
                Err(CustomLangError::runtime("yield outside generator").with_pos(pos))
            }

            Expr::Await { expr, .. } => {
                // Single-threaded: await is a no-op, just evaluates synchronously
                self.eval_expr(expr)
            }

            Expr::Pipe { left, right, pos } => {
                let lval = self.eval_expr(left)?;
                let rfn = self.eval_expr(right)?;
                self.call_value(rfn, vec![lval], None, pos)
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

    fn get_property(&mut self, obj: &Value, name: &str, pos: &Position) -> Result<Value> {
        match obj {
            Value::Instance(inst) => {
                let inst_b = inst.borrow();
                // Check getter first
                let getter = inst_b.class.getters.get(name).cloned();
                if let Some(g) = getter {
                    drop(inst_b);
                    return self.call_fn(&g, vec![], Some(obj.clone()), pos);
                }
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
                if let Some(v) = cls.static_fields.get(name) { return Ok(v.clone()); }
                if let Some(m) = cls.static_methods.get(name) { return Ok(Value::Function(Rc::clone(m))); }
                Ok(Value::Null)
            }
            Value::Generator(_) => {
                match name {
                    "next" => Ok(Value::Builtin("gen_next".to_string())),
                    "to_array" => Ok(Value::Builtin("gen_to_array".to_string())),
                    _ => Ok(Value::Null),
                }
            }
            Value::Function(fd) => {
                match name {
                    "name" => Ok(Value::Str(fd.name.clone())),
                    _ => Ok(Value::Null),
                }
            }
            Value::Builtin(n) => {
                match name {
                    "name" => Ok(Value::Str(n.clone())),
                    _ => Ok(Value::Null),
                }
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
            Value::Builtin(name) => {
                // For builtins that expect a receiver as first arg (collection methods, etc.)
                // prepend `this` if provided
                let final_args = if let Some(t) = this {
                    let needs_this = name.starts_with("coll_") || name.starts_with("gen_");
                    if needs_this {
                        let mut new_args = vec![t];
                        new_args.extend(args);
                        new_args
                    } else {
                        args
                    }
                } else {
                    args
                };
                self.call_builtin(&name, final_args, pos)
            }
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

        // Generator function: collect all yielded values
        if fd.is_generator {
            let mut yielded = Vec::new();
            // Execute body, collecting yields via YieldSignal mechanism
            let body = fd.body.clone();
            let result = self.collect_generator_yields(&body, &mut yielded);
            self.call_depth -= 1;
            self.env = outer;
            result?;
            let gen = Rc::new(RefCell::new(GeneratorState { values: yielded, index: 0, done: false }));
            return Ok(Value::Generator(gen));
        }

        let result = self.exec_stmt(&fd.body);
        self.call_depth -= 1;
        self.env = outer;

        match result? {
            Signal::Return(v) => Ok(v),
            _ => Ok(Value::Null),
        }
    }

    fn collect_generator_yields(&mut self, body: &Stmt, yielded: &mut Vec<Value>) -> Result<()> {
        match body {
            Stmt::Block { stmts, .. } => {
                for stmt in stmts {
                    match self.collect_generator_yields(stmt, yielded) {
                        Ok(()) => {}
                        Err(e) if matches!(e, CustomLangError::ThrownException) => {
                            if let Some(v) = self.thrown_value.take() { yielded.push(v); }
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            Stmt::Expr { expr, .. } => {
                if let Expr::Yield { value, .. } = expr {
                    let v = if let Some(e) = value { self.eval_expr(e)? } else { Value::Null };
                    yielded.push(v);
                } else {
                    self.eval_expr(expr)?;
                }
            }
            Stmt::While { cond, body, .. } => {
                loop {
                    if !self.eval_expr(cond)?.is_truthy() { break; }
                    self.collect_generator_yields(body, yielded)?;
                }
            }
            Stmt::For { init, cond, update, body, .. } => {
                if let Some(i) = init { self.exec_stmt(i)?; }
                loop {
                    if let Some(c) = cond { if !self.eval_expr(c)?.is_truthy() { break; } }
                    self.collect_generator_yields(body, yielded)?;
                    if let Some(u) = update { self.eval_expr(u)?; }
                }
            }
            Stmt::If { cond, then_b, else_b, .. } => {
                if self.eval_expr(cond)?.is_truthy() { self.collect_generator_yields(then_b, yielded)?; }
                else if let Some(e) = else_b { self.collect_generator_yields(e, yielded)?; }
            }
            Stmt::Return { value, .. } => {
                if let Some(e) = value { self.eval_expr(e)?; }
            }
            other => { self.exec_stmt(other)?; }
        }
        Ok(())
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
        let mut static_fields = HashMap::new();
        let mut getters = HashMap::new();
        let mut setters = HashMap::new();

        for method in methods {
            match method {
                Stmt::Function { name: mn, params, body, is_static, is_generator, is_async, .. } => {
                    let fd = Rc::new(FnData {
                        name: mn.clone(), params: params.clone(), body: body.clone(),
                        closure: Rc::clone(&self.env), is_generator: *is_generator, is_async: *is_async,
                    });
                    if mn.starts_with("__get_") && mn.ends_with("__") {
                        let prop = mn.trim_start_matches("__get_").trim_end_matches("__").to_string();
                        getters.insert(prop, fd);
                    } else if mn.starts_with("__set_") && mn.ends_with("__") {
                        let prop = mn.trim_start_matches("__set_").trim_end_matches("__").to_string();
                        setters.insert(prop, fd);
                    } else if mn.starts_with("__static_field_") && mn.ends_with("__") {
                        let field_name = mn.trim_start_matches("__static_field_").trim_end_matches("__").to_string();
                        // evaluate: call the fd with no args to get the value
                        let val = self.call_fn(&fd, vec![], None, &Position::default())?;
                        static_fields.insert(field_name, val);
                    } else if *is_static {
                        static_method_map.insert(mn.clone(), fd);
                    } else {
                        method_map.insert(mn.clone(), fd);
                    }
                }
                Stmt::Decorator { name: dname, target, .. } => {
                    // Handle method decorators — apply decorator to the function
                    if let Stmt::Function { name: mn, params, body, is_static, is_generator, is_async, .. } = target.as_ref() {
                        let fd = Rc::new(FnData {
                            name: mn.clone(), params: params.clone(), body: body.clone(),
                            closure: Rc::clone(&self.env), is_generator: *is_generator, is_async: *is_async,
                        });
                        let fn_val = Value::Function(fd);
                        let decorated = if let Some(decorator) = Env::get(&self.env, dname) {
                            self.call_value(decorator, vec![fn_val], None, &Position::default())?
                        } else { fn_val };
                        if let Value::Function(dfd) = decorated {
                            if *is_static { static_method_map.insert(mn.clone(), dfd); } else { method_map.insert(mn.clone(), dfd); }
                        }
                    }
                }
                _ => {}
            }
        }

        let cls = Rc::new(ClassData {
            name: name.to_string(),
            methods: method_map,
            static_methods: static_method_map,
            static_fields,
            getters,
            setters,
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

    fn exec_import_with_names(&mut self, path: &str, alias: Option<&str>, names: &[(String, Option<String>)]) -> Result<()> {
        if !names.is_empty() {
            // Selective import: import { a, b as c } from "module"
            if path.starts_with("std/") {
                let ns = self.get_std_module(path)?;
                for (name, alias_name) in names {
                    let bind = alias_name.as_ref().unwrap_or(name);
                    let val = ns.get(name).cloned().unwrap_or(Value::Null);
                    Env::define(&self.env, bind, val);
                }
            } else {
                // Load module and selectively import
                self.exec_import(path, None)?;
                // Names already imported flat; nothing extra to do
            }
            return Ok(());
        }
        self.exec_import(path, alias)
    }

    fn get_std_module(&self, path: &str) -> Result<HashMap<String, Value>> {
        match path {
            "std/json" => Ok(Self::make_json_module()),
            "std/random" => Ok(Self::make_random_module()),
            "std/math" => Ok(Self::make_math_module()),
            "std/fs" => Ok(Self::make_fs_module()),
            "std/path" => Ok(Self::make_path_module()),
            "std/process" => Ok(Self::make_process_module()),
            "std/http" => Ok(Self::make_http_module()),
            "std/string" => Ok(Self::make_string_module()),
            "std/array" => Ok(Self::make_array_module()),
            "std/object" => Ok(Self::make_object_module()),
            "std/datetime" => Ok(Self::make_datetime_module()),
            "std/encoding" => Ok(Self::make_encoding_module()),
            "std/crypto" => Ok(Self::make_crypto_module()),
            "std/collections" => Ok(Self::make_collections_module()),
            "std/testing" => Ok(Self::make_testing_module()),
            "std/regex" => Ok(Self::make_regex_module()),
            "std/os" => Ok(Self::make_process_module()),
            "std/weak" => Ok(Self::make_weak_module()),
            _ => Err(CustomLangError::runtime(format!("unknown standard module '{path}'"))),
        }
    }

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
        let mod_name = alias.unwrap_or_else(|| path.trim_start_matches("std/")).to_string();
        let ns = self.get_std_module(path)?;
        Env::define(&self.env, &mod_name, Value::make_object(ns));
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
        m.insert("lerp".to_string(), Value::Builtin("math_lerp".to_string()));
        m.insert("degrees".to_string(), Value::Builtin("math_degrees".to_string()));
        m.insert("radians".to_string(), Value::Builtin("math_radians".to_string()));
        m.insert("atan2".to_string(), Value::Builtin("math_atan2".to_string()));
        m.insert("asin".to_string(), Value::Builtin("math_asin".to_string()));
        m.insert("acos".to_string(), Value::Builtin("math_acos".to_string()));
        m.insert("atan".to_string(), Value::Builtin("math_atan".to_string()));
        m.insert("exp".to_string(), Value::Builtin("math_exp".to_string()));
        m.insert("log2".to_string(), Value::Builtin("math_log2".to_string()));
        m.insert("log10".to_string(), Value::Builtin("math_log10".to_string()));
        m.insert("cbrt".to_string(), Value::Builtin("math_cbrt".to_string()));
        m.insert("trunc".to_string(), Value::Builtin("math_trunc".to_string()));
        m
    }

    fn make_fs_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("read_text","fs_read_text"),("write_text","fs_write_text"),("append_text","fs_append_text"),
                       ("exists","fs_exists"),("delete","fs_delete"),("rename","fs_rename"),("copy","fs_copy"),
                       ("mkdir","fs_mkdir"),("mkdir_all","fs_mkdir_all"),("rmdir","fs_rmdir"),
                       ("list_dir","fs_list_dir"),("is_file","fs_is_file"),("is_dir","fs_is_dir"),
                       ("file_size","fs_file_size"),("last_modified","fs_last_modified"),
                       ("temp_file","fs_temp_file"),("temp_dir","fs_temp_dir"),("read_bytes","fs_read_text"),("write_bytes","fs_write_text")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_path_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("join","path_join"),("dirname","path_dirname"),("basename","path_basename"),
                       ("stem","path_stem"),("extension","path_extension"),("absolute","path_absolute"),
                       ("normalize","path_normalize"),("split","path_split"),("is_absolute","path_is_absolute")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_process_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("args","proc_args"),("env","proc_env"),("env_all","proc_env_all"),
                       ("cwd","proc_cwd"),("chdir","proc_chdir"),("exit","exit"),
                       ("pid","proc_pid"),("platform","proc_platform"),("run","proc_run")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_http_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("get","http_get"),("post","http_post"),("put","http_put"),
                       ("delete","http_delete"),("patch","http_patch")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_string_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("repeat","repeat"),("pad_start","pad_start"),("pad_end","pad_end"),
                       ("at","str_at"),("index_of","index_of"),("last_index_of","str_last_index_of"),
                       ("char_codes","str_char_codes"),("from_char_codes","str_from_char_codes"),
                       ("is_digit","str_is_digit"),("is_alpha","str_is_alpha"),("is_alphanumeric","str_is_alnum"),
                       ("is_whitespace","str_is_whitespace"),("is_upper","str_is_upper"),("is_lower","str_is_lower"),
                       ("count_occurrences","str_count_occurrences"),("reverse","str_reverse"),
                       ("word_count","str_word_count"),("lines","str_lines"),("format","format")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_array_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("flat","flat"),("flat_map","flat_map"),("zip","zip"),("unzip","arr_unzip"),
                       ("chunk","chunk"),("unique","unique"),("group_by","arr_group_by"),
                       ("partition","arr_partition"),("rotate","arr_rotate"),("take","arr_take"),
                       ("drop","arr_drop"),("take_while","arr_take_while"),("drop_while","arr_drop_while"),
                       ("flatten_deep","arr_flatten_deep"),("count","count"),("sum","sum"),("average","average"),
                       ("fill","arr_fill"),("fill_with","arr_fill_with"),
                       ("min_by","arr_min_by"),("max_by","arr_max_by"),("sort_by","arr_sort_by"),
                       ("difference","arr_difference"),("intersection","arr_intersection"),("union","arr_union")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_object_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("merge","obj_merge"),("deep_clone","obj_deep_clone"),
                       ("get_path","obj_get_path"),("set_path","obj_set_path"),
                       ("omit","obj_omit"),("pick","obj_pick"),("map_values","obj_map_values"),
                       ("map_keys","obj_map_keys"),("filter_values","obj_filter_values"),
                       ("invert","obj_invert"),("from_entries","obj_from_entries")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_datetime_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("now","dt_now"),("from_timestamp","dt_from_timestamp"),("new","dt_new"),
                       ("format","dt_format"),("parse","dt_parse")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_encoding_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("url_encode","enc_url_encode"),("url_decode","enc_url_decode"),
                       ("html_encode","enc_html_encode"),("html_decode","enc_html_decode"),
                       ("base64_encode","crypto_base64_encode"),("base64_decode","crypto_base64_decode")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_crypto_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("sha256","crypto_sha256"),("sha512","crypto_sha512"),("md5","crypto_md5"),
                       ("hmac_sha256","crypto_hmac_sha256"),("base64_encode","crypto_base64_encode"),
                       ("base64_decode","crypto_base64_decode"),("hex_encode","crypto_hex_encode"),
                       ("hex_decode","crypto_hex_decode"),("random_bytes","crypto_random_bytes"),
                       ("compare_secure","crypto_compare_secure")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_collections_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        // Queue, Stack, Deque, LinkedList are implemented as classes in the interpreter
        // We provide factory functions
        for (k, v) in [("Queue","coll_queue"),("Stack","coll_stack"),("Deque","coll_deque"),("LinkedList","coll_linked_list")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_testing_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("test","test_run"),("describe","test_describe"),("assert_eq","test_assert_eq"),
                       ("assert_neq","test_assert_neq"),("assert_throws","test_assert_throws"),
                       ("assert_true","test_assert_true"),("assert_false","test_assert_false"),
                       ("before_each","test_before_each"),("after_each","test_after_each")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_regex_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        for (k, v) in [("new","regex_new"),("test","regex_test"),("match","regex_match"),
                       ("match_all","regex_match_all"),("replace","regex_replace"),("replace_all","regex_replace_all")] {
            m.insert(k.to_string(), Value::Builtin(v.to_string()));
        }
        m
    }

    fn make_weak_module() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        m.insert("WeakMap".to_string(), Value::Builtin("weakmap_new".to_string()));
        m.insert("WeakRef".to_string(), Value::Builtin("weakref_new".to_string()));
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
            "json_parse","json_stringify","json_is_valid",
            "random_float","random_int","random_bool","random_choice","random_shuffle",
            "math_clamp","math_sign","math_hypot","math_gcd","math_lcm",
            "math_factorial","math_is_nan","math_is_finite",
            "math_lerp","math_degrees","math_radians","math_atan2","math_asin","math_acos",
            "math_atan","math_exp","math_log2","math_log10","math_cbrt","math_trunc",
            "flat","flat_map","zip","chunk","unique","count","sum","average",
            "repeat","pad_start","pad_end",
            // fs
            "fs_read_text","fs_write_text","fs_append_text","fs_exists","fs_delete",
            "fs_rename","fs_copy","fs_mkdir","fs_mkdir_all","fs_rmdir","fs_list_dir",
            "fs_is_file","fs_is_dir","fs_file_size","fs_last_modified","fs_temp_file","fs_temp_dir",
            // path
            "path_join","path_dirname","path_basename","path_stem","path_extension",
            "path_absolute","path_normalize","path_split","path_is_absolute",
            // process
            "proc_args","proc_env","proc_env_all","proc_cwd","proc_chdir","proc_pid","proc_platform","proc_run",
            // http
            "http_get","http_post","http_put","http_delete","http_patch",
            // string extras
            "str_at","str_last_index_of","str_char_codes","str_from_char_codes",
            "str_is_digit","str_is_alpha","str_is_alnum","str_is_whitespace",
            "str_is_upper","str_is_lower","str_count_occurrences","str_reverse",
            "str_word_count","str_lines",
            // array extras
            "arr_unzip","arr_group_by","arr_partition","arr_rotate","arr_take","arr_drop",
            "arr_take_while","arr_drop_while","arr_flatten_deep","arr_fill","arr_fill_with",
            "arr_min_by","arr_max_by","arr_sort_by","arr_difference","arr_intersection","arr_union",
            // object
            "obj_merge","obj_deep_clone","obj_get_path","obj_set_path","obj_omit","obj_pick",
            "obj_map_values","obj_map_keys","obj_filter_values","obj_invert","obj_from_entries",
            // datetime
            "dt_now","dt_from_timestamp","dt_new","dt_format","dt_parse",
            // encoding
            "enc_url_encode","enc_url_decode","enc_html_encode","enc_html_decode",
            // crypto
            "crypto_sha256","crypto_sha512","crypto_md5","crypto_hmac_sha256",
            "crypto_base64_encode","crypto_base64_decode","crypto_hex_encode","crypto_hex_decode",
            "crypto_random_bytes","crypto_compare_secure",
            // regex
            "regex_new","regex_test","regex_match","regex_match_all","regex_replace","regex_replace_all",
            // collections
            "coll_queue","coll_stack","coll_deque","coll_linked_list",
            // testing
            "test_run","test_describe","test_assert_eq","test_assert_neq","test_assert_throws",
            "test_assert_true","test_assert_false","test_before_each","test_after_each",
            // FP helpers
            "partial","curry","compose","pipe_fn","memoize","update","set_at",
            // immutable
            "deep_freeze",
            // weak
            "weakmap_new","weakref_new",
            // generators
            "gen_next","gen_to_array",
            // misc
            "get_type","is_function","is_class","is_instance","instanceof_check",
            "Promise","Ok","Err","Some","None_val","is_ok","is_err","unwrap","unwrap_or",
            "memoize","partial","curry","compose",
            // format
            "fmt",
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

            // ── Math extended ─────────────────────────────────────────────
            "math_lerp" => { if argc != 3 { return err_argc("3"); } match (&args[0],&args[1],&args[2]) { (Value::Number(a),Value::Number(b),Value::Number(t)) => Ok(Value::Number(a + (b-a)*t)), _ => Err(CustomLangError::type_err("lerp() requires 3 numbers")) } }
            "math_degrees" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(r) => Ok(Value::Number(r.to_degrees())), _ => Err(CustomLangError::type_err("degrees() requires number")) } }
            "math_radians" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(d) => Ok(Value::Number(d.to_radians())), _ => Err(CustomLangError::type_err("radians() requires number")) } }
            "math_atan2" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Number(y),Value::Number(x)) => Ok(Value::Number(y.atan2(*x))), _ => Err(CustomLangError::type_err("atan2() requires 2 numbers")) } }
            "math_asin" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.asin())), _ => Err(CustomLangError::type_err("asin() requires number")) } }
            "math_acos" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.acos())), _ => Err(CustomLangError::type_err("acos() requires number")) } }
            "math_atan" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.atan())), _ => Err(CustomLangError::type_err("atan() requires number")) } }
            "math_exp" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.exp())), _ => Err(CustomLangError::type_err("exp() requires number")) } }
            "math_log2" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.log2())), _ => Err(CustomLangError::type_err("log2() requires number")) } }
            "math_log10" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.log10())), _ => Err(CustomLangError::type_err("log10() requires number")) } }
            "math_cbrt" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.cbrt())), _ => Err(CustomLangError::type_err("cbrt() requires number")) } }
            "math_trunc" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => Ok(Value::Number(n.trunc())), _ => Err(CustomLangError::type_err("trunc() requires number")) } }

            // ── String extras ─────────────────────────────────────────────
            "str_at" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(s),Value::Number(n)) => { let chars: Vec<char> = s.chars().collect(); let len = chars.len(); let i = if *n < 0.0 { (len as f64 + n) as usize } else { *n as usize }; Ok(chars.get(i).map(|c| Value::Str(c.to_string())).unwrap_or(Value::Null)) } _ => Err(CustomLangError::type_err("at() requires (string, number)")) } }
            "str_last_index_of" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(s),Value::Str(sub)) => Ok(Value::Number(s.rfind(sub.as_str()).map(|i| i as f64).unwrap_or(-1.0))), _ => Err(CustomLangError::type_err("last_index_of() requires (string, string)")) } }
            "str_char_codes" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::make_array(s.chars().map(|c| Value::Number(c as u32 as f64)).collect())), _ => Err(CustomLangError::type_err("char_codes() requires string")) } }
            "str_from_char_codes" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Array(arr) => { let s: String = arr.borrow().iter().filter_map(|v| if let Value::Number(n) = v { char::from_u32(*n as u32) } else { None }).collect(); Ok(Value::Str(s)) } _ => Err(CustomLangError::type_err("from_char_codes() requires array")) } }
            "str_is_digit" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Bool(s.chars().all(|c| c.is_ascii_digit()))), _ => Err(CustomLangError::type_err("is_digit() requires string")) } }
            "str_is_alpha" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Bool(s.chars().all(|c| c.is_alphabetic()))), _ => Err(CustomLangError::type_err("is_alpha() requires string")) } }
            "str_is_alnum" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Bool(s.chars().all(|c| c.is_alphanumeric()))), _ => Err(CustomLangError::type_err("is_alphanumeric() requires string")) } }
            "str_is_whitespace" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Bool(s.chars().all(|c| c.is_whitespace()))), _ => Err(CustomLangError::type_err("is_whitespace() requires string")) } }
            "str_is_upper" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Bool(s.chars().all(|c| c.is_uppercase()))), _ => Err(CustomLangError::type_err("is_upper() requires string")) } }
            "str_is_lower" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Bool(s.chars().all(|c| c.is_lowercase()))), _ => Err(CustomLangError::type_err("is_lower() requires string")) } }
            "str_count_occurrences" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(s),Value::Str(sub)) => Ok(Value::Number(s.matches(sub.as_str()).count() as f64)), _ => Err(CustomLangError::type_err("count_occurrences() requires (string, string)")) } }
            "str_reverse" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())), _ => Err(CustomLangError::type_err("reverse_string() requires string")) } }
            "str_word_count" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Number(s.split_whitespace().count() as f64)), _ => Err(CustomLangError::type_err("word_count() requires string")) } }
            "str_lines" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::make_array(s.lines().map(|l| Value::Str(l.to_string())).collect())), _ => Err(CustomLangError::type_err("lines() requires string")) } }

            // ── Array extras ──────────────────────────────────────────────
            "arr_unzip" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Array(arr) => { let items = arr.borrow().clone(); let mut a: Vec<Value> = Vec::new(); let mut b: Vec<Value> = Vec::new(); for item in &items { if let Value::Array(pair) = item { let p = pair.borrow(); a.push(p.get(0).cloned().unwrap_or(Value::Null)); b.push(p.get(1).cloned().unwrap_or(Value::Null)); } } Ok(Value::make_array(vec![Value::make_array(a), Value::make_array(b)])) } _ => Err(CustomLangError::type_err("unzip() requires array")) } }
            "arr_group_by" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Array(arr) => { let cb = args[1].clone(); let items = arr.borrow().clone(); let mut groups: HashMap<String,Vec<Value>> = HashMap::new(); for item in items { let k = self.call_value(cb.clone(), vec![item.clone()], None, pos)?; groups.entry(k.to_string()).or_default().push(item); } let mut map = HashMap::new(); for (k,v) in groups { map.insert(k, Value::make_array(v)); } Ok(Value::make_object(map)) } _ => Err(CustomLangError::type_err("group_by() requires array")) } }
            "arr_partition" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Array(arr) => { let cb = args[1].clone(); let items = arr.borrow().clone(); let mut yes = Vec::new(); let mut no = Vec::new(); for item in items { if self.call_value(cb.clone(), vec![item.clone()], None, pos)?.is_truthy() { yes.push(item); } else { no.push(item); } } Ok(Value::make_array(vec![Value::make_array(yes), Value::make_array(no)])) } _ => Err(CustomLangError::type_err("partition() requires array")) } }
            "arr_rotate" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Array(arr),Value::Number(n)) => { let mut items = arr.borrow().clone(); let len = items.len(); if len > 0 { let n = ((*n as isize).rem_euclid(len as isize)) as usize; items.rotate_left(n); } Ok(Value::make_array(items)) } _ => Err(CustomLangError::type_err("rotate() requires (array, number)")) } }
            "arr_take" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Array(arr),Value::Number(n)) => { let items = arr.borrow().clone(); Ok(Value::make_array(items.into_iter().take(*n as usize).collect())) } _ => Err(CustomLangError::type_err("take() requires (array, number)")) } }
            "arr_drop" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Array(arr),Value::Number(n)) => { let items = arr.borrow().clone(); Ok(Value::make_array(items.into_iter().skip(*n as usize).collect())) } _ => Err(CustomLangError::type_err("drop() requires (array, number)")) } }
            "arr_take_while" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Array(arr) => { let cb = args[1].clone(); let items = arr.borrow().clone(); let mut result = Vec::new(); for item in items { if self.call_value(cb.clone(), vec![item.clone()], None, pos)?.is_truthy() { result.push(item); } else { break; } } Ok(Value::make_array(result)) } _ => Err(CustomLangError::type_err("take_while() requires array")) } }
            "arr_drop_while" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Array(arr) => { let cb = args[1].clone(); let items = arr.borrow().clone(); let mut skip = true; let mut result = Vec::new(); for item in items { if skip && self.call_value(cb.clone(), vec![item.clone()], None, pos)?.is_truthy() { continue; } skip = false; result.push(item); } Ok(Value::make_array(result)) } _ => Err(CustomLangError::type_err("drop_while() requires array")) } }
            "arr_flatten_deep" => { if argc != 1 { return err_argc("1"); } fn fd(v: &Value) -> Vec<Value> { match v { Value::Array(a) => a.borrow().iter().flat_map(fd).collect(), _ => vec![v.clone()] } } match &args[0] { Value::Array(_) => Ok(Value::make_array(fd(&args[0]))), _ => Err(CustomLangError::type_err("flatten_deep() requires array")) } }
            "arr_fill" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Number(n),v) => Ok(Value::make_array(vec![v.clone(); *n as usize])), _ => Err(CustomLangError::type_err("fill() requires (number, value)")) } }
            "arr_fill_with" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Number(n) => { let cb = args[1].clone(); let mut result = Vec::new(); for i in 0..*n as usize { result.push(self.call_value(cb.clone(), vec![Value::Number(i as f64)], None, pos)?); } Ok(Value::make_array(result)) } _ => Err(CustomLangError::type_err("fill_with() requires (number, fn)")) } }
            "arr_min_by" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Array(arr) => { let cb = args[1].clone(); let items = arr.borrow().clone(); if items.is_empty() { return Ok(Value::Null); } let mut min_item = items[0].clone(); let mut min_val = self.call_value(cb.clone(), vec![min_item.clone()], None, pos)?; for item in items[1..].iter() { let v = self.call_value(cb.clone(), vec![item.clone()], None, pos)?; if let (Value::Number(a), Value::Number(b)) = (&v, &min_val) { if a < b { min_val = v; min_item = item.clone(); } } } Ok(min_item) } _ => Err(CustomLangError::type_err("min_by() requires array")) } }
            "arr_max_by" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Array(arr) => { let cb = args[1].clone(); let items = arr.borrow().clone(); if items.is_empty() { return Ok(Value::Null); } let mut max_item = items[0].clone(); let mut max_val = self.call_value(cb.clone(), vec![max_item.clone()], None, pos)?; for item in items[1..].iter() { let v = self.call_value(cb.clone(), vec![item.clone()], None, pos)?; if let (Value::Number(a), Value::Number(b)) = (&v, &max_val) { if a > b { max_val = v; max_item = item.clone(); } } } Ok(max_item) } _ => Err(CustomLangError::type_err("max_by() requires array")) } }
            "arr_sort_by" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Array(arr) => { let cb = args[1].clone(); let mut items = arr.borrow().clone(); let mut error: Option<CustomLangError> = None; items.sort_by(|a,b| { if error.is_some() { return std::cmp::Ordering::Equal; } let av = self.call_value(cb.clone(), vec![a.clone()], None, pos); let bv = self.call_value(cb.clone(), vec![b.clone()], None, pos); match (av, bv) { (Ok(Value::Number(an)), Ok(Value::Number(bn))) => an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal), (Ok(Value::Str(as_)), Ok(Value::Str(bs))) => as_.cmp(&bs), (Err(e), _) | (_, Err(e)) => { error = Some(e); std::cmp::Ordering::Equal } _ => std::cmp::Ordering::Equal } }); if let Some(e) = error { return Err(e); } Ok(Value::make_array(items)) } _ => Err(CustomLangError::type_err("sort_by() requires array")) } }
            "arr_difference" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Array(a),Value::Array(b)) => { let bi = b.borrow().clone(); let result: Vec<Value> = a.borrow().iter().filter(|v| !bi.iter().any(|bv| v.equals(bv))).cloned().collect(); Ok(Value::make_array(result)) } _ => Err(CustomLangError::type_err("difference() requires 2 arrays")) } }
            "arr_intersection" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Array(a),Value::Array(b)) => { let bi = b.borrow().clone(); let result: Vec<Value> = a.borrow().iter().filter(|v| bi.iter().any(|bv| v.equals(bv))).cloned().collect(); Ok(Value::make_array(result)) } _ => Err(CustomLangError::type_err("intersection() requires 2 arrays")) } }
            "arr_union" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Array(a),Value::Array(b)) => { let mut result = a.borrow().clone(); for item in b.borrow().iter() { if !result.iter().any(|v| v.equals(item)) { result.push(item.clone()); } } Ok(Value::make_array(result)) } _ => Err(CustomLangError::type_err("union() requires 2 arrays")) } }

            // ── Object extras ─────────────────────────────────────────────
            "obj_merge" => {
                let mut map = HashMap::new();
                for arg in &args {
                    match arg { Value::Object(o) => { for (k,v) in o.borrow().iter() { map.insert(k.clone(), v.clone()); } } _ => {} }
                }
                Ok(Value::make_object(map))
            }
            "obj_deep_clone" => { if argc != 1 { return err_argc("1"); } Ok(deep_clone_value(&args[0])) }
            "obj_get_path" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (obj,Value::Str(path)) => { let mut current = obj.clone(); for key in path.split('.') { current = match &current { Value::Object(o) => o.borrow().get(key).cloned().unwrap_or(Value::Null), _ => Value::Null }; } Ok(current) } _ => Err(CustomLangError::type_err("get_path() requires (object, string)")) } }
            "obj_set_path" => { if argc != 3 { return err_argc("3"); } match (&args[0],&args[1]) { (Value::Object(obj),Value::Str(path)) => { let val = args[2].clone(); let keys: Vec<&str> = path.split('.').collect(); let mut current = obj.clone(); for (i, key) in keys.iter().enumerate() { if i == keys.len() - 1 { current.borrow_mut().insert(key.to_string(), val.clone()); } else { let next = current.borrow().get(*key).cloned().unwrap_or_else(|| Value::make_object(HashMap::new())); if let Value::Object(o) = next { current = o; } } } Ok(args[0].clone()) } _ => Err(CustomLangError::type_err("set_path() requires (object, string, value)")) } }
            "obj_omit" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Object(obj),Value::Array(keys)) => { let exclude: Vec<String> = keys.borrow().iter().filter_map(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None }).collect(); let map: HashMap<String,Value> = obj.borrow().iter().filter(|(k,_)| !exclude.contains(k)).map(|(k,v)| (k.clone(),v.clone())).collect(); Ok(Value::make_object(map)) } _ => Err(CustomLangError::type_err("omit() requires (object, array)")) } }
            "obj_pick" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Object(obj),Value::Array(keys)) => { let include: Vec<String> = keys.borrow().iter().filter_map(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None }).collect(); let map: HashMap<String,Value> = obj.borrow().iter().filter(|(k,_)| include.contains(k)).map(|(k,v)| (k.clone(),v.clone())).collect(); Ok(Value::make_object(map)) } _ => Err(CustomLangError::type_err("pick() requires (object, array)")) } }
            "obj_map_values" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Object(obj) => { let cb = args[1].clone(); let mut map = HashMap::new(); for (k,v) in obj.borrow().iter() { map.insert(k.clone(), self.call_value(cb.clone(), vec![v.clone()], None, pos)?); } Ok(Value::make_object(map)) } _ => Err(CustomLangError::type_err("map_values() requires object")) } }
            "obj_map_keys" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Object(obj) => { let cb = args[1].clone(); let mut map = HashMap::new(); for (k,v) in obj.borrow().iter() { let new_k = self.call_value(cb.clone(), vec![Value::Str(k.clone())], None, pos)?.to_string(); map.insert(new_k, v.clone()); } Ok(Value::make_object(map)) } _ => Err(CustomLangError::type_err("map_keys() requires object")) } }
            "obj_filter_values" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Object(obj) => { let cb = args[1].clone(); let mut map = HashMap::new(); for (k,v) in obj.borrow().iter() { if self.call_value(cb.clone(), vec![v.clone()], None, pos)?.is_truthy() { map.insert(k.clone(), v.clone()); } } Ok(Value::make_object(map)) } _ => Err(CustomLangError::type_err("filter_values() requires object")) } }
            "obj_invert" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(obj) => { let map: HashMap<String,Value> = obj.borrow().iter().map(|(k,v)| (v.to_string(), Value::Str(k.clone()))).collect(); Ok(Value::make_object(map)) } _ => Err(CustomLangError::type_err("invert() requires object")) } }
            "obj_from_entries" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Array(arr) => { let mut map = HashMap::new(); for item in arr.borrow().iter() { if let Value::Array(pair) = item { let p = pair.borrow(); if let (Some(Value::Str(k)), Some(v)) = (p.get(0), p.get(1)) { map.insert(k.clone(), v.clone()); } } } Ok(Value::make_object(map)) } _ => Err(CustomLangError::type_err("from_entries() requires array")) } }

            // ── FS ────────────────────────────────────────────────────────
            "fs_read_text" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => std::fs::read_to_string(p).map(Value::Str).map_err(|e| CustomLangError::io_err(format!("read: {e}"))), _ => Err(CustomLangError::type_err("read_text() requires string")) } }
            "fs_write_text" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(p),c) => { std::fs::write(p, c.to_string()).map_err(|e| CustomLangError::io_err(format!("write: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("write_text() requires (string, value)")) } }
            "fs_append_text" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(p),c) => { use std::io::Write as W; let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).map_err(|e| CustomLangError::io_err(format!("append: {e}")))?; write!(f, "{}", c).map_err(|e| CustomLangError::io_err(format!("append: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("append_text() requires (string, value)")) } }
            "fs_exists" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::Bool(std::path::Path::new(p).exists())), _ => Err(CustomLangError::type_err("exists() requires string")) } }
            "fs_delete" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { let path = std::path::Path::new(p); if path.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) }.map_err(|e| CustomLangError::io_err(format!("delete: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("delete() requires string")) } }
            "fs_rename" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(from),Value::Str(to)) => { std::fs::rename(from, to).map_err(|e| CustomLangError::io_err(format!("rename: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("rename() requires (string, string)")) } }
            "fs_copy" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(from),Value::Str(to)) => { std::fs::copy(from, to).map_err(|e| CustomLangError::io_err(format!("copy: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("copy() requires (string, string)")) } }
            "fs_mkdir" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { std::fs::create_dir(p).map_err(|e| CustomLangError::io_err(format!("mkdir: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("mkdir() requires string")) } }
            "fs_mkdir_all" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { std::fs::create_dir_all(p).map_err(|e| CustomLangError::io_err(format!("mkdir_all: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("mkdir_all() requires string")) } }
            "fs_rmdir" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { std::fs::remove_dir(p).map_err(|e| CustomLangError::io_err(format!("rmdir: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("rmdir() requires string")) } }
            "fs_list_dir" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { let entries: Vec<Value> = std::fs::read_dir(p).map_err(|e| CustomLangError::io_err(format!("list_dir: {e}")))?.filter_map(|e| e.ok()).map(|e| Value::Str(e.file_name().to_string_lossy().to_string())).collect(); Ok(Value::make_array(entries)) } _ => Err(CustomLangError::type_err("list_dir() requires string")) } }
            "fs_is_file" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::Bool(std::path::Path::new(p).is_file())), _ => Err(CustomLangError::type_err("is_file() requires string")) } }
            "fs_is_dir" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::Bool(std::path::Path::new(p).is_dir())), _ => Err(CustomLangError::type_err("is_dir() requires string")) } }
            "fs_file_size" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { let meta = std::fs::metadata(p).map_err(|e| CustomLangError::io_err(format!("file_size: {e}")))?; Ok(Value::Number(meta.len() as f64)) } _ => Err(CustomLangError::type_err("file_size() requires string")) } }
            "fs_last_modified" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { let meta = std::fs::metadata(p).map_err(|e| CustomLangError::io_err(format!("last_modified: {e}")))?; let t = meta.modified().map_err(|e| CustomLangError::io_err(format!("last_modified: {e}")))?; use std::time::UNIX_EPOCH; let ms = t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0); Ok(Value::Number(ms)) } _ => Err(CustomLangError::type_err("last_modified() requires string")) } }
            "fs_temp_file" => { let t = std::env::temp_dir().join(format!("cl_tmp_{}", lcg_rand(42) % 1000000)); Ok(Value::Str(t.to_string_lossy().to_string())) }
            "fs_temp_dir" => { Ok(Value::Str(std::env::temp_dir().to_string_lossy().to_string())) }

            // ── Path ──────────────────────────────────────────────────────
            "path_join" => { let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect(); let p = parts.iter().fold(std::path::PathBuf::new(), |mut acc, p| { acc.push(p); acc }); Ok(Value::Str(p.to_string_lossy().to_string())) }
            "path_dirname" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::Str(std::path::Path::new(p).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or(".".to_string()))), _ => Err(CustomLangError::type_err("dirname() requires string")) } }
            "path_basename" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::Str(std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())), _ => Err(CustomLangError::type_err("basename() requires string")) } }
            "path_stem" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::Str(std::path::Path::new(p).file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())), _ => Err(CustomLangError::type_err("stem() requires string")) } }
            "path_extension" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::Str(std::path::Path::new(p).extension().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())), _ => Err(CustomLangError::type_err("extension() requires string")) } }
            "path_absolute" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { let abs = std::fs::canonicalize(p).unwrap_or_else(|_| std::path::PathBuf::from(p)); Ok(Value::Str(abs.to_string_lossy().to_string())) } _ => Err(CustomLangError::type_err("absolute() requires string")) } }
            "path_normalize" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { let mut result = std::path::PathBuf::new(); for comp in std::path::Path::new(p).components() { match comp { std::path::Component::ParentDir => { result.pop(); } std::path::Component::CurDir => {} c => result.push(c) } } Ok(Value::Str(result.to_string_lossy().to_string())) } _ => Err(CustomLangError::type_err("normalize() requires string")) } }
            "path_split" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::make_array(std::path::Path::new(p).components().map(|c| Value::Str(c.as_os_str().to_string_lossy().to_string())).collect())), _ => Err(CustomLangError::type_err("split() requires string")) } }
            "path_is_absolute" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => Ok(Value::Bool(std::path::Path::new(p).is_absolute())), _ => Err(CustomLangError::type_err("is_absolute() requires string")) } }

            // ── Process ───────────────────────────────────────────────────
            "proc_args" => Ok(Value::make_array(std::env::args().map(Value::Str).collect())),
            "proc_env" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(k) => Ok(std::env::var(k).map(Value::Str).unwrap_or(Value::Null)), _ => Err(CustomLangError::type_err("env() requires string")) } }
            "proc_env_all" => { let mut map = HashMap::new(); for (k,v) in std::env::vars() { map.insert(k, Value::Str(v)); } Ok(Value::make_object(map)) }
            "proc_cwd" => Ok(Value::Str(std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default())),
            "proc_chdir" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(p) => { std::env::set_current_dir(p).map_err(|e| CustomLangError::io_err(format!("chdir: {e}")))?; Ok(Value::Bool(true)) } _ => Err(CustomLangError::type_err("chdir() requires string")) } }
            "proc_pid" => Ok(Value::Number(std::process::id() as f64)),
            "proc_platform" => Ok(Value::Str(if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "macos" } else { "linux" }.to_string())),
            "proc_run" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Str(cmd) => {
                        let output = if cfg!(windows) {
                            std::process::Command::new("cmd").args(["/C", cmd]).output()
                        } else {
                            std::process::Command::new("sh").args(["-c", cmd]).output()
                        }.map_err(|e| CustomLangError::io_err(format!("run: {e}")))?;
                        let mut map = HashMap::new();
                        map.insert("stdout".to_string(), Value::Str(String::from_utf8_lossy(&output.stdout).to_string()));
                        map.insert("stderr".to_string(), Value::Str(String::from_utf8_lossy(&output.stderr).to_string()));
                        map.insert("exit_code".to_string(), Value::Number(output.status.code().unwrap_or(0) as f64));
                        Ok(Value::make_object(map))
                    }
                    _ => Err(CustomLangError::type_err("run() requires string")),
                }
            }

            // ── HTTP ──────────────────────────────────────────────────────
            "http_get" => {
                if argc < 1 { return err_argc("1+"); }
                match &args[0] {
                    Value::Str(url) => {
                        match ureq::get(url).call() {
                            Ok(resp) => {
                                let status = resp.status() as f64;
                                let body = resp.into_string().unwrap_or_default();
                                let mut map = HashMap::new();
                                map.insert("status".to_string(), Value::Number(status));
                                map.insert("body".to_string(), Value::Str(body.clone()));
                                map.insert("ok".to_string(), Value::Bool(status < 400.0));
                                Ok(Value::make_object(map))
                            }
                            Err(e) => Err(CustomLangError::runtime(format!("http.get failed: {e}"))),
                        }
                    }
                    _ => Err(CustomLangError::type_err("http.get() requires string URL")),
                }
            }
            "http_post" => {
                if argc < 2 { return err_argc("2+"); }
                match (&args[0], &args[1]) {
                    (Value::Str(url), body) => {
                        let body_str = body.to_string();
                        match ureq::post(url).send_string(&body_str) {
                            Ok(resp) => {
                                let status = resp.status() as f64;
                                let rbody = resp.into_string().unwrap_or_default();
                                let mut map = HashMap::new();
                                map.insert("status".to_string(), Value::Number(status));
                                map.insert("body".to_string(), Value::Str(rbody));
                                map.insert("ok".to_string(), Value::Bool(status < 400.0));
                                Ok(Value::make_object(map))
                            }
                            Err(e) => Err(CustomLangError::runtime(format!("http.post failed: {e}"))),
                        }
                    }
                    _ => Err(CustomLangError::type_err("http.post() requires (string, value)")),
                }
            }
            "http_put" | "http_delete" | "http_patch" => {
                if argc < 1 { return err_argc("1+"); }
                match &args[0] {
                    Value::Str(url) => {
                        let method = name.trim_start_matches("http_").to_uppercase();
                        match ureq::request(&method, url).call() {
                            Ok(resp) => {
                                let status = resp.status() as f64;
                                let body = resp.into_string().unwrap_or_default();
                                let mut map = HashMap::new();
                                map.insert("status".to_string(), Value::Number(status));
                                map.insert("body".to_string(), Value::Str(body));
                                map.insert("ok".to_string(), Value::Bool(status < 400.0));
                                Ok(Value::make_object(map))
                            }
                            Err(e) => Err(CustomLangError::runtime(format!("http.{method} failed: {e}"))),
                        }
                    }
                    _ => Err(CustomLangError::type_err("http method requires string URL")),
                }
            }

            // ── Encoding ──────────────────────────────────────────────────
            "enc_url_encode" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(url_encode(s))), _ => Err(CustomLangError::type_err("url_encode() requires string")) } }
            "enc_url_decode" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(url_decode(s))), _ => Err(CustomLangError::type_err("url_decode() requires string")) } }
            "enc_html_encode" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace('\'',"&#x27;"))), _ => Err(CustomLangError::type_err("html_encode() requires string")) } }
            "enc_html_decode" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => Ok(Value::Str(s.replace("&amp;","&").replace("&lt;","<").replace("&gt;",">").replace("&quot;","\"").replace("&#x27;","'"))), _ => Err(CustomLangError::type_err("html_decode() requires string")) } }

            // ── Crypto ────────────────────────────────────────────────────
            "crypto_sha256" => { if argc != 1 { return err_argc("1"); } let s = args[0].to_string(); Ok(Value::Str(sha256_hex(s.as_bytes()))) }
            "crypto_sha512" => { if argc != 1 { return err_argc("1"); } let s = args[0].to_string(); Ok(Value::Str(sha512_hex(s.as_bytes()))) }
            "crypto_md5" => { if argc != 1 { return err_argc("1"); } let s = args[0].to_string(); Ok(Value::Str(md5_hex(s.as_bytes()))) }
            "crypto_hmac_sha256" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(key),Value::Str(msg)) => Ok(Value::Str(hmac_sha256_hex(key.as_bytes(), msg.as_bytes()))), _ => Err(CustomLangError::type_err("hmac_sha256() requires (string, string)")) } }
            "crypto_base64_encode" => { if argc != 1 { return err_argc("1"); } Ok(Value::Str(base64_encode(args[0].to_string().as_bytes()))) }
            "crypto_base64_decode" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => base64_decode(s).map(|b| Value::Str(String::from_utf8_lossy(&b).to_string())).map_err(|e| CustomLangError::runtime(e)), _ => Err(CustomLangError::type_err("base64_decode() requires string")) } }
            "crypto_hex_encode" => { if argc != 1 { return err_argc("1"); } Ok(Value::Str(hex_encode(args[0].to_string().as_bytes()))) }
            "crypto_hex_decode" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Str(s) => hex_decode(s).map(|b| Value::Str(String::from_utf8_lossy(&b).to_string())).map_err(|e| CustomLangError::runtime(e)), _ => Err(CustomLangError::type_err("hex_decode() requires string")) } }
            "crypto_random_bytes" => { if argc != 1 { return err_argc("1"); } match args[0] { Value::Number(n) => { let bytes: Vec<u8> = (0..n as usize).map(|_| lcg_rand(42) as u8).collect(); Ok(Value::Str(hex_encode(&bytes))) } _ => Err(CustomLangError::type_err("random_bytes() requires number")) } }
            "crypto_compare_secure" => { if argc != 2 { return err_argc("2"); } match (&args[0],&args[1]) { (Value::Str(a),Value::Str(b)) => { let same = a.len() == b.len() && a.bytes().zip(b.bytes()).all(|(x,y)| x == y); Ok(Value::Bool(same)) } _ => Err(CustomLangError::type_err("compare_secure() requires (string, string)")) } }

            // ── Regex ─────────────────────────────────────────────────────
            "regex_new" => { if argc < 1 { return err_argc("1+"); } match &args[0] { Value::Str(pattern) => { // Store pattern as object
                let mut map = HashMap::new();
                map.insert("pattern".to_string(), Value::Str(pattern.clone()));
                map.insert("test".to_string(), Value::Builtin("regex_test_method".to_string()));
                Ok(Value::make_object(map))
            } _ => Err(CustomLangError::type_err("regex.new() requires string pattern")) } }
            "regex_test" | "regex_test_method" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Str(pattern), Value::Str(text)) | (Value::Str(text), Value::Str(pattern)) => {
                        let re = regex::Regex::new(pattern).map_err(|e| CustomLangError::runtime(format!("regex error: {e}")))?;
                        Ok(Value::Bool(re.is_match(text)))
                    }
                    _ => Err(CustomLangError::type_err("regex_test() requires (string, string)")),
                }
            }
            "regex_match" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Str(pattern), Value::Str(text)) => {
                        let re = regex::Regex::new(pattern).map_err(|e| CustomLangError::runtime(format!("regex error: {e}")))?;
                        Ok(re.find(text).map(|m| Value::Str(m.as_str().to_string())).unwrap_or(Value::Null))
                    }
                    _ => Err(CustomLangError::type_err("regex_match() requires (string, string)")),
                }
            }
            "regex_match_all" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Str(pattern), Value::Str(text)) => {
                        let re = regex::Regex::new(pattern).map_err(|e| CustomLangError::runtime(format!("regex error: {e}")))?;
                        let matches: Vec<Value> = re.find_iter(text).map(|m| Value::Str(m.as_str().to_string())).collect();
                        Ok(Value::make_array(matches))
                    }
                    _ => Err(CustomLangError::type_err("regex_match_all() requires (string, string)")),
                }
            }
            "regex_replace" => {
                if argc != 3 { return err_argc("3"); }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Str(text), Value::Str(pattern), Value::Str(replacement)) => {
                        let re = regex::Regex::new(pattern).map_err(|e| CustomLangError::runtime(format!("regex error: {e}")))?;
                        Ok(Value::Str(re.replace(text, replacement.as_str()).to_string()))
                    }
                    _ => Err(CustomLangError::type_err("regex_replace() requires (string, string, string)")),
                }
            }
            "regex_replace_all" => {
                if argc != 3 { return err_argc("3"); }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Str(text), Value::Str(pattern), Value::Str(replacement)) => {
                        let re = regex::Regex::new(pattern).map_err(|e| CustomLangError::runtime(format!("regex error: {e}")))?;
                        Ok(Value::Str(re.replace_all(text, replacement.as_str()).to_string()))
                    }
                    _ => Err(CustomLangError::type_err("regex_replace_all() requires (string, string, string)")),
                }
            }

            // ── DateTime ──────────────────────────────────────────────────
            "dt_now" => {
                let now = chrono::Local::now();
                let mut map = HashMap::new();
                map.insert("year".to_string(), Value::Number(now.year() as f64));
                map.insert("month".to_string(), Value::Number(now.month() as f64));
                map.insert("day".to_string(), Value::Number(now.day() as f64));
                map.insert("hour".to_string(), Value::Number(now.hour() as f64));
                map.insert("minute".to_string(), Value::Number(now.minute() as f64));
                map.insert("second".to_string(), Value::Number(now.second() as f64));
                map.insert("timestamp".to_string(), Value::Number(now.timestamp_millis() as f64));
                map.insert("day_of_week".to_string(), Value::Number(now.weekday().num_days_from_sunday() as f64));
                Ok(Value::make_object(map))
            }
            "dt_from_timestamp" => {
                if argc != 1 { return err_argc("1"); }
                match args[0] {
                    Value::Number(ts) => {
                        use chrono::TimeZone;
                        let dt = chrono::Local.timestamp_millis_opt(ts as i64).earliest().unwrap_or_default();
                        let mut map = HashMap::new();
                        map.insert("year".to_string(), Value::Number(dt.year() as f64));
                        map.insert("month".to_string(), Value::Number(dt.month() as f64));
                        map.insert("day".to_string(), Value::Number(dt.day() as f64));
                        map.insert("hour".to_string(), Value::Number(dt.hour() as f64));
                        map.insert("minute".to_string(), Value::Number(dt.minute() as f64));
                        map.insert("second".to_string(), Value::Number(dt.second() as f64));
                        map.insert("timestamp".to_string(), Value::Number(ts));
                        Ok(Value::make_object(map))
                    }
                    _ => Err(CustomLangError::type_err("from_timestamp() requires number")),
                }
            }
            "dt_new" => {
                if argc < 3 { return err_argc("3+"); }
                match (&args[0], &args[1], &args[2]) {
                    (Value::Number(y), Value::Number(mo), Value::Number(d)) => {
                        use chrono::{TimeZone, Datelike};
                        let h = if argc > 3 { match args[3] { Value::Number(n) => n as u32, _ => 0 } } else { 0 };
                        let mi = if argc > 4 { match args[4] { Value::Number(n) => n as u32, _ => 0 } } else { 0 };
                        let s = if argc > 5 { match args[5] { Value::Number(n) => n as u32, _ => 0 } } else { 0 };
                        let dt = chrono::Local.with_ymd_and_hms(*y as i32, *mo as u32, *d as u32, h, mi, s).earliest().unwrap_or_default();
                        let mut map = HashMap::new();
                        map.insert("year".to_string(), Value::Number(*y));
                        map.insert("month".to_string(), Value::Number(*mo));
                        map.insert("day".to_string(), Value::Number(*d));
                        map.insert("hour".to_string(), Value::Number(h as f64));
                        map.insert("minute".to_string(), Value::Number(mi as f64));
                        map.insert("second".to_string(), Value::Number(s as f64));
                        map.insert("timestamp".to_string(), Value::Number(dt.timestamp_millis() as f64));
                        Ok(Value::make_object(map))
                    }
                    _ => Err(CustomLangError::type_err("dt_new() requires numbers")),
                }
            }
            "dt_format" | "dt_parse" => Ok(Value::Str(format!("{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")))),

            // ── Collections ───────────────────────────────────────────────
            "coll_queue" => {
                let mut map = HashMap::new();
                let data = Value::make_array(vec![]);
                map.insert("_data".to_string(), data.clone());
                map.insert("enqueue".to_string(), Value::Builtin("coll_queue_enqueue".to_string()));
                map.insert("dequeue".to_string(), Value::Builtin("coll_queue_dequeue".to_string()));
                map.insert("peek".to_string(), Value::Builtin("coll_queue_peek".to_string()));
                map.insert("is_empty".to_string(), Value::Builtin("coll_is_empty".to_string()));
                map.insert("size".to_string(), Value::Number(0.0));
                Ok(Value::make_object(map))
            }
            "coll_stack" => {
                let mut map = HashMap::new();
                map.insert("_data".to_string(), Value::make_array(vec![]));
                map.insert("push".to_string(), Value::Builtin("coll_stack_push".to_string()));
                map.insert("pop".to_string(), Value::Builtin("coll_stack_pop".to_string()));
                map.insert("peek".to_string(), Value::Builtin("coll_stack_peek".to_string()));
                map.insert("is_empty".to_string(), Value::Builtin("coll_is_empty".to_string()));
                Ok(Value::make_object(map))
            }
            "coll_deque" => {
                let mut map = HashMap::new();
                map.insert("_data".to_string(), Value::make_array(vec![]));
                map.insert("push_front".to_string(), Value::Builtin("coll_deque_push_front".to_string()));
                map.insert("push_back".to_string(), Value::Builtin("coll_deque_push_back".to_string()));
                map.insert("pop_front".to_string(), Value::Builtin("coll_deque_pop_front".to_string()));
                map.insert("pop_back".to_string(), Value::Builtin("coll_deque_pop_back".to_string()));
                map.insert("is_empty".to_string(), Value::Builtin("coll_is_empty".to_string()));
                Ok(Value::make_object(map))
            }
            "coll_linked_list" => {
                let mut map = HashMap::new();
                map.insert("_data".to_string(), Value::make_array(vec![]));
                map.insert("prepend".to_string(), Value::Builtin("coll_deque_push_front".to_string()));
                map.insert("append".to_string(), Value::Builtin("coll_deque_push_back".to_string()));
                map.insert("to_array".to_string(), Value::Builtin("coll_to_array".to_string()));
                Ok(Value::make_object(map))
            }
            "coll_is_empty" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(o) => { let d = o.borrow(); let data = d.get("_data"); Ok(Value::Bool(match data { Some(Value::Array(a)) => a.borrow().is_empty(), _ => true })) } _ => Err(CustomLangError::type_err("is_empty requires collection")) } }
            "coll_to_array" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(o) => { let d = o.borrow(); Ok(d.get("_data").cloned().unwrap_or(Value::make_array(vec![]))) } _ => Err(CustomLangError::type_err("to_array requires collection")) } }
            "coll_queue_enqueue" | "coll_stack_push" | "coll_deque_push_back" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] { Value::Object(o) => { if let Some(Value::Array(a)) = o.borrow().get("_data") { a.borrow_mut().push(args[1].clone()); } Ok(Value::Null) } _ => Err(CustomLangError::type_err("push requires collection")) }
            }
            "coll_deque_push_front" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] { Value::Object(o) => { if let Some(Value::Array(a)) = o.borrow().get("_data") { a.borrow_mut().insert(0, args[1].clone()); } Ok(Value::Null) } _ => Err(CustomLangError::type_err("push_front requires collection")) }
            }
            "coll_queue_dequeue" | "coll_deque_pop_front" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] { Value::Object(o) => { if let Some(Value::Array(a)) = o.borrow().get("_data") { let r = if a.borrow().is_empty() { Value::Null } else { a.borrow_mut().remove(0) }; Ok(r) } else { Ok(Value::Null) } } _ => Err(CustomLangError::type_err("dequeue requires collection")) }
            }
            "coll_stack_pop" | "coll_deque_pop_back" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] { Value::Object(o) => { if let Some(Value::Array(a)) = o.borrow().get("_data") { Ok(a.borrow_mut().pop().unwrap_or(Value::Null)) } else { Ok(Value::Null) } } _ => Err(CustomLangError::type_err("pop requires collection")) }
            }
            "coll_queue_peek" | "coll_stack_peek" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] { Value::Object(o) => { if let Some(Value::Array(a)) = o.borrow().get("_data") { Ok(a.borrow().last().cloned().unwrap_or(Value::Null)) } else { Ok(Value::Null) } } _ => Err(CustomLangError::type_err("peek requires collection")) }
            }

            // ── Testing ───────────────────────────────────────────────────
            "test_run" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Str(name) => {
                        let cb = args[1].clone();
                        print!("test '{}' ... ", name);
                        let _ = io::stdout().flush();
                        match self.call_value(cb, vec![], None, pos) {
                            Ok(_) => { println!("ok"); Ok(Value::Bool(true)) }
                            Err(e) => { println!("FAILED: {e}"); Ok(Value::Bool(false)) }
                        }
                    }
                    _ => Err(CustomLangError::type_err("test() requires (string, function)")),
                }
            }
            "test_describe" => {
                if argc != 2 { return err_argc("2"); }
                match &args[0] {
                    Value::Str(name) => {
                        println!("\n{name}");
                        let cb = args[1].clone();
                        self.call_value(cb, vec![], None, pos)?;
                        Ok(Value::Null)
                    }
                    _ => Err(CustomLangError::type_err("describe() requires (string, function)")),
                }
            }
            "test_assert_eq" => {
                if argc != 2 { return err_argc("2"); }
                if !args[0].equals(&args[1]) {
                    return Err(CustomLangError::runtime(format!("assert_eq failed: {} != {}", args[0].repr(), args[1].repr())));
                }
                Ok(Value::Null)
            }
            "test_assert_neq" => {
                if argc != 2 { return err_argc("2"); }
                if args[0].equals(&args[1]) {
                    return Err(CustomLangError::runtime(format!("assert_neq failed: {} == {}", args[0].repr(), args[1].repr())));
                }
                Ok(Value::Null)
            }
            "test_assert_throws" => {
                if argc != 1 { return err_argc("1"); }
                let cb = args[0].clone();
                match self.call_value(cb, vec![], None, pos) {
                    Err(_) => Ok(Value::Null),
                    Ok(_) => Err(CustomLangError::runtime("assert_throws: expected exception but none was thrown")),
                }
            }
            "test_assert_true" => { if argc != 1 { return err_argc("1"); } if !args[0].is_truthy() { return Err(CustomLangError::runtime(format!("assert_true failed: {}", args[0].repr()))); } Ok(Value::Null) }
            "test_assert_false" => { if argc != 1 { return err_argc("1"); } if args[0].is_truthy() { return Err(CustomLangError::runtime(format!("assert_false failed: {}", args[0].repr()))); } Ok(Value::Null) }
            "test_before_each" | "test_after_each" => Ok(Value::Null),

            // ── FP helpers ────────────────────────────────────────────────
            "partial" => {
                if argc < 2 { return err_argc("2+"); }
                let fn_val = args[0].clone();
                let bound_args: Vec<Value> = args[1..].to_vec();
                // Return a new function that prepends bound_args
                let fd = Rc::new(FnData {
                    name: "<partial>".to_string(),
                    params: vec![Param { name: "...rest".to_string(), default: None, is_rest: true }],
                    body: Box::new(Stmt::Return { value: None, pos: Position::default() }),
                    closure: Rc::clone(&self.env),
                    is_generator: false, is_async: false,
                });
                // We'll use a closure-based approach via an object
                let mut map = HashMap::new();
                map.insert("__fn__".to_string(), fn_val);
                map.insert("__args__".to_string(), Value::make_array(bound_args));
                map.insert("__partial__".to_string(), Value::Bool(true));
                let _ = fd;
                Ok(Value::make_object(map))
            }
            "memoize" => {
                if argc != 1 { return err_argc("1"); }
                let mut map = HashMap::new();
                map.insert("__fn__".to_string(), args[0].clone());
                map.insert("__cache__".to_string(), Value::make_object(HashMap::new()));
                map.insert("__memoized__".to_string(), Value::Bool(true));
                Ok(Value::make_object(map))
            }
            "compose" => {
                // compose(f, g, h)(x) = f(g(h(x)))
                let fns = args.clone();
                let mut map = HashMap::new();
                map.insert("__fns__".to_string(), Value::make_array(fns));
                map.insert("__compose__".to_string(), Value::Bool(true));
                Ok(Value::make_object(map))
            }
            "pipe_fn" => {
                // pipe(f, g, h)(x) = h(g(f(x)))
                let mut fns = args.clone(); fns.reverse();
                let mut map = HashMap::new();
                map.insert("__fns__".to_string(), Value::make_array(fns));
                map.insert("__compose__".to_string(), Value::Bool(true));
                Ok(Value::make_object(map))
            }
            "curry" => {
                if argc != 1 { return err_argc("1"); }
                // Simple curry implementation
                let mut map = HashMap::new();
                map.insert("__fn__".to_string(), args[0].clone());
                map.insert("__curry__".to_string(), Value::Bool(true));
                Ok(Value::make_object(map))
            }
            "update" => {
                if argc != 2 { return err_argc("2"); }
                match (&args[0], &args[1]) {
                    (Value::Object(orig), Value::Object(updates)) => {
                        let mut map = orig.borrow().clone();
                        for (k, v) in updates.borrow().iter() { map.insert(k.clone(), v.clone()); }
                        Ok(Value::make_object(map))
                    }
                    _ => Err(CustomLangError::type_err("update() requires (object, object)")),
                }
            }
            "set_at" => {
                if argc != 3 { return err_argc("3"); }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::Number(idx)) => {
                        let mut new_arr = arr.borrow().clone();
                        let i = *idx as usize;
                        if i < new_arr.len() { new_arr[i] = args[2].clone(); }
                        Ok(Value::make_array(new_arr))
                    }
                    _ => Err(CustomLangError::type_err("set_at() requires (array, number, value)")),
                }
            }
            "deep_freeze" => { Ok(args.into_iter().next().unwrap_or(Value::Null)) }

            // ── Generators ────────────────────────────────────────────────
            "gen_next" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Generator(g) => {
                        let mut gs = g.borrow_mut();
                        if gs.done || gs.index >= gs.values.len() {
                            gs.done = true;
                            let mut map = HashMap::new();
                            map.insert("value".to_string(), Value::Null);
                            map.insert("done".to_string(), Value::Bool(true));
                            Ok(Value::make_object(map))
                        } else {
                            let v = gs.values[gs.index].clone();
                            gs.index += 1;
                            let mut map = HashMap::new();
                            map.insert("value".to_string(), v);
                            map.insert("done".to_string(), Value::Bool(false));
                            Ok(Value::make_object(map))
                        }
                    }
                    _ => Err(CustomLangError::type_err("next() requires generator")),
                }
            }
            "gen_to_array" => {
                if argc != 1 { return err_argc("1"); }
                match &args[0] {
                    Value::Generator(g) => Ok(Value::make_array(g.borrow().values.clone())),
                    _ => Err(CustomLangError::type_err("to_array() requires generator")),
                }
            }

            // ── Result/Option types ───────────────────────────────────────
            "Ok" => { if argc != 1 { return err_argc("1"); } let mut m = HashMap::new(); m.insert("__ok__".to_string(), Value::Bool(true)); m.insert("value".to_string(), args[0].clone()); Ok(Value::make_object(m)) }
            "Err" => { if argc != 1 { return err_argc("1"); } let mut m = HashMap::new(); m.insert("__ok__".to_string(), Value::Bool(false)); m.insert("error".to_string(), args[0].clone()); Ok(Value::make_object(m)) }
            "Some" => { if argc != 1 { return err_argc("1"); } let mut m = HashMap::new(); m.insert("__some__".to_string(), Value::Bool(true)); m.insert("value".to_string(), args[0].clone()); Ok(Value::make_object(m)) }
            "None_val" => { let mut m = HashMap::new(); m.insert("__some__".to_string(), Value::Bool(false)); Ok(Value::make_object(m)) }
            "is_ok" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(o) => Ok(Value::Bool(matches!(o.borrow().get("__ok__"), Some(Value::Bool(true))))), _ => Ok(Value::Bool(false)) } }
            "is_err" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(o) => Ok(Value::Bool(matches!(o.borrow().get("__ok__"), Some(Value::Bool(false))))), _ => Ok(Value::Bool(false)) } }
            "unwrap" => { if argc != 1 { return err_argc("1"); } match &args[0] { Value::Object(o) => { let b = o.borrow(); if matches!(b.get("__ok__"), Some(Value::Bool(true))) { Ok(b.get("value").cloned().unwrap_or(Value::Null)) } else if let Some(v) = b.get("value").or(b.get("error")) { Err(CustomLangError::runtime(format!("unwrap on Err: {}", v))) } else { Err(CustomLangError::runtime("unwrap failed")) } } _ => Ok(args[0].clone()) } }
            "unwrap_or" => { if argc != 2 { return err_argc("2"); } match &args[0] { Value::Object(o) => { let b = o.borrow(); if matches!(b.get("__ok__"), Some(Value::Bool(true))) { Ok(b.get("value").cloned().unwrap_or(Value::Null)) } else { Ok(args[1].clone()) } } _ => Ok(args[0].clone()) } }

            // ── Promise (synchronous) ─────────────────────────────────────
            "Promise" => {
                if argc != 1 { return err_argc("1"); }
                let cb = args[0].clone();
                let mut resolved = Value::Null;
                let mut rejected = Value::Null;
                // Synchronously execute the promise
                let resolve_fn = Value::Builtin("__promise_resolve__".to_string());
                let reject_fn = Value::Builtin("__promise_reject__".to_string());
                Env::define(&self.env, "__promise_resolved__", Value::Null);
                Env::define(&self.env, "__promise_rejected__", Value::Null);
                let _ = self.call_value(cb, vec![resolve_fn, reject_fn], None, pos);
                resolved = Env::get(&self.env, "__promise_resolved__").unwrap_or(Value::Null);
                rejected = Env::get(&self.env, "__promise_rejected__").unwrap_or(Value::Null);
                let mut map = HashMap::new();
                map.insert("value".to_string(), resolved);
                map.insert("error".to_string(), rejected);
                map.insert("__promise__".to_string(), Value::Bool(true));
                Ok(Value::make_object(map))
            }
            "__promise_resolve__" => { if argc == 1 { Env::define(&self.env, "__promise_resolved__", args[0].clone()); } Ok(Value::Null) }
            "__promise_reject__" => { if argc == 1 { Env::define(&self.env, "__promise_rejected__", args[0].clone()); } Ok(Value::Null) }

            // ── Misc ──────────────────────────────────────────────────────
            "get_type" | "is_function" | "is_class" | "is_instance" => {
                if argc < 1 { return err_argc("1"); }
                match name {
                    "get_type" => Ok(Value::Str(args[0].type_name().to_string())),
                    "is_function" => Ok(Value::Bool(matches!(args[0], Value::Function(_) | Value::Builtin(_)))),
                    "is_class" => Ok(Value::Bool(matches!(args[0], Value::Class(_)))),
                    "is_instance" => Ok(Value::Bool(matches!(args[0], Value::Instance(_)))),
                    _ => Ok(Value::Null),
                }
            }
            "instanceof_check" => { if argc != 2 { return err_argc("2"); } let name_str = args[1].to_string(); Ok(Value::Bool(args[0].is_instance_of(&name_str))) }
            "weakmap_new" | "weakref_new" => {
                // WeakRef/WeakMap — just thin wrappers, no true GC
                let mut map = HashMap::new();
                map.insert("__weakref__".to_string(), Value::Bool(true));
                if name == "weakref_new" && argc == 1 { map.insert("target".to_string(), args[0].clone()); }
                Ok(Value::make_object(map))
            }
            "fmt" => {
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
                    _ => Err(CustomLangError::type_err("fmt() first argument must be string")),
                }
            }

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

fn deep_clone_value(v: &Value) -> Value {
    match v {
        Value::Array(arr) => Value::make_array(arr.borrow().iter().map(deep_clone_value).collect()),
        Value::Object(obj) => { let map: HashMap<String,Value> = obj.borrow().iter().map(|(k,v)| (k.clone(), deep_clone_value(v))).collect(); Value::make_object(map) }
        other => other.clone(),
    }
}

fn url_encode(s: &str) -> String {
    s.bytes().flat_map(|b| {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' { vec![b as char] }
        else { format!("%{:02X}", b).chars().collect() }
    }).collect()
}

fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let bytes = s.as_bytes(); let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i+1..i+3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) { result.push(byte as char); i += 3; continue; }
            }
        } else if bytes[i] == b'+' { result.push(' '); i += 1; continue; }
        result.push(bytes[i] as char); i += 1;
    }
    result
}

fn hex_encode(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{:02x}", b)).collect() }

fn hex_decode(s: &str) -> std::result::Result<Vec<u8>, String> {
    if s.len() % 2 != 0 { return Err("odd length hex string".to_string()); }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i+2).unwrap_or(""), 16).map_err(|e| e.to_string())).collect()
}

fn base64_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[(combined >> 18) & 63] as char);
        result.push(CHARS[(combined >> 12) & 63] as char);
        result.push(if chunk.len() > 1 { CHARS[(combined >> 6) & 63] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[combined & 63] as char } else { '=' });
    }
    result
}

fn base64_decode(s: &str) -> std::result::Result<Vec<u8>, String> {
    const DEC: [i8; 256] = { let mut t = [-1i8; 256]; let enc = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"; let mut i = 0; while i < 64 { t[enc[i] as usize] = i as i8; i += 1; } t };
    let mut result = Vec::new();
    let s: Vec<u8> = s.bytes().filter(|&b| b != b'=').collect();
    for chunk in s.chunks(4) {
        let vals: Vec<i8> = chunk.iter().map(|&b| DEC[b as usize]).collect();
        if vals.iter().any(|&v| v < 0) { return Err("invalid base64".to_string()); }
        result.push(((vals[0] << 2) | (vals.get(1).copied().unwrap_or(0) >> 4)) as u8);
        if vals.len() > 2 { result.push(((vals[1] << 4) | (vals[2] >> 2)) as u8); }
        if vals.len() > 3 { result.push(((vals[2] << 6) | vals[3]) as u8); }
    }
    Ok(result)
}

fn sha256_hex(data: &[u8]) -> String { hex_encode(&sha256_hash(data)) }
fn sha512_hex(data: &[u8]) -> String { let a = sha256_hash(data); let b = sha256_hash(&a); format!("{}{}", hex_encode(&a), hex_encode(&b)) }
fn md5_hex(data: &[u8]) -> String { hex_encode(&md5_hash(data)) }

fn sha256_hash(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
    let mut h = [0x6a09e667u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    let mut msg = data.to_vec(); let orig_len = data.len() as u64 * 8;
    msg.push(0x80); while msg.len() % 64 != 56 { msg.push(0); }
    for b in orig_len.to_be_bytes() { msg.push(b); }
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 { w[i] = u32::from_be_bytes([chunk[i*4],chunk[i*4+1],chunk[i*4+2],chunk[i*4+3]]); }
        for i in 16..64 { let s0=w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3); let s1=w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10); w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1); }
        let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh)=(h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
        for i in 0..64 { let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25); let ch=(e&f)^((!e)&g); let t1=hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]); let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22); let maj=(a&b)^(a&c)^(b&c); let t2=s0.wrapping_add(maj); hh=g;g=f;f=e;e=d.wrapping_add(t1);d=c;c=b;b=a;a=t1.wrapping_add(t2); }
        h[0]=h[0].wrapping_add(a);h[1]=h[1].wrapping_add(b);h[2]=h[2].wrapping_add(c);h[3]=h[3].wrapping_add(d);h[4]=h[4].wrapping_add(e);h[5]=h[5].wrapping_add(f);h[6]=h[6].wrapping_add(g);h[7]=h[7].wrapping_add(hh);
    }
    let mut out=[0u8;32]; for (i,&v) in h.iter().enumerate() { out[i*4..i*4+4].copy_from_slice(&v.to_be_bytes()); } out
}

fn md5_hash(data: &[u8]) -> [u8; 16] {
    const S: [u32;64]=[7,12,17,22,7,12,17,22,7,12,17,22,7,12,17,22,5,9,14,20,5,9,14,20,5,9,14,20,5,9,14,20,4,11,16,23,4,11,16,23,4,11,16,23,4,11,16,23,6,10,15,21,6,10,15,21,6,10,15,21,6,10,15,21];
    const K: [u32;64]=[0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391];
    let (mut a0,mut b0,mut c0,mut d0)=(0x67452301u32,0xefcdab89u32,0x98badcfeu32,0x10325476u32);
    let mut msg=data.to_vec(); let orig_len=(data.len() as u64).wrapping_mul(8);
    msg.push(0x80); while msg.len()%64!=56{msg.push(0);}
    for b in orig_len.to_le_bytes(){msg.push(b);}
    for chunk in msg.chunks(64) {
        let m: Vec<u32>=(0..16).map(|i|u32::from_le_bytes([chunk[i*4],chunk[i*4+1],chunk[i*4+2],chunk[i*4+3]])).collect();
        let (mut a,mut b,mut c,mut d)=(a0,b0,c0,d0);
        for i in 0u32..64 {
            let (f,g)=match i{0..=15=>((b&c)|(!b&d),i),16..=31=>((d&b)|(!d&c),(5*i+1)%16),32..=47=>(b^c^d,(3*i+5)%16),_=>(c^(b|!d),(7*i)%16)};
            let temp=d;d=c;c=b;b=b.wrapping_add((a.wrapping_add(f).wrapping_add(K[i as usize]).wrapping_add(m[g as usize])).rotate_left(S[i as usize]));a=temp;
        }
        a0=a0.wrapping_add(a);b0=b0.wrapping_add(b);c0=c0.wrapping_add(c);d0=d0.wrapping_add(d);
    }
    let mut out=[0u8;16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

fn hmac_sha256_hex(key: &[u8], msg: &[u8]) -> String {
    let block_size = 64;
    let mut k = if key.len() > block_size { sha256_hash(key).to_vec() } else { key.to_vec() };
    k.resize(block_size, 0);
    let ipad: Vec<u8> = k.iter().map(|b| b^0x36).collect();
    let opad: Vec<u8> = k.iter().map(|b| b^0x5c).collect();
    let inner: Vec<u8> = ipad.into_iter().chain(msg.iter().copied()).collect();
    let inner_hash = sha256_hash(&inner);
    let outer: Vec<u8> = opad.into_iter().chain(inner_hash.iter().copied()).collect();
    hex_encode(&sha256_hash(&outer))
}

use chrono::{Datelike, Timelike};
