//! # Semantic Analysis
//!
//! Static analysis phase that performs type checking and semantic validation
//! before code execution. This helps catch errors early and provides better
//! error messages for common programming mistakes.
//!
//! ## Features
//! - Type inference and checking
//! - Variable scope validation
//! - Function signature verification
//! - Dead code detection
//! - Comprehensive error reporting

use crate::ast::*;
use crate::error::{CustomLangError, Result};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Number,
    String,
    Boolean,
    Null,
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Unknown,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::Boolean => write!(f, "boolean"),
            Type::Null => write!(f, "null"),
            Type::Function {
                params,
                return_type,
            } => {
                write!(f, "function(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {return_type}")
            }
            Type::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug)]
struct Scope {
    variables: HashMap<String, Type>,
    functions: HashMap<String, Type>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}

pub struct SemanticAnalyzer {
    scopes: Vec<Scope>,
    current_function_return_type: Option<Type>,
    errors: Vec<CustomLangError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            scopes: vec![Scope::new()],
            current_function_return_type: None,
            errors: Vec::new(),
        };

        // Add built-in functions to global scope
        analyzer.add_builtin_functions();
        analyzer
    }

    fn add_builtin_functions(&mut self) {
        let global_scope = &mut self.scopes[0];

        // Helper function to add both as function and variable
        let mut add_builtin = |name: &str, func_type: Type| {
            global_scope
                .functions
                .insert(name.to_string(), func_type.clone());
            global_scope.variables.insert(name.to_string(), func_type);
        };

        // Math functions
        add_builtin(
            "abs",
            Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            },
        );
        add_builtin(
            "sqrt",
            Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            },
        );
        add_builtin(
            "pow",
            Type::Function {
                params: vec![Type::Number, Type::Number],
                return_type: Box::new(Type::Number),
            },
        );
        add_builtin(
            "min",
            Type::Function {
                params: vec![Type::Number, Type::Number],
                return_type: Box::new(Type::Number),
            },
        );
        add_builtin(
            "max",
            Type::Function {
                params: vec![Type::Number, Type::Number],
                return_type: Box::new(Type::Number),
            },
        );

        // Utility functions
        add_builtin(
            "len",
            Type::Function {
                params: vec![Type::Unknown], // Now supports strings and arrays
                return_type: Box::new(Type::Number),
            },
        );
        add_builtin(
            "type",
            Type::Function {
                params: vec![Type::Unknown], // Can accept any type
                return_type: Box::new(Type::String),
            },
        );
        add_builtin(
            "print",
            Type::Function {
                params: vec![Type::Unknown], // Can print any type
                return_type: Box::new(Type::Null),
            },
        );

        // Array functions
        add_builtin(
            "push",
            Type::Function {
                params: vec![Type::Unknown, Type::Unknown],
                return_type: Box::new(Type::Unknown),
            },
        );
        add_builtin(
            "pop",
            Type::Function {
                params: vec![Type::Unknown],
                return_type: Box::new(Type::Unknown),
            },
        );
        add_builtin(
            "first",
            Type::Function {
                params: vec![Type::Unknown],
                return_type: Box::new(Type::Unknown),
            },
        );
        add_builtin(
            "last",
            Type::Function {
                params: vec![Type::Unknown],
                return_type: Box::new(Type::Unknown),
            },
        );
        add_builtin(
            "sort",
            Type::Function {
                params: vec![Type::Unknown],          // Array
                return_type: Box::new(Type::Unknown), // Array
            },
        );
        add_builtin(
            "reverse",
            Type::Function {
                params: vec![Type::Unknown],          // Array
                return_type: Box::new(Type::Unknown), // Array
            },
        );
        add_builtin(
            "includes",
            Type::Function {
                params: vec![Type::Unknown, Type::Unknown], // Array, value
                return_type: Box::new(Type::Boolean),
            },
        );
        add_builtin(
            "find",
            Type::Function {
                params: vec![Type::Unknown, Type::Unknown], // Array, value
                return_type: Box::new(Type::Unknown),
            },
        );
        add_builtin(
            "filter",
            Type::Function {
                params: vec![Type::Unknown], // Will implement later
                return_type: Box::new(Type::Unknown),
            },
        );
        add_builtin(
            "map",
            Type::Function {
                params: vec![Type::Unknown], // Will implement later
                return_type: Box::new(Type::Unknown),
            },
        );
        add_builtin(
            "reduce",
            Type::Function {
                params: vec![Type::Unknown], // Will implement later
                return_type: Box::new(Type::Unknown),
            },
        );

        // File I/O functions
        add_builtin(
            "read_file",
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );
        add_builtin(
            "write_file",
            Type::Function {
                params: vec![Type::String, Type::Unknown],
                return_type: Box::new(Type::Boolean),
            },
        );

        // String manipulation functions
        add_builtin(
            "split",
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::Unknown), // Returns array
            },
        );
        add_builtin(
            "join",
            Type::Function {
                params: vec![Type::Unknown, Type::String], // Array, delimiter
                return_type: Box::new(Type::String),
            },
        );
        add_builtin(
            "substring",
            Type::Function {
                params: vec![Type::Unknown], // Variable arguments (2 or 3)
                return_type: Box::new(Type::String),
            },
        );
        add_builtin(
            "to_upper",
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );
        add_builtin(
            "to_lower",
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );
        add_builtin(
            "trim",
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );
        add_builtin(
            "starts_with",
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::Boolean),
            },
        );
        add_builtin(
            "ends_with",
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::Boolean),
            },
        );
        add_builtin(
            "contains",
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::Boolean),
            },
        );
        add_builtin(
            "replace",
            Type::Function {
                params: vec![Type::String, Type::String, Type::String],
                return_type: Box::new(Type::String),
            },
        );
    }

    pub fn analyze(&mut self, program: &Program) -> Result<()> {
        for stmt in &program.statements {
            self.analyze_statement(stmt)?;
        }

        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
        }

        Ok(())
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_variable(&mut self, name: &str, var_type: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name.to_string(), var_type);
        }
    }

    fn declare_function(&mut self, name: &str, func_type: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.functions.insert(name.to_string(), func_type);
        }
    }

    fn lookup_variable(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(var_type) = scope.variables.get(name) {
                return Some(var_type);
            }
        }
        None
    }

    fn lookup_function(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(func_type) = scope.functions.get(name) {
                return Some(func_type);
            }
        }
        None
    }

    fn analyze_statement(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::VarDeclaration {
                name, initializer, ..
            } => {
                if let Some(value) = initializer {
                    let value_type = self.analyze_expression(value)?;
                    self.declare_variable(name, value_type);
                } else {
                    self.declare_variable(name, Type::Unknown);
                }
            }
            Stmt::Function {
                name, params, body, ..
            } => {
                // Create function type
                let param_types = vec![Type::Unknown; params.len()]; // We'll infer these
                let func_type = Type::Function {
                    params: param_types,
                    return_type: Box::new(Type::Unknown), // We'll infer this
                };

                self.declare_function(name, func_type);

                // Analyze function body in new scope
                self.push_scope();

                // Add parameters to scope
                for param in params {
                    self.declare_variable(param, Type::Unknown);
                }

                let old_return_type = self.current_function_return_type.clone();
                self.current_function_return_type = Some(Type::Unknown);

                self.analyze_statement(body)?;

                self.current_function_return_type = old_return_type;
                self.pop_scope();
            }
            Stmt::If {
                condition,
                then_stmt,
                else_stmt,
                ..
            } => {
                let cond_type = self.analyze_expression(condition)?;
                if !matches!(cond_type, Type::Boolean | Type::Unknown) {
                    self.errors.push(CustomLangError::SemanticError {
                        message: format!("If condition must be boolean, got {cond_type}"),
                    });
                }

                self.push_scope();
                self.analyze_statement(then_stmt)?;
                self.pop_scope();

                if let Some(else_statement) = else_stmt {
                    self.push_scope();
                    self.analyze_statement(else_statement)?;
                    self.pop_scope();
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                let cond_type = self.analyze_expression(condition)?;
                if !matches!(cond_type, Type::Boolean | Type::Unknown) {
                    self.errors.push(CustomLangError::SemanticError {
                        message: format!("While condition must be boolean, got {cond_type}"),
                    });
                }

                self.push_scope();
                self.analyze_statement(body)?;
                self.pop_scope();
            }
            Stmt::Block { statements, .. } => {
                self.push_scope();
                for stmt in statements {
                    self.analyze_statement(stmt)?;
                }
                self.pop_scope();
            }
            Stmt::Print { expr, .. } => {
                self.analyze_expression(expr)?;
            }
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    let return_type = self.analyze_expression(expr)?;
                    // Check if return type matches function return type
                    if let Some(expected_type) = &self.current_function_return_type {
                        if !self.types_compatible(&return_type, expected_type)
                            && !matches!(expected_type, Type::Unknown)
                        {
                            self.errors.push(CustomLangError::SemanticError {
                                message: format!(
                                    "Return type mismatch: expected {expected_type}, got {return_type}"
                                ),
                            });
                        }
                    }
                }
            }
            Stmt::Expression { expr, .. } => {
                self.analyze_expression(expr)?;
            }
            Stmt::Import {
                module_path, alias, ..
            } => {
                // For now, just validate that the module path is a string
                // In a more sophisticated system, we'd validate the module exists
                // and analyze its exports
                if alias.is_some() {
                    // If there's an alias, we could add it to the current scope
                    // For now, we'll skip this
                }
                // Note: module_path is already validated as a string by the parser
                let _ = module_path; // Suppress unused warning
            }
            Stmt::Export { name, .. } => {
                // Validate that the exported name exists in the current scope
                if self.lookup_variable(name).is_none() {
                    self.errors
                        .push(CustomLangError::undefined_variable_with_suggestion(
                            name,
                            "Cannot export undefined variable or function".to_string(),
                        ));
                }
            }
            Stmt::Class {
                name,
                superclass,
                methods,
                ..
            } => {
                // Declare the class in the current scope
                self.declare_variable(name, Type::Unknown);

                // Validate superclass if present
                if let Some(superclass_name) = superclass {
                    if self.lookup_variable(superclass_name).is_none() {
                        self.errors
                            .push(CustomLangError::undefined_variable_with_suggestion(
                                superclass_name,
                                "Superclass must be defined before use".to_string(),
                            ));
                    }
                }

                // Analyze methods
                for method in methods {
                    self.analyze_statement(method)?;
                }
            }
        }
        Ok(())
    }

    fn analyze_expression(&mut self, expr: &Expr) -> Result<Type> {
        match expr {
            Expr::Literal { value, .. } => {
                Ok(match value {
                    Value::Number(_) => Type::Number,
                    Value::String(_) => Type::String,
                    Value::Boolean(_) => Type::Boolean,
                    Value::Null => Type::Null,
                    Value::Array(_) => Type::Unknown, // Arrays are dynamic for now
                    Value::Object(_) => Type::Unknown, // Objects are dynamic for now
                    Value::Class { .. } => Type::Unknown, // Classes are dynamic for now
                    Value::Instance { .. } => Type::Unknown, // Instances are dynamic for now
                    Value::Function { .. } => Type::Function {
                        params: vec![Type::Unknown],
                        return_type: Box::new(Type::Unknown),
                    },
                    Value::BuiltinFunction(_) => Type::Function {
                        params: vec![Type::Unknown],
                        return_type: Box::new(Type::Unknown),
                    },
                })
            }
            Expr::Identifier { name, .. } => {
                if let Some(var_type) = self.lookup_variable(name) {
                    Ok(var_type.clone())
                } else {
                    let available_vars = self.get_available_variable_names();
                    if let Some(suggestion) =
                        CustomLangError::find_similar_name(name, &available_vars)
                    {
                        self.errors
                            .push(CustomLangError::undefined_variable_with_suggestion(
                                name,
                                format!("Did you mean '{suggestion}'?"),
                            ));
                    } else {
                        self.errors.push(CustomLangError::undefined_variable_with_suggestion(
                            name,
                            format!("Variable '{name}' is not defined. Use 'let {name} = value;' to declare it.")
                        ));
                    }
                    Ok(Type::Unknown)
                }
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let left_type = self.analyze_expression(left)?;
                let right_type = self.analyze_expression(right)?;

                match op {
                    BinaryOp::Add => {
                        if matches!(left_type, Type::Unknown) || matches!(right_type, Type::Unknown)
                        {
                            Ok(Type::Unknown) // Allow unknown types for now
                        } else if matches!(left_type, Type::String)
                            || matches!(right_type, Type::String)
                        {
                            Ok(Type::String) // String concatenation
                        } else if matches!(left_type, Type::Number)
                            && matches!(right_type, Type::Number)
                        {
                            Ok(Type::Number)
                        } else {
                            self.errors.push(CustomLangError::SemanticError {
                                message: format!(
                                    "Invalid operands for +: {left_type} and {right_type}"
                                ),
                            });
                            Ok(Type::Unknown)
                        }
                    }
                    BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo => {
                        if matches!(left_type, Type::Unknown) || matches!(right_type, Type::Unknown)
                        {
                            Ok(Type::Unknown) // Allow unknown types for now
                        } else if matches!(left_type, Type::Number)
                            && matches!(right_type, Type::Number)
                        {
                            Ok(Type::Number)
                        } else {
                            self.errors.push(CustomLangError::SemanticError {
                                message: format!(
                                    "Invalid operands for {op:?}: {left_type} and {right_type}"
                                ),
                            });
                            Ok(Type::Unknown)
                        }
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => Ok(Type::Boolean),
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if matches!(left_type, Type::Unknown)
                            || matches!(right_type, Type::Unknown)
                            || (matches!(left_type, Type::Number)
                                && matches!(right_type, Type::Number))
                        {
                            Ok(Type::Boolean)
                        } else {
                            self.errors.push(CustomLangError::SemanticError {
                                message: format!(
                                    "Invalid operands for {op:?}: {left_type} and {right_type}"
                                ),
                            });
                            Ok(Type::Boolean)
                        }
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if matches!(left_type, Type::Unknown)
                            || matches!(right_type, Type::Unknown)
                            || (matches!(left_type, Type::Boolean)
                                && matches!(right_type, Type::Boolean))
                        {
                            Ok(Type::Boolean)
                        } else {
                            self.errors.push(CustomLangError::SemanticError {
                                message: format!(
                                    "Invalid operands for {op:?}: {left_type} and {right_type}"
                                ),
                            });
                            Ok(Type::Boolean)
                        }
                    }
                }
            }
            Expr::Unary { op, expr, .. } => {
                let operand_type = self.analyze_expression(expr)?;
                match op {
                    UnaryOp::Not => {
                        if matches!(operand_type, Type::Unknown)
                            || matches!(operand_type, Type::Boolean)
                        {
                            Ok(Type::Boolean)
                        } else {
                            self.errors.push(CustomLangError::SemanticError {
                                message: format!("Invalid operand for !: {operand_type}"),
                            });
                            Ok(Type::Boolean)
                        }
                    }
                    UnaryOp::Minus => {
                        if matches!(operand_type, Type::Unknown)
                            || matches!(operand_type, Type::Number)
                        {
                            Ok(Type::Number)
                        } else {
                            self.errors.push(CustomLangError::SemanticError {
                                message: format!("Invalid operand for -: {operand_type}"),
                            });
                            Ok(Type::Number)
                        }
                    }
                }
            }
            Expr::Call { callee, args, .. } => {
                // For now, we'll assume callee is an identifier
                if let Expr::Identifier { name, .. } = callee.as_ref() {
                    // Clone the function type to avoid borrowing issues
                    let func_type_opt = self.lookup_function(name).cloned();

                    if let Some(func_type) = func_type_opt {
                        if let Type::Function {
                            params,
                            return_type,
                        } = func_type
                        {
                            // Check argument count
                            if args.len() != params.len()
                                && !matches!(params.first(), Some(Type::Unknown))
                            {
                                self.errors.push(CustomLangError::SemanticError {
                                    message: format!(
                                        "Function '{}' expects {} arguments, got {}",
                                        name,
                                        params.len(),
                                        args.len()
                                    ),
                                });
                            }

                            // Analyze argument types
                            for arg in args {
                                self.analyze_expression(arg)?;
                            }

                            Ok((*return_type).clone())
                        } else {
                            self.errors.push(CustomLangError::SemanticError {
                                message: format!("'{name}' is not a function"),
                            });
                            Ok(Type::Unknown)
                        }
                    } else {
                        self.errors.push(CustomLangError::UndefinedFunction {
                            name: name.clone(),
                            suggestion: None,
                        });
                        Ok(Type::Unknown)
                    }
                } else {
                    // Handle non-identifier callees
                    Ok(Type::Unknown)
                }
            }
            Expr::Assignment { .. } => {
                // This should be handled in statement analysis
                Ok(Type::Unknown)
            }
            Expr::Array { elements, .. } => {
                // Analyze all elements
                for element in elements {
                    self.analyze_expression(element)?;
                }
                Ok(Type::Unknown) // Arrays are dynamic for now
            }
            Expr::Object { pairs, .. } => {
                // Analyze all property values
                for (_, value_expr) in pairs {
                    self.analyze_expression(value_expr)?;
                }
                Ok(Type::Unknown) // Objects are dynamic for now
            }
            Expr::Index { object, index, .. } => {
                self.analyze_expression(object)?;
                self.analyze_expression(index)?;
                Ok(Type::Unknown) // Index results are dynamic for now
            }
            Expr::New {
                class_name: _,
                args,
                ..
            } => {
                // Validate that the class exists (simplified for now)
                // In a full implementation, we'd check the class definition
                for arg in args {
                    self.analyze_expression(arg)?;
                }
                Ok(Type::Unknown) // Class instances are dynamic for now
            }
            Expr::This { .. } => {
                // For now, just return unknown type
                // In a full implementation, we'd check if we're inside a class method
                Ok(Type::Unknown)
            }
            Expr::PropertyAccess {
                object,
                property: _,
                ..
            } => {
                self.analyze_expression(object)?;
                Ok(Type::Unknown) // Property access results are dynamic for now
            }
            Expr::Match { expr, arms, .. } => {
                self.analyze_expression(expr)?;

                // Analyze all match arms
                for arm in arms {
                    self.analyze_expression(&arm.body)?;
                }

                Ok(Type::Unknown) // Match results are dynamic for now
            }
        }
    }

    fn types_compatible(&self, actual: &Type, expected: &Type) -> bool {
        matches!(
            (actual, expected),
            (Type::Unknown, _)
                | (_, Type::Unknown)
                | (Type::Number, Type::Number)
                | (Type::String, Type::String)
                | (Type::Boolean, Type::Boolean)
                | (Type::Null, Type::Null)
        )
    }

    /// Get all available variable names for error suggestions
    fn get_available_variable_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for scope in &self.scopes {
            names.extend(scope.variables.keys().cloned());
        }
        names
    }
}
