//! # Abstract Syntax Tree (AST) Definitions
//!
//! This module defines the core data structures that represent the parsed structure
//! of Custom Language programs. The AST serves as the intermediate representation
//! between the parser and the interpreter.
//!
//! ## Key Components
//!
//! - **Position**: Source location tracking for error reporting
//! - **Value**: Runtime values (numbers, strings, booleans, arrays, functions)
//! - **Expr**: Expression nodes (literals, variables, operations, function calls)
//! - **Stmt**: Statement nodes (declarations, control flow, function definitions)
//! - **Program**: Top-level container for a complete program
//! - **Environment**: Variable and function scope management

use std::collections::HashMap;

/// Position information for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Data types supported by the language
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Class {
        name: String,
        methods: HashMap<String, Value>, // Method name -> Function value
        superclass: Option<Box<Value>>,
    },
    Instance {
        class_name: String,
        fields: HashMap<String, Value>,
        methods: HashMap<String, Value>,
    },
    Function {
        name: String,
        params: Vec<String>,
        body: Box<crate::ast::Stmt>,
        closure: Environment,
    },
    BuiltinFunction(String),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Boolean(_) => "boolean",
            Value::Null => "null",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Class { .. } => "class",
            Value::Instance { .. } => "instance",
            Value::Function { .. } => "function",
            Value::BuiltinFunction(_) => "builtin_function",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Null => false,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
            Value::Class { .. } => true,
            Value::Instance { .. } => true,
            Value::Function { .. } => true,
            Value::BuiltinFunction(_) => true,
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Minus,
    Not,
}

/// Pattern matching structures
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Value),
    Variable(String),
    Wildcard,
    Array(Vec<Pattern>),
    Object(Vec<(String, Pattern)>),
}

/// Expression AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        value: Value,
        pos: Position,
    },
    Identifier {
        name: String,
        pos: Position,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        pos: Position,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        pos: Position,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        pos: Position,
    },
    Assignment {
        name: String,
        value: Box<Expr>,
        pos: Position,
    },
    Array {
        elements: Vec<Expr>,
        pos: Position,
    },
    Object {
        pairs: Vec<(String, Expr)>,
        pos: Position,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        pos: Position,
    },
    New {
        class_name: String,
        args: Vec<Expr>,
        pos: Position,
    },
    This {
        pos: Position,
    },
    PropertyAccess {
        object: Box<Expr>,
        property: String,
        pos: Position,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        pos: Position,
    },
}

impl Expr {
    pub fn position(&self) -> &Position {
        match self {
            Expr::Literal { pos, .. }
            | Expr::Identifier { pos, .. }
            | Expr::Binary { pos, .. }
            | Expr::Unary { pos, .. }
            | Expr::Call { pos, .. }
            | Expr::Assignment { pos, .. }
            | Expr::Array { pos, .. }
            | Expr::Object { pos, .. }
            | Expr::Index { pos, .. }
            | Expr::New { pos, .. }
            | Expr::This { pos, .. }
            | Expr::PropertyAccess { pos, .. }
            | Expr::Match { pos, .. } => pos,
        }
    }
}

/// Statement AST nodes
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expression {
        expr: Expr,
        pos: Position,
    },
    VarDeclaration {
        name: String,
        initializer: Option<Expr>,
        pos: Position,
    },
    Block {
        statements: Vec<Stmt>,
        pos: Position,
    },
    If {
        condition: Expr,
        then_stmt: Box<Stmt>,
        else_stmt: Option<Box<Stmt>>,
        pos: Position,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
        pos: Position,
    },
    Function {
        name: String,
        params: Vec<String>,
        body: Box<Stmt>,
        pos: Position,
    },
    Return {
        value: Option<Expr>,
        pos: Position,
    },
    Import {
        module_path: String,
        alias: Option<String>,
        pos: Position,
    },
    Export {
        name: String,
        pos: Position,
    },
    Class {
        name: String,
        superclass: Option<String>,
        methods: Vec<Stmt>, // Function statements
        pos: Position,
    },
    Print {
        expr: Expr,
        pos: Position,
    },
}

impl Stmt {
    #[allow(dead_code)]
    pub fn position(&self) -> &Position {
        match self {
            Stmt::Expression { pos, .. }
            | Stmt::VarDeclaration { pos, .. }
            | Stmt::Block { pos, .. }
            | Stmt::If { pos, .. }
            | Stmt::While { pos, .. }
            | Stmt::Function { pos, .. }
            | Stmt::Return { pos, .. }
            | Stmt::Import { pos, .. }
            | Stmt::Export { pos, .. }
            | Stmt::Class { pos, .. }
            | Stmt::Print { pos, .. } => pos,
        }
    }
}

/// Program is a collection of statements
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

impl Program {
    pub fn new(statements: Vec<Stmt>) -> Self {
        Self { statements }
    }
}

/// Environment for variable and function storage
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    pub variables: HashMap<String, Value>,
    pub parent: Option<Box<Environment>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Environment) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(name)))
    }

    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            true
        } else if let Some(parent) = &mut self.parent {
            parent.assign(name, value)
        } else {
            false
        }
    }
}
