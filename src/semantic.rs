use crate::ast::*;
use crate::error::{CustomLangError, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Number,
    Str,
    Boolean,
    Null,
    Function,
    Unknown,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Number => write!(f, "number"),
            Type::Str => write!(f, "string"),
            Type::Boolean => write!(f, "boolean"),
            Type::Null => write!(f, "null"),
            Type::Function => write!(f, "function"),
            Type::Unknown => write!(f, "unknown"),
        }
    }
}

struct Scope {
    vars: HashMap<String, Type>,
}

impl Scope {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}

pub struct SemanticAnalyzer {
    scopes: Vec<Scope>,
    in_function: bool,
    in_loop: bool,
    /// Set to true when a flat import is encountered — we can't know what names it exports
    has_wild_import: bool,
    errors: Vec<CustomLangError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            scopes: vec![Scope::new()],
            in_function: false,
            in_loop: false,
            has_wild_import: false,
            errors: Vec::new(),
        };
        analyzer.register_builtins();
        analyzer
    }

    fn register_builtins(&mut self) {
        let builtins = [
            "print",
            "println",
            "input",
            "len",
            "type",
            "range",
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
            "keys",
            "values",
            "entries",
            "has_key",
            "delete_key",
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
            "parse_int",
            "parse_float",
            "is_number",
            "is_string",
            "is_bool",
            "is_null",
            "is_array",
            "is_object",
            "read_file",
            "write_file",
            "append_file",
            "assert",
            "exit",
            "now",
        ];
        let scope = self.scopes.last_mut().expect("always a scope");
        for name in &builtins {
            scope.vars.insert(name.to_string(), Type::Function);
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<()> {
        for stmt in &program.stmts {
            self.check_stmt(stmt);
        }
        if let Some(first_err) = self.errors.first() {
            return Err(first_err.clone());
        }
        Ok(())
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.vars.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.vars.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn all_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for scope in &self.scopes {
            names.extend(scope.vars.keys().cloned());
        }
        names
    }

    fn error_undef(&mut self, name: &str) {
        if self.has_wild_import {
            return; // name may come from an import we can't statically analyze
        }
        let names = self.all_names();
        let suggestion = CustomLangError::find_similar(name, &names);
        self.errors
            .push(CustomLangError::undef_var(name, suggestion));
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, init, .. } => {
                let ty = if let Some(expr) = init {
                    self.check_expr(expr)
                } else {
                    Type::Null
                };
                self.define(name, ty);
            }
            Stmt::Function {
                name, params, body, ..
            } => {
                self.define(name, Type::Function);
                self.push_scope();
                for p in params {
                    self.define(p, Type::Unknown);
                }
                let was_fn = self.in_function;
                self.in_function = true;
                self.check_stmt(body);
                self.in_function = was_fn;
                self.pop_scope();
            }
            Stmt::Class {
                name,
                super_name,
                methods,
                ..
            } => {
                if let Some(sn) = super_name {
                    if self.lookup(sn).is_none() {
                        self.error_undef(sn);
                    }
                }
                self.define(name, Type::Unknown);
                self.push_scope();
                self.define("this", Type::Unknown);
                for m in methods {
                    self.check_stmt(m);
                }
                self.pop_scope();
            }
            Stmt::If {
                cond,
                then_b,
                else_b,
                ..
            } => {
                self.check_expr(cond);
                self.push_scope();
                self.check_stmt(then_b);
                self.pop_scope();
                if let Some(eb) = else_b {
                    self.push_scope();
                    self.check_stmt(eb);
                    self.pop_scope();
                }
            }
            Stmt::While { cond, body, .. } => {
                self.check_expr(cond);
                self.push_scope();
                let was_loop = self.in_loop;
                self.in_loop = true;
                self.check_stmt(body);
                self.in_loop = was_loop;
                self.pop_scope();
            }
            Stmt::For {
                init,
                cond,
                update,
                body,
                ..
            } => {
                self.push_scope();
                if let Some(s) = init {
                    self.check_stmt(s);
                }
                if let Some(e) = cond {
                    self.check_expr(e);
                }
                if let Some(e) = update {
                    self.check_expr(e);
                }
                let was_loop = self.in_loop;
                self.in_loop = true;
                self.check_stmt(body);
                self.in_loop = was_loop;
                self.pop_scope();
            }
            Stmt::ForIn {
                var, iter, body, ..
            } => {
                self.check_expr(iter);
                self.push_scope();
                self.define(var, Type::Unknown);
                let was_loop = self.in_loop;
                self.in_loop = true;
                self.check_stmt(body);
                self.in_loop = was_loop;
                self.pop_scope();
            }
            Stmt::Block { stmts, .. } => {
                self.push_scope();
                for s in stmts {
                    self.check_stmt(s);
                }
                self.pop_scope();
            }
            Stmt::Return { value, .. } => {
                if !self.in_function {
                    self.errors.push(CustomLangError::semantic(
                        "'return' used outside of a function",
                    ));
                }
                if let Some(e) = value {
                    self.check_expr(e);
                }
            }
            Stmt::Break { .. } => {
                if !self.in_loop {
                    self.errors
                        .push(CustomLangError::semantic("'break' used outside of a loop"));
                }
            }
            Stmt::Continue { .. } => {
                if !self.in_loop {
                    self.errors.push(CustomLangError::semantic(
                        "'continue' used outside of a loop",
                    ));
                }
            }
            Stmt::Expr { expr, .. } => {
                self.check_expr(expr);
            }
            Stmt::Print { expr, .. } => {
                self.check_expr(expr);
            }
            Stmt::Import { alias, .. } => {
                if let Some(alias) = alias {
                    self.define(alias, Type::Unknown);
                } else {
                    self.has_wild_import = true; // flat import: can't know exported names statically
                }
            }
            Stmt::Export { name, .. } => {
                if self.lookup(name).is_none() {
                    self.error_undef(name);
                }
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal { value, .. } => match value {
                Value::Number(_) => Type::Number,
                Value::Str(_) => Type::Str,
                Value::Bool(_) => Type::Boolean,
                Value::Null => Type::Null,
                Value::Function(_) | Value::Builtin(_) => Type::Function,
                _ => Type::Unknown,
            },
            Expr::Var { name, .. } => match self.lookup(name) {
                Some(ty) => ty,
                None => {
                    self.error_undef(name);
                    Type::Unknown
                }
            },
            Expr::Assign { name, value, .. } => {
                let ty = self.check_expr(value);
                if self.lookup(name).is_none() {
                    self.error_undef(name);
                } else {
                    self.define(name, ty.clone());
                }
                ty
            }
            Expr::CompoundAssign { name, value, .. } => {
                self.check_expr(value);
                if self.lookup(name).is_none() {
                    self.error_undef(name);
                }
                Type::Unknown
            }
            Expr::IndexAssign {
                object,
                index,
                value,
                ..
            } => {
                self.check_expr(object);
                self.check_expr(index);
                self.check_expr(value)
            }
            Expr::PropAssign { object, value, .. } => {
                self.check_expr(object);
                self.check_expr(value)
            }
            Expr::Binary { left, right, .. } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                if lt == Type::Number && rt == Type::Number {
                    Type::Number
                } else if lt == Type::Str || rt == Type::Str {
                    Type::Str
                } else {
                    Type::Unknown
                }
            }
            Expr::Unary { expr, .. } => self.check_expr(expr),
            Expr::Call { callee, args, .. } => {
                self.check_expr(callee);
                for a in args {
                    self.check_expr(a);
                }
                Type::Unknown
            }
            Expr::Index { object, index, .. } => {
                self.check_expr(object);
                self.check_expr(index);
                Type::Unknown
            }
            Expr::Prop { object, .. } => {
                self.check_expr(object);
                Type::Unknown
            }
            Expr::Array { elements, .. } => {
                for e in elements {
                    self.check_expr(e);
                }
                Type::Unknown
            }
            Expr::Object { pairs, .. } => {
                for (_, v) in pairs {
                    self.check_expr(v);
                }
                Type::Unknown
            }
            Expr::New { args, .. } => {
                for a in args {
                    self.check_expr(a);
                }
                Type::Unknown
            }
            Expr::This { .. } => Type::Unknown,
            Expr::Lambda { params, body, .. } => {
                self.push_scope();
                for p in params {
                    self.define(p, Type::Unknown);
                }
                let was_fn = self.in_function;
                self.in_function = true;
                self.check_stmt(body);
                self.in_function = was_fn;
                self.pop_scope();
                Type::Function
            }
            Expr::Match { expr, arms, .. } => {
                self.check_expr(expr);
                for arm in arms {
                    self.push_scope();
                    self.define_pattern_bindings(&arm.pattern);
                    self.check_expr(&arm.body);
                    self.pop_scope();
                }
                Type::Unknown
            }
        }
    }

    fn define_pattern_bindings(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Binding(name) => { self.define(name, Type::Unknown); }
            Pattern::Array(pats) => {
                for p in pats { self.define_pattern_bindings(p); }
            }
            Pattern::Object(pairs) => {
                for (_, p) in pairs { self.define_pattern_bindings(p); }
            }
            _ => {}
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
