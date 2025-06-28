//! # Interpreter (Execution Engine)
//!
//! The interpreter executes Custom Language programs by walking the AST and
//! performing the operations specified by each node. It manages:
//! - Variable and function environments with proper scoping
//! - Built-in function implementations
//! - Control flow execution (if/else, while loops, function calls)
//! - Runtime error detection and reporting
//! - Value conversions and type checking

use crate::ast::*;
use crate::error::{CustomLangError, Result};
use std::collections::HashMap;

/// Return value for early returns from functions
#[derive(Debug, Clone)]
pub enum ControlFlow {
    None,
    Return(Value),
}

/// The main interpreter that executes the AST
pub struct Interpreter {
    #[allow(dead_code)]
    globals: Environment,
    environment: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut globals = Environment::new();

        // Add built-in functions
        globals.define(
            "print".to_string(),
            Value::BuiltinFunction("print".to_string()),
        );
        globals.define("len".to_string(), Value::BuiltinFunction("len".to_string()));
        globals.define("abs".to_string(), Value::BuiltinFunction("abs".to_string()));
        globals.define(
            "sqrt".to_string(),
            Value::BuiltinFunction("sqrt".to_string()),
        );
        globals.define("pow".to_string(), Value::BuiltinFunction("pow".to_string()));
        globals.define("min".to_string(), Value::BuiltinFunction("min".to_string()));
        globals.define("max".to_string(), Value::BuiltinFunction("max".to_string()));
        globals.define(
            "input".to_string(),
            Value::BuiltinFunction("input".to_string()),
        );
        globals.define(
            "type".to_string(),
            Value::BuiltinFunction("type".to_string()),
        );

        // Array functions
        globals.define(
            "push".to_string(),
            Value::BuiltinFunction("push".to_string()),
        );
        globals.define("pop".to_string(), Value::BuiltinFunction("pop".to_string()));
        globals.define(
            "first".to_string(),
            Value::BuiltinFunction("first".to_string()),
        );
        globals.define(
            "last".to_string(),
            Value::BuiltinFunction("last".to_string()),
        );
        globals.define(
            "sort".to_string(),
            Value::BuiltinFunction("sort".to_string()),
        );
        globals.define(
            "reverse".to_string(),
            Value::BuiltinFunction("reverse".to_string()),
        );
        globals.define(
            "filter".to_string(),
            Value::BuiltinFunction("filter".to_string()),
        );
        globals.define("map".to_string(), Value::BuiltinFunction("map".to_string()));
        globals.define(
            "reduce".to_string(),
            Value::BuiltinFunction("reduce".to_string()),
        );
        globals.define(
            "find".to_string(),
            Value::BuiltinFunction("find".to_string()),
        );
        globals.define(
            "includes".to_string(),
            Value::BuiltinFunction("includes".to_string()),
        );

        // File I/O functions
        globals.define(
            "read_file".to_string(),
            Value::BuiltinFunction("read_file".to_string()),
        );
        globals.define(
            "write_file".to_string(),
            Value::BuiltinFunction("write_file".to_string()),
        );

        // String manipulation functions
        globals.define(
            "split".to_string(),
            Value::BuiltinFunction("split".to_string()),
        );
        globals.define(
            "join".to_string(),
            Value::BuiltinFunction("join".to_string()),
        );
        globals.define(
            "substring".to_string(),
            Value::BuiltinFunction("substring".to_string()),
        );
        globals.define(
            "to_upper".to_string(),
            Value::BuiltinFunction("to_upper".to_string()),
        );
        globals.define(
            "to_lower".to_string(),
            Value::BuiltinFunction("to_lower".to_string()),
        );
        globals.define(
            "trim".to_string(),
            Value::BuiltinFunction("trim".to_string()),
        );
        globals.define(
            "starts_with".to_string(),
            Value::BuiltinFunction("starts_with".to_string()),
        );
        globals.define(
            "ends_with".to_string(),
            Value::BuiltinFunction("ends_with".to_string()),
        );
        globals.define(
            "contains".to_string(),
            Value::BuiltinFunction("contains".to_string()),
        );
        globals.define(
            "replace".to_string(),
            Value::BuiltinFunction("replace".to_string()),
        );

