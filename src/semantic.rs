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
            // New builtins
            "json_parse",
            "json_stringify",
            "json_is_valid",
            "random_float",
            "random_int",
            "random_bool",
            "random_choice",
            "random_shuffle",
            "math_clamp",
            "math_sign",
            "math_hypot",
            "math_gcd",
            "math_lcm",
            "math_factorial",
            "math_is_nan",
            "math_is_finite",
            "math_lerp",
            "math_degrees",
            "math_radians",
            "math_atan2",
            "math_asin",
            "math_acos",
            "math_atan",
            "math_exp",
            "math_log2",
            "math_log10",
            "math_cbrt",
            "math_trunc",
            "flat",
            "flat_map",
            "zip",
            "chunk",
            "unique",
            "count",
            "sum",
            "average",
            "repeat",
            "pad_start",
            "pad_end",
            "arr_unzip",
            "arr_group_by",
            "arr_partition",
            "arr_rotate",
            "arr_take",
            "arr_drop",
            "arr_take_while",
            "arr_drop_while",
            "arr_flatten_deep",
            "arr_fill",
            "arr_fill_with",
            "arr_min_by",
            "arr_max_by",
            "arr_sort_by",
            "arr_difference",
            "arr_intersection",
            "arr_union",
            "obj_merge",
            "obj_deep_clone",
            "obj_get_path",
            "obj_set_path",
            "obj_omit",
            "obj_pick",
            "obj_map_values",
            "obj_map_keys",
            "obj_filter_values",
            "obj_invert",
            "obj_from_entries",
            "str_at",
            "str_last_index_of",
            "str_char_codes",
            "str_from_char_codes",
            "str_is_digit",
            "str_is_alpha",
            "str_is_alnum",
            "str_is_whitespace",
            "str_is_upper",
            "str_is_lower",
            "str_count_occurrences",
            "str_reverse",
            "str_word_count",
            "str_lines",
            "fs_read_text",
            "fs_write_text",
            "fs_append_text",
            "fs_exists",
            "fs_delete",
            "fs_rename",
            "fs_copy",
            "fs_mkdir",
            "fs_mkdir_all",
            "fs_rmdir",
            "fs_list_dir",
            "fs_is_file",
            "fs_is_dir",
            "fs_file_size",
            "fs_last_modified",
            "fs_temp_file",
            "fs_temp_dir",
            "path_join",
            "path_dirname",
            "path_basename",
            "path_stem",
            "path_extension",
            "path_absolute",
            "path_normalize",
            "path_split",
            "path_is_absolute",
            "proc_args",
            "proc_env",
            "proc_env_all",
            "proc_cwd",
            "proc_chdir",
            "proc_pid",
            "proc_platform",
            "proc_run",
            "http_get",
            "http_post",
            "http_put",
            "http_delete",
            "http_patch",
            "enc_url_encode",
            "enc_url_decode",
            "enc_html_encode",
            "enc_html_decode",
            "crypto_sha256",
            "crypto_sha512",
            "crypto_md5",
            "crypto_hmac_sha256",
            "crypto_base64_encode",
            "crypto_base64_decode",
            "crypto_hex_encode",
            "crypto_hex_decode",
            "crypto_random_bytes",
            "crypto_compare_secure",
            "regex_new",
            "regex_test",
            "regex_match",
            "regex_match_all",
            "regex_replace",
            "regex_replace_all",
            "dt_now",
            "dt_from_timestamp",
            "dt_new",
            "dt_format",
            "dt_parse",
            "coll_queue",
            "coll_stack",
            "coll_deque",
            "coll_linked_list",
            "test_run",
            "test_describe",
            "test_assert_eq",
            "test_assert_neq",
            "test_assert_throws",
            "test_assert_true",
            "test_assert_false",
            "test_before_each",
            "test_after_each",
            "partial",
            "curry",
            "compose",
            "pipe_fn",
            "memoize",
            "update",
            "set_at",
            "deep_freeze",
            "weakmap_new",
            "weakref_new",
            "gen_next",
            "gen_to_array",
            "get_type",
            "is_function",
            "is_class",
            "is_instance",
            "instanceof_check",
            "Promise",
            "Ok",
            "Err",
            "Some",
            "None_val",
            "is_ok",
            "is_err",
            "unwrap",
            "unwrap_or",
            "fmt",
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

    /// Analyze and return lint hints (warnings) without failing
    pub fn analyze_with_hints(&mut self, program: &Program) -> Vec<String> {
        for stmt in &program.stmts {
            self.check_stmt(stmt);
        }
        self.errors
            .iter()
            .map(|e| format!("warning: {e}"))
            .collect()
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
                    self.define(&p.name, Type::Unknown);
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
            Stmt::DoWhile { body, cond, .. } => {
                let was_loop = self.in_loop;
                self.in_loop = true;
                self.check_stmt(body);
                self.in_loop = was_loop;
                self.check_expr(cond);
            }
            Stmt::TryCatch {
                try_b,
                catch_var,
                catch_b,
                finally_b,
                ..
            } => {
                self.check_stmt(try_b);
                if let Some(cb) = catch_b {
                    self.push_scope();
                    if let Some(var) = catch_var {
                        self.define(var, Type::Unknown);
                    }
                    self.check_stmt(cb);
                    self.pop_scope();
                }
                if let Some(fb) = finally_b {
                    self.check_stmt(fb);
                }
            }
            Stmt::Throw { value, .. } => {
                self.check_expr(value);
            }
            Stmt::LetDestructArray { elems, init, .. } => {
                self.check_expr(init);
                for elem in elems {
                    match elem {
                        DestructElem::Bind { name, default } => {
                            self.define(name, Type::Unknown);
                            if let Some(d) = default {
                                self.check_expr(d);
                            }
                        }
                        DestructElem::Rest(name) => {
                            self.define(name, Type::Unknown);
                        }
                        DestructElem::Skip => {}
                    }
                }
            }
            Stmt::LetDestructObject { fields, init, .. } => {
                self.check_expr(init);
                for f in fields {
                    let name = f.alias.as_ref().unwrap_or(&f.key);
                    self.define(name, Type::Unknown);
                    if let Some(d) = &f.default {
                        self.check_expr(d);
                    }
                }
            }
            Stmt::ForOf {
                var, iter, body, ..
            } => {
                self.check_expr(iter);
                self.push_scope();
                self.define(var, Type::Unknown);
                let was = self.in_loop;
                self.in_loop = true;
                self.check_stmt(body);
                self.in_loop = was;
                self.pop_scope();
            }
            Stmt::Labeled { body, .. } => {
                self.check_stmt(body);
            }
            Stmt::Enum { name, variants, .. } => {
                self.define(name, Type::Unknown);
                for (_, v) in variants {
                    if let Some(e) = v {
                        self.check_expr(e);
                    }
                }
            }
            Stmt::TypeAlias { name, .. } => {
                self.define(name, Type::Unknown);
            }
            Stmt::Interface { name, .. } => {
                self.define(name, Type::Unknown);
            }
            Stmt::Decorator { target, .. } => {
                self.check_stmt(target);
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
                    self.define(&p.name, Type::Unknown);
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
                    if let Some(guard) = &arm.guard {
                        self.check_expr(guard);
                    }
                    self.check_expr(&arm.body);
                    self.pop_scope();
                }
                Type::Unknown
            }
            Expr::Ternary {
                cond,
                then_e,
                else_e,
                ..
            } => {
                self.check_expr(cond);
                self.check_expr(then_e);
                self.check_expr(else_e);
                Type::Unknown
            }
            Expr::OptionalProp { object, .. } => {
                self.check_expr(object);
                Type::Unknown
            }
            Expr::OptionalIndex { object, index, .. } => {
                self.check_expr(object);
                self.check_expr(index);
                Type::Unknown
            }
            Expr::OptionalCall { callee, args, .. } => {
                self.check_expr(callee);
                for a in args {
                    self.check_expr(a);
                }
                Type::Unknown
            }
            Expr::Spread { expr, .. } => {
                self.check_expr(expr);
                Type::Unknown
            }
            Expr::Super { .. } => Type::Unknown,
            Expr::Yield { value, .. } => {
                if let Some(e) = value {
                    self.check_expr(e);
                }
                Type::Unknown
            }
            Expr::Await { expr, .. } => {
                self.check_expr(expr);
                Type::Unknown
            }
            Expr::Pipe { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
                Type::Unknown
            }
        }
    }

    fn define_pattern_bindings(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Binding(name) => {
                self.define(name, Type::Unknown);
            }
            Pattern::Array(pats) => {
                for p in pats {
                    self.define_pattern_bindings(p);
                }
            }
            Pattern::Object(pairs) => {
                for (_, p) in pairs {
                    self.define_pattern_bindings(p);
                }
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
