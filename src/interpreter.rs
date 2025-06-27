use crate::ast::*;
use crate::error::{CustomLangError, Result};

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
                .ok_or_else(|| CustomLangError::undefined_variable(name)),
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
                    Err(CustomLangError::undefined_variable(name))
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
                    _ => Err(CustomLangError::RuntimeError {
                        message: format!(
                            "Cannot index {} with {}",
                            obj_value.type_name(),
                            index_value.type_name()
                        ),
                    }),
                }
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

            _ => Err(CustomLangError::type_error(format!(
                "Unsupported operation: {} {:?} {}",
                left.type_name(),
                op,
                right.type_name()
            ))),
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
            Value::Function { name, .. } => format!("<function {name}>"),
            Value::BuiltinFunction(name) => format!("<builtin {name}>"),
        }
    }
}