        Self {
            globals: globals.clone(),
            environment: globals,
        }
    }

    pub fn interpret(&mut self, program: &Program) -> Result<()> {
        for statement in &program.statements {
            match self.execute_stmt(statement)? {
                ControlFlow::Return(_) => {
                    return Err(CustomLangError::runtime_error(
                        "Cannot return from top-level code",
                    ));
                }
                ControlFlow::None => {}
            }
        }
        Ok(())
    }

    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<ControlFlow> {
        match stmt {
            Stmt::Expression { expr, .. } => {
                self.evaluate_expr(expr)?;
                Ok(ControlFlow::None)
            }
            Stmt::VarDeclaration {
                name, initializer, ..
            } => {
                let value = if let Some(init) = initializer {
                    self.evaluate_expr(init)?
                } else {
                    Value::Null
                };
                self.environment.define(name.clone(), value);
                Ok(ControlFlow::None)
            }
            Stmt::Block { statements, .. } => self.execute_block(statements),
            Stmt::If {
                condition,
                then_stmt,
                else_stmt,
                ..
            } => {
                let condition_value = self.evaluate_expr(condition)?;
                if condition_value.is_truthy() {
                    self.execute_stmt(then_stmt)
                } else if let Some(else_stmt) = else_stmt {
                    self.execute_stmt(else_stmt)
                } else {
                    Ok(ControlFlow::None)
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                while self.evaluate_expr(condition)?.is_truthy() {
                    match self.execute_stmt(body)? {
                        ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }
            Stmt::Function {
                name, params, body, ..
            } => {
                // Store user-defined functions
                let function_value = Value::Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: self.environment.clone(),
                };
                self.environment.define(name.clone(), function_value);
                Ok(ControlFlow::None)
            }
            Stmt::Return { value, .. } => {
                let return_value = if let Some(expr) = value {
                    self.evaluate_expr(expr)?
                } else {
                    Value::Null
                };
                Ok(ControlFlow::Return(return_value))
            }
            Stmt::Print { expr, .. } => {
                let value = self.evaluate_expr(expr)?;
                println!("{}", self.value_to_string(&value));
                Ok(ControlFlow::None)
            }
            Stmt::Import {
                module_path, alias, ..
            } => {
                self.handle_import(module_path, alias.as_deref())?;
                Ok(ControlFlow::None)
            }
            Stmt::Export { name, .. } => {
                self.handle_export(name)?;
                Ok(ControlFlow::None)
            }
            Stmt::Class {
                name,
                superclass,
                methods,
                ..
            } => {
                self.handle_class_declaration(name, superclass.as_deref(), methods)?;
                Ok(ControlFlow::None)
            }
        }
    }

    fn execute_block(&mut self, statements: &[Stmt]) -> Result<ControlFlow> {
        let previous = self.environment.clone();
        self.environment = Environment::with_parent(previous.clone());

        let mut result = ControlFlow::None;
        for statement in statements {
            match self.execute_stmt(statement)? {
                ControlFlow::Return(value) => {
                    result = ControlFlow::Return(value);
                    break;
                }
                ControlFlow::None => {}
            }
        }

        self.environment = previous;
        Ok(result)
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Literal { value, .. } => Ok(value.clone()),
            Expr::Identifier { name, pos: _ } => self
                .environment
                .get(name)
                .cloned()
                .ok_or_else(|| {
                    let available_vars = self.get_available_variable_names();
                    if let Some(suggestion) = CustomLangError::find_similar_name(name, &available_vars) {
                        CustomLangError::undefined_variable_with_suggestion(
                            name,
                            format!("Did you mean '{suggestion}'?")
                        )
                    } else {
                        CustomLangError::undefined_variable_with_suggestion(
                            name,
                            format!("Variable '{name}' is not defined. Use 'let {name} = value;' to declare it.")
                        )
                    }
                }),
            Expr::Binary {
                left,
                op,
                right,
                pos,
            } => {
                let left_val = self.evaluate_expr(left)?;
                let right_val = self.evaluate_expr(right)?;
                self.apply_binary_op(&left_val, op, &right_val, pos)
            }
            Expr::Unary { op, expr, pos } => {
                let value = self.evaluate_expr(expr)?;
                self.apply_unary_op(op, &value, pos)
            }
            Expr::Assignment {
                name,
                value,
                pos: _,
            } => {
                let val = self.evaluate_expr(value)?;
                if self.environment.assign(name, val.clone()) {
                    Ok(val)
                } else {
                    let available_vars = self.get_available_variable_names();
                    if let Some(suggestion) = CustomLangError::find_similar_name(name, &available_vars) {
                        Err(CustomLangError::undefined_variable_with_suggestion(
                            name,
                            format!("Did you mean '{suggestion}'? Or use 'let {name} = value;' to declare a new variable.")
                        ))
                    } else {
                        Err(CustomLangError::undefined_variable_with_suggestion(
                            name,
                            format!("Variable '{name}' is not defined. Use 'let {name} = value;' to declare it first.")
                        ))
                    }
                }
            }
            Expr::Call { callee, args, .. } => {
                let function = self.evaluate_expr(callee)?;
                let arguments: Result<Vec<Value>> =
                    args.iter().map(|arg| self.evaluate_expr(arg)).collect();
                let arguments = arguments?;

                match function {
                    Value::BuiltinFunction(name) => self.call_builtin_function(&name, &arguments),
                    Value::Function {
                        name,
                        params,
                        body,
                        closure,
                    } => self.call_user_function(&name, &params, &body, &closure, &arguments),
                    _ => Err(CustomLangError::runtime_error(format!(
                        "Cannot call non-function value: {}",
                        self.value_to_string(&function)
                    ))),
                }
            }
            Expr::Array { elements, .. } => {
                let mut array_values = Vec::new();
                for element in elements {
                    array_values.push(self.evaluate_expr(element)?);
                }
                Ok(Value::Array(array_values))
            }
            Expr::Object { pairs, .. } => {
                let mut object_map = std::collections::HashMap::new();
                for (key, value_expr) in pairs {
                    let value = self.evaluate_expr(value_expr)?;
                    object_map.insert(key.clone(), value);
                }
                Ok(Value::Object(object_map))
            }
            Expr::Index { object, index, .. } => {
                let obj_value = self.evaluate_expr(object)?;
                let index_value = self.evaluate_expr(index)?;

                match (&obj_value, &index_value) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let idx = *n as usize;
                        if idx < arr.len() {
                            Ok(arr[idx].clone())
                        } else {
                            Err(CustomLangError::RuntimeError {
                                message: format!(
                                    "Array index {} out of bounds (length {})",
                                    idx,
                                    arr.len()
                                ),
                            })
                        }
                    }
                    (Value::String(s), Value::Number(n)) => {
                        let idx = *n as usize;
                        if idx < s.len() {
                            Ok(Value::String(s.chars().nth(idx).unwrap().to_string()))
                        } else {
                            Err(CustomLangError::RuntimeError {
                                message: format!(
                                    "String index {} out of bounds (length {})",
                                    idx,
                                    s.len()
                                ),
                            })
                        }
                    }
                    (Value::Object(obj), Value::String(key)) => {
                        Ok(obj.get(key).cloned().unwrap_or(Value::Null))
                    }
                    _ => Err(CustomLangError::RuntimeError {
                        message: format!(
                            "Cannot index {} with {}",
                            obj_value.type_name(),
                            index_value.type_name()
                        ),
                    }),
                }
            }
            Expr::New { class_name, args, .. } => {
                self.handle_class_instantiation(class_name, args)
            }
            Expr::This { .. } => {
                // For now, return a simple placeholder
                // In a full implementation, this would reference the current instance
                Err(CustomLangError::runtime_error(
                    "'this' keyword not yet implemented in this context"
                ))
            }
            Expr::PropertyAccess { object, property, .. } => {
                let obj_value = self.evaluate_expr(object)?;
                match obj_value {
                    Value::Instance { fields, .. } => {
                        Ok(fields.get(property).cloned().unwrap_or(Value::Null))
                    }
                    Value::Object(obj) => {
                        Ok(obj.get(property).cloned().unwrap_or(Value::Null))
                    }
                    _ => Err(CustomLangError::runtime_error(format!(
                        "Cannot access property '{}' on {}",
                        property,
                        obj_value.type_name()
                    ))),
                }
            }
            Expr::Match { expr, arms, .. } => {
                let value = self.evaluate_expr(expr)?;
                self.evaluate_match(&value, arms)
            }
        }
    }

    fn apply_binary_op(
        &self,
        left: &Value,
        op: &BinaryOp,
        right: &Value,
        _pos: &Position,
    ) -> Result<Value> {
        match (left, op, right) {
            // Arithmetic operations
            (Value::Number(a), BinaryOp::Add, Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Number(a), BinaryOp::Subtract, Value::Number(b)) => Ok(Value::Number(a - b)),
            (Value::Number(a), BinaryOp::Multiply, Value::Number(b)) => Ok(Value::Number(a * b)),
            (Value::Number(a), BinaryOp::Divide, Value::Number(b)) => {
                if *b == 0.0 {
                    Err(CustomLangError::DivisionByZero)
                } else {
                    Ok(Value::Number(a / b))
                }
            }
            (Value::Number(a), BinaryOp::Modulo, Value::Number(b)) => {
                if *b == 0.0 {
                    Err(CustomLangError::DivisionByZero)
                } else {
                    Ok(Value::Number(a % b))
                }
            }

            // String concatenation
            (Value::String(a), BinaryOp::Add, Value::String(b)) => {
                Ok(Value::String(format!("{a}{b}")))
            }
            // String + Number concatenation
            (Value::String(a), BinaryOp::Add, Value::Number(b)) => Ok(Value::String(format!(
                "{}{}",
                a,
                self.value_to_string(&Value::Number(*b))
            ))),
            // Number + String concatenation
            (Value::Number(a), BinaryOp::Add, Value::String(b)) => Ok(Value::String(format!(
                "{}{}",
                self.value_to_string(&Value::Number(*a)),
                b
            ))),
            // String + Boolean concatenation
            (Value::String(a), BinaryOp::Add, Value::Boolean(b)) => Ok(Value::String(format!(
                "{}{}",
                a,
                self.value_to_string(&Value::Boolean(*b))
            ))),
            // Boolean + String concatenation
            (Value::Boolean(a), BinaryOp::Add, Value::String(b)) => Ok(Value::String(format!(
                "{}{}",
                self.value_to_string(&Value::Boolean(*a)),
                b
            ))),
            // String + Null concatenation
            (Value::String(a), BinaryOp::Add, Value::Null) => Ok(Value::String(format!(
                "{}{}",
                a,
                self.value_to_string(&Value::Null)
            ))),
            // Null + String concatenation
            (Value::Null, BinaryOp::Add, Value::String(b)) => Ok(Value::String(format!(
                "{}{}",
                self.value_to_string(&Value::Null),
                b
            ))),
            // String + Array concatenation
            (Value::String(a), BinaryOp::Add, Value::Array(b)) => Ok(Value::String(format!(
                "{}{}",
                a,
                self.value_to_string(&Value::Array(b.clone()))
            ))),
            // Array + String concatenation
            (Value::Array(a), BinaryOp::Add, Value::String(b)) => Ok(Value::String(format!(
                "{}{}",
                self.value_to_string(&Value::Array(a.clone())),
                b
            ))),
            // Array + Array concatenation
            (Value::Array(a), BinaryOp::Add, Value::Array(b)) => {
                let mut result = a.clone();
                result.extend(b.clone());
                Ok(Value::Array(result))
            }
            // String + Object concatenation
            (Value::String(a), BinaryOp::Add, Value::Object(b)) => Ok(Value::String(format!(
                "{}{}",
                a,
                self.value_to_string(&Value::Object(b.clone()))
            ))),
            // Object + String concatenation
            (Value::Object(a), BinaryOp::Add, Value::String(b)) => Ok(Value::String(format!(
                "{}{}",
                self.value_to_string(&Value::Object(a.clone())),
                b
            ))),

            // Comparison operations
            (Value::Number(a), BinaryOp::Less, Value::Number(b)) => Ok(Value::Boolean(a < b)),
            (Value::Number(a), BinaryOp::LessEqual, Value::Number(b)) => Ok(Value::Boolean(a <= b)),
            (Value::Number(a), BinaryOp::Greater, Value::Number(b)) => Ok(Value::Boolean(a > b)),
            (Value::Number(a), BinaryOp::GreaterEqual, Value::Number(b)) => {
                Ok(Value::Boolean(a >= b))
            }

            // Equality operations (work with any types)
            (a, BinaryOp::Equal, b) => Ok(Value::Boolean(self.values_equal(a, b))),
            (a, BinaryOp::NotEqual, b) => Ok(Value::Boolean(!self.values_equal(a, b))),

            // Logical operations
            (a, BinaryOp::And, b) => {
                if a.is_truthy() {
                    Ok(b.clone())
                } else {
                    Ok(a.clone())
                }
            }
            (a, BinaryOp::Or, b) => {
                if a.is_truthy() {
                    Ok(a.clone())
                } else {
                    Ok(b.clone())
                }
            }

            _ => {
                let op_name = match op {
                    BinaryOp::Add => "addition (+)",
                    BinaryOp::Subtract => "subtraction (-)",
                    BinaryOp::Multiply => "multiplication (*)",
                    BinaryOp::Divide => "division (/)",
                    BinaryOp::Modulo => "modulo (%)",
                    BinaryOp::Less => "less than (<)",
                    BinaryOp::LessEqual => "less than or equal (<=)",
                    BinaryOp::Greater => "greater than (>)",
                    BinaryOp::GreaterEqual => "greater than or equal (>=)",
                    BinaryOp::Equal => "equality (==)",
                    BinaryOp::NotEqual => "inequality (!=)",
                    BinaryOp::And => "logical AND (&&)",
                    BinaryOp::Or => "logical OR (||)",
                };

                let suggestion = match (left.type_name(), op, right.type_name()) {
                    ("string", BinaryOp::Add, other) => {
                        format!("To concatenate strings, convert the {other} to a string first.")
                    }
                    (
                        left_type,
                        BinaryOp::Add
                        | BinaryOp::Subtract
                        | BinaryOp::Multiply
                        | BinaryOp::Divide
                        | BinaryOp::Modulo,
                        right_type,
                    ) => {
                        format!("Arithmetic operations require numbers. You have {left_type} and {right_type}.")
                    }
                    (
                        left_type,
                        BinaryOp::Less
                        | BinaryOp::LessEqual
                        | BinaryOp::Greater
                        | BinaryOp::GreaterEqual,
                        right_type,
                    ) => {
                        format!("Comparison operations require compatible types. You have {left_type} and {right_type}.")
                    }
                    _ => "Check the types of your values. Use type() function to inspect them."
                        .to_string(),
                };

                Err(CustomLangError::type_error(format!(
                    "Cannot perform {} on {} and {}. {}",
                    op_name,
                    left.type_name(),
                    right.type_name(),
                    suggestion
                )))
            }
        }
    }

    fn apply_unary_op(&self, op: &UnaryOp, value: &Value, _pos: &Position) -> Result<Value> {
        match (op, value) {
            (UnaryOp::Minus, Value::Number(n)) => Ok(Value::Number(-n)),
            (UnaryOp::Not, value) => Ok(Value::Boolean(!value.is_truthy())),
            _ => Err(CustomLangError::type_error(format!(
                "Unsupported unary operation: {:?} {}",
                op,
                value.type_name()
            ))),
        }
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

    fn call_builtin_function(&mut self, name: &str, args: &[Value]) -> Result<Value> {
        match name {
            "len" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "len() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::Number(s.len() as f64)),
                    Value::Array(arr) => Ok(Value::Number(arr.len() as f64)),
                    _ => Err(CustomLangError::type_error(
                        "len() argument must be a string or array",
                    )),
                }
            }
            "abs" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "abs() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Number(n) => Ok(Value::Number(n.abs())),
                    _ => Err(CustomLangError::type_error(
                        "abs() argument must be a number",
                    )),
                }
            }
            "sqrt" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "sqrt() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Number(n) => {
                        if *n < 0.0 {
                            Err(CustomLangError::runtime_error(
                                "sqrt() argument must be non-negative",
                            ))
                        } else {
                            Ok(Value::Number(n.sqrt()))
                        }
                    }
                    _ => Err(CustomLangError::type_error(
                        "sqrt() argument must be a number",
                    )),
                }
            }
            "pow" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "pow() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::Number(base), Value::Number(exp)) => Ok(Value::Number(base.powf(*exp))),
                    _ => Err(CustomLangError::type_error(
                        "pow() arguments must be numbers",
                    )),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "min() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.min(*b))),
                    _ => Err(CustomLangError::type_error(
                        "min() arguments must be numbers",
                    )),
                }
            }
            "max" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "max() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.max(*b))),
                    _ => Err(CustomLangError::type_error(
                        "max() arguments must be numbers",
                    )),
                }
            }
            "type" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "type() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                Ok(Value::String(args[0].type_name().to_string()))
            }
            "input" => {
                if args.len() > 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "input() takes at most 1 argument ({} given)",
                        args.len()
                    )));
                }

                // Print prompt if provided
                if !args.is_empty() {
                    print!("{}", self.value_to_string(&args[0]));
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }

                // Read input
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).map_err(|e| {
                    CustomLangError::runtime_error(format!("Failed to read input: {e}"))
                })?;

                // Remove trailing newline
                if input.ends_with('\n') {
                    input.pop();
                    if input.ends_with('\r') {
                        input.pop();
                    }
                }

                Ok(Value::String(input))
            }
            "push" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "push() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.push(args[1].clone());
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(CustomLangError::type_error(
                        "push() first argument must be an array",
                    )),
                }
            }
            "pop" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "pop() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            Ok(Value::Null)
                        } else {
                            Ok(arr[arr.len() - 1].clone())
                        }
                    }
                    _ => Err(CustomLangError::type_error(
                        "pop() argument must be an array",
                    )),
                }
            }
            "first" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "first() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            Ok(Value::Null)
                        } else {
                            Ok(arr[0].clone())
                        }
                    }
                    _ => Err(CustomLangError::type_error(
                        "first() argument must be an array",
                    )),
                }
            }
            "last" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "last() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            Ok(Value::Null)
                        } else {
                            Ok(arr[arr.len() - 1].clone())
                        }
                    }
                    _ => Err(CustomLangError::type_error(
                        "last() argument must be an array",
                    )),
                }
            }
            "read_file" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "read_file() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(filename) => match std::fs::read_to_string(filename) {
                        Ok(content) => Ok(Value::String(content)),
                        Err(e) => Err(CustomLangError::runtime_error(format!(
                            "Failed to read file '{filename}': {e}"
                        ))),
                    },
                    _ => Err(CustomLangError::type_error(
                        "read_file() argument must be a string (filename)",
                    )),
                }
            }
            "write_file" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "write_file() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::String(filename), content) => {
                        let content_str = self.value_to_string(content);
                        match std::fs::write(filename, content_str) {
                            Ok(()) => Ok(Value::Boolean(true)),
                            Err(e) => Err(CustomLangError::runtime_error(format!(
                                "Failed to write file '{filename}': {e}"
                            ))),
                        }
                    }
                    _ => Err(CustomLangError::type_error(
                        "write_file() first argument must be a string (filename)",
                    )),
                }
            }
            "split" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "split() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::String(text), Value::String(delimiter)) => {
                        let parts: Vec<Value> = text
                            .split(delimiter)
                            .map(|s| Value::String(s.to_string()))
                            .collect();
                        Ok(Value::Array(parts))
                    }
                    _ => Err(CustomLangError::type_error(
                        "split() arguments must be strings (text, delimiter)",
                    )),
                }
            }
            "join" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "join() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), Value::String(delimiter)) => {
                        let strings: Vec<String> =
                            arr.iter().map(|v| self.value_to_string(v)).collect();
                        Ok(Value::String(strings.join(delimiter)))
                    }
                    _ => Err(CustomLangError::type_error(
                        "join() arguments must be (array, string delimiter)",
                    )),
                }
            }
            "substring" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(CustomLangError::runtime_error(format!(
                        "substring() takes 2 or 3 arguments ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(text) => {
                        let start = match &args[1] {
                            Value::Number(n) => *n as usize,
                            _ => {
                                return Err(CustomLangError::type_error(
                                    "substring() start index must be a number",
                                ))
                            }
                        };

                        let chars: Vec<char> = text.chars().collect();
                        if start >= chars.len() {
                            return Ok(Value::String(String::new()));
                        }

                        let end = if args.len() == 3 {
                            match &args[2] {
                                Value::Number(n) => (*n as usize).min(chars.len()),
                                _ => {
                                    return Err(CustomLangError::type_error(
                                        "substring() end index must be a number",
                                    ))
                                }
                            }
                        } else {
                            chars.len()
                        };

                        if start >= end {
                            Ok(Value::String(String::new()))
                        } else {
                            let result: String = chars[start..end].iter().collect();
                            Ok(Value::String(result))
                        }
                    }
                    _ => Err(CustomLangError::type_error(
                        "substring() first argument must be a string",
                    )),
                }
            }
            "to_upper" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "to_upper() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(text) => Ok(Value::String(text.to_uppercase())),
                    _ => Err(CustomLangError::type_error(
                        "to_upper() argument must be a string",
                    )),
                }
            }
            "to_lower" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "to_lower() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(text) => Ok(Value::String(text.to_lowercase())),
                    _ => Err(CustomLangError::type_error(
                        "to_lower() argument must be a string",
                    )),
                }
            }
            "trim" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "trim() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::String(text) => Ok(Value::String(text.trim().to_string())),
                    _ => Err(CustomLangError::type_error(
                        "trim() argument must be a string",
                    )),
                }
            }
            "starts_with" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "starts_with() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::String(text), Value::String(prefix)) => {
                        Ok(Value::Boolean(text.starts_with(prefix)))
                    }
                    _ => Err(CustomLangError::type_error(
                        "starts_with() arguments must be strings (text, prefix)",
                    )),
                }
            }
            "ends_with" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "ends_with() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::String(text), Value::String(suffix)) => {
                        Ok(Value::Boolean(text.ends_with(suffix)))
                    }
                    _ => Err(CustomLangError::type_error(
                        "ends_with() arguments must be strings (text, suffix)",
                    )),
                }
            }
            "contains" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "contains() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1]) {
                    (Value::String(text), Value::String(substring)) => {
                        Ok(Value::Boolean(text.contains(substring)))
                    }
                    _ => Err(CustomLangError::type_error(
                        "contains() arguments must be strings (text, substring)",
                    )),
                }
            }
            "replace" => {
                if args.len() != 3 {
                    return Err(CustomLangError::runtime_error(format!(
                        "replace() takes exactly 3 arguments ({} given)",
                        args.len()
                    )));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(text), Value::String(from), Value::String(to)) => {
                        Ok(Value::String(text.replace(from, to)))
                    }
                    _ => Err(CustomLangError::type_error(
                        "replace() arguments must be strings (text, from, to)",
                    )),
                }
            }
            "sort" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "sort() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut sorted_arr = arr.clone();
                        sorted_arr.sort_by(|a, b| match (a, b) {
                            (Value::Number(a), Value::Number(b)) => {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            (Value::String(a), Value::String(b)) => a.cmp(b),
                            (Value::Boolean(a), Value::Boolean(b)) => a.cmp(b),
                            _ => std::cmp::Ordering::Equal,
                        });
                        Ok(Value::Array(sorted_arr))
                    }
                    _ => Err(CustomLangError::type_error(
                        "sort() argument must be an array",
                    )),
                }
            }
            "reverse" => {
                if args.len() != 1 {
                    return Err(CustomLangError::runtime_error(format!(
                        "reverse() takes exactly 1 argument ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut reversed_arr = arr.clone();
                        reversed_arr.reverse();
                        Ok(Value::Array(reversed_arr))
                    }
                    _ => Err(CustomLangError::type_error(
                        "reverse() argument must be an array",
                    )),
                }
            }
            "includes" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "includes() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let search_value = &args[1];
                        let found = arr.iter().any(|item| self.values_equal(item, search_value));
                        Ok(Value::Boolean(found))
                    }
                    _ => Err(CustomLangError::type_error(
                        "includes() first argument must be an array",
                    )),
                }
            }
            "find" => {
                if args.len() != 2 {
                    return Err(CustomLangError::runtime_error(format!(
                        "find() takes exactly 2 arguments ({} given)",
                        args.len()
                    )));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let search_value = &args[1];
                        for item in arr {
                            if self.values_equal(item, search_value) {
                                return Ok(item.clone());
                            }
                        }
                        Ok(Value::Null)
                    }
                    _ => Err(CustomLangError::type_error(
                        "find() first argument must be an array",
                    )),
                }
            }
            _ => Err(CustomLangError::runtime_error(format!(
                "Unknown builtin function: {name}"
            ))),
        }
    }

    fn call_user_function(
        &mut self,
        name: &str,
        params: &[String],
        body: &Stmt,
        closure: &Environment,
        args: &[Value],
    ) -> Result<Value> {
        if args.len() != params.len() {
            return Err(CustomLangError::runtime_error(format!(
                "Function expects {} arguments, got {}",
                params.len(),
                args.len()
            )));
        }

        // Create new environment for function execution
        let previous_env = self.environment.clone();
        self.environment = Environment::with_parent(closure.clone());

        // Add the function itself to the environment for recursion
        let function_value = Value::Function {
            name: name.to_string(),
            params: params.to_vec(),
            body: Box::new(body.clone()),
            closure: closure.clone(),
        };
        self.environment.define(name.to_string(), function_value);

        // Bind parameters to arguments
        for (param, arg) in params.iter().zip(args.iter()) {
            self.environment.define(param.clone(), arg.clone());
        }

        // Execute function body
        let result = match self.execute_stmt(body)? {
            ControlFlow::Return(value) => Ok(value),
            ControlFlow::None => Ok(Value::Null),
        };

        // Restore previous environment
        self.environment = previous_env;

        result
    }

    #[allow(clippy::only_used_in_recursion)]
    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Value::String(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| self.value_to_string(v)).collect();
                format!("[{}]", elements.join(", "))
            }
            Value::Object(obj) => {
                let pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.value_to_string(v)))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
            Value::Class { name, .. } => format!("<class {name}>"),
            Value::Instance {
                class_name, fields, ..
            } => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.value_to_string(v)))
                    .collect();
                format!("<{} instance {{{}}}>", class_name, field_strs.join(", "))
            }
            Value::Function { name, .. } => format!("<function {name}>"),
            Value::BuiltinFunction(name) => format!("<builtin {name}>"),
        }
    }

    /// Get all available variable names for error suggestions
    fn get_available_variable_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut current_env = Some(&self.environment);

        while let Some(env) = current_env {
            names.extend(env.variables.keys().cloned());
            current_env = env.parent.as_ref().map(|p| p.as_ref());
        }

        names
    }

    /// Get all available function names for error suggestions
    #[allow(dead_code)]
    fn get_available_function_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut current_env = Some(&self.environment);

        while let Some(env) = current_env {
            for (name, value) in &env.variables {
                if matches!(value, Value::Function { .. } | Value::BuiltinFunction(_)) {
                    names.push(name.clone());
                }
            }
            current_env = env.parent.as_ref().map(|p| p.as_ref());
        }

        names
    }

    /// Handle import statement
    fn handle_import(&mut self, module_path: &str, alias: Option<&str>) -> Result<()> {
        // For now, implement a basic file-based module system
        let file_path = if module_path.ends_with(".cl") {
            module_path.to_string()
        } else {
            format!("{module_path}.cl")
        };

        // Read and parse the module file
        let module_source = std::fs::read_to_string(&file_path).map_err(|e| {
            CustomLangError::runtime_error(format!("Failed to read module '{file_path}': {e}"))
        })?;

        // Parse the module
        let mut lexer = crate::lexer::Lexer::new(&module_source);
        let tokens = lexer.tokenize().map_err(|e| {
            CustomLangError::runtime_error(format!("Failed to tokenize module '{file_path}': {e}"))
        })?;

        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().map_err(|e| {
            CustomLangError::runtime_error(format!("Failed to parse module '{file_path}': {e}"))
        })?;

        // Create a new environment for the module
        let mut module_env = Environment::new();
        module_env.parent = Some(Box::new(self.environment.clone()));

        // Execute the module in its own environment
        let mut module_interpreter = Interpreter::new();
        module_interpreter.environment = module_env;

        println!("Executing module statements...");
        for stmt in &program.statements {
            module_interpreter.execute_stmt(stmt)?;
        }
        println!(
            "Module execution complete. Variables in module: {:?}",
            module_interpreter
                .environment
                .variables
                .keys()
                .collect::<Vec<_>>()
        );

        // Import exported values into current environment
        let _module_name = alias.unwrap_or(
            std::path::Path::new(module_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(module_path),
        );

        // For now, import all variables from the module
        // In a more sophisticated system, we'd only import explicitly exported items
        for (name, value) in &module_interpreter.environment.variables {
            let imported_name = if let Some(alias) = alias {
                format!("{alias}_{name}")
            } else {
                name.clone()
            };
            self.environment.define(imported_name, value.clone());
        }

        println!(
            "Imported {} items from module '{}'",
            module_interpreter.environment.variables.len(),
            module_path
        );

        Ok(())
    }

    /// Handle export statement
    fn handle_export(&mut self, name: &str) -> Result<()> {
        // For now, exports are just markers
        // In a more sophisticated system, we'd track which items are exported
        // and only make those available to importing modules

        // Verify that the exported name exists
        if !self.environment.variables.contains_key(name) {
            return Err(CustomLangError::runtime_error(format!(
                "Cannot export '{name}': variable or function not found"
            )));
        }

        // For now, we'll just mark it as exported by adding a special prefix
        // This is a simplified implementation
        println!("Exported: {name}");
        Ok(())
    }

    /// Handle class declaration
    fn handle_class_declaration(
        &mut self,
        name: &str,
        superclass: Option<&str>,
        methods: &[Stmt],
    ) -> Result<()> {
        // Create method map
        let mut method_map = HashMap::new();

        for method in methods {
            if let Stmt::Function {
                name: method_name,
                params,
                body,
                ..
            } = method
            {
                let function_value = Value::Function {
                    name: method_name.clone(),
                    params: params.clone(),
                    body: Box::new(body.as_ref().clone()),
                    closure: self.environment.clone(),
                };
                method_map.insert(method_name.clone(), function_value);
            }
        }

        // Handle inheritance
        let superclass_value = if let Some(superclass_name) = superclass {
            match self.environment.get(superclass_name) {
                Some(Value::Class { .. }) => Some(Box::new(
                    self.environment.get(superclass_name).unwrap().clone(),
                )),
                Some(_) => {
                    return Err(CustomLangError::runtime_error(format!(
                        "'{superclass_name}' is not a class"
                    )))
                }
                None => {
                    return Err(CustomLangError::runtime_error(format!(
                        "Undefined superclass '{superclass_name}'"
                    )))
                }
            }
        } else {
            None
        };

        // Create class value
        let class_value = Value::Class {
            name: name.to_string(),
            methods: method_map,
            superclass: superclass_value,
        };

        // Define the class in the current environment
        self.environment.define(name.to_string(), class_value);
        Ok(())
    }

    /// Handle class instantiation (new ClassName(args))
    fn handle_class_instantiation(&mut self, class_name: &str, args: &[Expr]) -> Result<Value> {
        // Get the class definition
        let class = match self.environment.get(class_name) {
            Some(Value::Class {
                name,
                methods,
                superclass,
            }) => (name.clone(), methods.clone(), superclass.clone()),
            Some(_) => {
                return Err(CustomLangError::runtime_error(format!(
                    "'{class_name}' is not a class"
                )))
            }
            None => {
                return Err(CustomLangError::runtime_error(format!(
                    "Undefined class '{class_name}'"
                )))
            }
        };

        let (class_name, methods, _superclass) = class;

        // Create instance with empty fields initially
        let mut instance = Value::Instance {
            class_name: class_name.clone(),
            fields: HashMap::new(),
            methods: methods.clone(),
        };

        // Call constructor if it exists
        if let Some(constructor) = methods.get("init") {
            // Evaluate arguments
            let mut arg_values = Vec::new();
            for arg in args {
                arg_values.push(self.evaluate_expr(arg)?);
            }

            // Call constructor with the instance as 'this'
            if let Value::Function {
                params,
                body,
                closure,
                ..
            } = constructor
            {
                if params.len() != arg_values.len() {
                    return Err(CustomLangError::runtime_error(format!(
                        "Constructor expects {} arguments, got {}",
                        params.len(),
                        arg_values.len()
                    )));
                }

                // Create new environment for constructor
                let mut constructor_env = Environment::new();
                constructor_env.parent = Some(Box::new(closure.clone()));

                // Bind parameters
                for (param, arg) in params.iter().zip(arg_values.iter()) {
                    constructor_env.define(param.clone(), arg.clone());
                }

                // Bind 'this' to the instance
                constructor_env.define("this".to_string(), instance.clone());

                // Execute constructor
                let mut constructor_interpreter = Interpreter::new();
                constructor_interpreter.environment = constructor_env;

                match constructor_interpreter.execute_stmt(body) {
                    Ok(_) => {
                        // Update instance fields from constructor environment
                        if let Value::Instance { fields, .. } = &mut instance {
                            // Copy any new fields that were set in the constructor
                            for (key, value) in &constructor_interpreter.environment.variables {
                                if key != "this" && !params.contains(key) {
                                    fields.insert(key.clone(), value.clone());
                                }
                            }
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(instance)
    }

    /// Evaluate match expression
    fn evaluate_match(&mut self, value: &Value, arms: &[MatchArm]) -> Result<Value> {
        for arm in arms {
            if let Some(bindings) = self.pattern_matches(&arm.pattern, value)? {
                // Create new environment with pattern bindings
                let mut match_env = Environment::new();
                match_env.parent = Some(Box::new(self.environment.clone()));

                // Bind pattern variables
                for (name, val) in bindings {
                    match_env.define(name, val);
                }

                // Evaluate body in the new environment
                let old_env = std::mem::replace(&mut self.environment, match_env);
                let result = self.evaluate_expr(&arm.body);
                self.environment = old_env;

                return result;
            }
        }

        Err(CustomLangError::runtime_error(
            "No pattern matched in match expression",
        ))
    }

    /// Check if a pattern matches a value and return variable bindings
    fn pattern_matches(
        &self,
        pattern: &Pattern,
        value: &Value,
    ) -> Result<Option<Vec<(String, Value)>>> {
        match pattern {
            Pattern::Literal(literal_value) => {
                if self.values_equal(literal_value, value) {
                    Ok(Some(Vec::new()))
                } else {
                    Ok(None)
                }
            }
            Pattern::Variable(name) => Ok(Some(vec![(name.clone(), value.clone())])),
            Pattern::Wildcard => Ok(Some(Vec::new())),
            Pattern::Array(patterns) => {
                if let Value::Array(array) = value {
                    if patterns.len() != array.len() {
                        return Ok(None);
                    }

                    let mut bindings = Vec::new();
                    for (pattern, value) in patterns.iter().zip(array.iter()) {
                        if let Some(mut pattern_bindings) = self.pattern_matches(pattern, value)? {
                            bindings.append(&mut pattern_bindings);
                        } else {
                            return Ok(None);
                        }
                    }
                    Ok(Some(bindings))
                } else {
                    Ok(None)
                }
            }
            Pattern::Object(pattern_pairs) => {
                if let Value::Object(object) = value {
                    let mut bindings = Vec::new();

                    for (key, pattern) in pattern_pairs {
                        if let Some(obj_value) = object.get(key) {
                            if let Some(mut pattern_bindings) =
                                self.pattern_matches(pattern, obj_value)?
                            {
                                bindings.append(&mut pattern_bindings);
                            } else {
                                return Ok(None);
                            }
                        } else {
                            return Ok(None);
                        }
                    }
                    Ok(Some(bindings))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
