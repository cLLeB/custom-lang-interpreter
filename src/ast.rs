use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::env::EnvRef;

/// Source location for error reporting
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

// ─────────────────────────────── VALUES ─────────────────────────────────────

/// All runtime values produced by the interpreter
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
    Null,
    /// Arrays are heap-allocated and shared by reference, enabling mutation
    Array(Rc<RefCell<Vec<Value>>>),
    /// Objects likewise share state
    Object(Rc<RefCell<HashMap<String, Value>>>),
    Function(Rc<FnData>),
    Builtin(String),
    Class(Rc<ClassData>),
    Instance(Rc<RefCell<InstanceData>>),
}

/// A single function parameter (name + optional default + rest flag)
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
    pub is_rest: bool,
}

impl Param {
    pub fn simple(name: impl Into<String>) -> Self {
        Self { name: name.into(), default: None, is_rest: false }
    }
    #[allow(dead_code)]
    pub fn with_default(name: impl Into<String>, default: Expr) -> Self {
        Self { name: name.into(), default: Some(default), is_rest: false }
    }
    #[allow(dead_code)]
    pub fn rest(name: impl Into<String>) -> Self {
        Self { name: name.into(), default: None, is_rest: true }
    }
}

#[derive(Debug)]
pub struct FnData {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Box<Stmt>,
    /// The captured environment at the time the function was defined
    pub closure: EnvRef,
}

#[derive(Debug)]
pub struct ClassData {
    pub name: String,
    pub methods: HashMap<String, Rc<FnData>>,
    pub static_methods: HashMap<String, Rc<FnData>>,
    pub static_fields: HashMap<String, Value>,
    pub superclass: Option<Rc<ClassData>>,
}

#[derive(Debug)]
pub struct InstanceData {
    pub class: Rc<ClassData>,
    pub fields: HashMap<String, Value>,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Str(_) => "string",
            Value::Bool(_) => "boolean",
            Value::Null => "null",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function(_) | Value::Builtin(_) => "function",
            Value::Class(_) => "class",
            Value::Instance(_) => "instance",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Array(a) => !a.borrow().is_empty(),
            Value::Object(o) => !o.borrow().is_empty(),
            _ => true,
        }
    }

    /// Value equality (structural for primitives/arrays, reference for objects/instances)
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Array(a), Value::Array(b)) => {
                if Rc::ptr_eq(a, b) {
                    return true;
                }
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y))
            }
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Instance(a), Value::Instance(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    /// Make an empty mutable array
    pub fn make_array(elements: Vec<Value>) -> Value {
        Value::Array(Rc::new(RefCell::new(elements)))
    }

    /// Make an empty mutable object
    pub fn make_object(pairs: HashMap<String, Value>) -> Value {
        Value::Object(Rc::new(RefCell::new(pairs)))
    }

    /// Check if this value is an instance of the named class
    pub fn is_instance_of(&self, class_name: &str) -> bool {
        if let Value::Instance(inst) = self {
            let inst = inst.borrow();
            let mut cls = Some(Rc::clone(&inst.class));
            while let Some(c) = cls {
                if c.name == class_name {
                    return true;
                }
                cls = c.superclass.clone();
            }
        }
        false
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            Value::Str(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Null => write!(f, "null"),
            Value::Array(arr) => {
                let arr = arr.borrow();
                let parts: Vec<String> = arr.iter().map(|v| v.repr()).collect();
                write!(f, "[{}]", parts.join(", "))
            }
            Value::Object(obj) => {
                let obj = obj.borrow();
                let mut pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.repr()))
                    .collect();
                pairs.sort(); // deterministic output
                write!(f, "{{{}}}", pairs.join(", "))
            }
            Value::Function(fd) => write!(f, "<function {}>", fd.name),
            Value::Builtin(name) => write!(f, "<builtin {name}>"),
            Value::Class(c) => write!(f, "<class {}>", c.name),
            Value::Instance(inst) => {
                let inst = inst.borrow();
                write!(f, "<{} instance>", inst.class.name)
            }
        }
    }
}

impl Value {
    /// Like Display but wraps strings in quotes — used for array/object display
    pub fn repr(&self) -> String {
        match self {
            Value::Str(s) => format!("\"{s}\""),
            other => other.to_string(),
        }
    }
}

// ─────────────────────────────── AST NODES ──────────────────────────────────

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,          // **
    BitwiseAnd,     // &
    BitwiseOr,      // |
    BitwiseXor,     // ^
    ShiftLeft,      // <<
    ShiftRight,     // >>
    ShiftRightU,    // >>> (unsigned)
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    NullCoalesce,   // ??
    In,             // in
    Instanceof,     // instanceof
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Minus,
    Not,
    BitwiseNot,     // ~
    Typeof,         // typeof
}

#[derive(Debug, Clone)]
pub enum CompoundOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,          // **=
    BitAnd,         // &=
    BitOr,          // |=
    BitXor,         // ^=
    ShiftLeft,      // <<=
    ShiftRight,     // >>=
    NullCoalesce,   // ??=
    LogicalOr,      // ||=
    LogicalAnd,     // &&=
}

impl CompoundOp {
    pub fn to_binary(&self) -> BinaryOp {
        match self {
            CompoundOp::Add => BinaryOp::Add,
            CompoundOp::Subtract => BinaryOp::Subtract,
            CompoundOp::Multiply => BinaryOp::Multiply,
            CompoundOp::Divide => BinaryOp::Divide,
            CompoundOp::Modulo => BinaryOp::Modulo,
            CompoundOp::Power => BinaryOp::Power,
            CompoundOp::BitAnd => BinaryOp::BitwiseAnd,
            CompoundOp::BitOr => BinaryOp::BitwiseOr,
            CompoundOp::BitXor => BinaryOp::BitwiseXor,
            CompoundOp::ShiftLeft => BinaryOp::ShiftLeft,
            CompoundOp::ShiftRight => BinaryOp::ShiftRight,
            CompoundOp::NullCoalesce => BinaryOp::NullCoalesce,
            CompoundOp::LogicalOr => BinaryOp::Or,
            CompoundOp::LogicalAnd => BinaryOp::And,
        }
    }
}

/// Pattern nodes for match expressions
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Number(f64),
    Str(String),
    Bool(bool),
    Null,
    Wildcard,
    Binding(String),
    Array(Vec<Pattern>),
    Object(Vec<(String, Pattern)>),
}

/// Expression AST nodes
#[derive(Debug, Clone)]
pub enum Expr {
    Literal {
        value: Value,
        pos: Position,
    },
    Var {
        name: String,
        pos: Position,
    },
    Assign {
        name: String,
        value: Box<Expr>,
        pos: Position,
    },
    CompoundAssign {
        name: String,
        op: CompoundOp,
        value: Box<Expr>,
        pos: Position,
    },
    IndexAssign {
        object: Box<Expr>,
        index: Box<Expr>,
        value: Box<Expr>,
        pos: Position,
    },
    PropAssign {
        object: Box<Expr>,
        prop: String,
        value: Box<Expr>,
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
    Ternary {
        cond: Box<Expr>,
        then_e: Box<Expr>,
        else_e: Box<Expr>,
        pos: Position,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        pos: Position,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        pos: Position,
    },
    Prop {
        object: Box<Expr>,
        name: String,
        pos: Position,
    },
    OptionalProp {
        object: Box<Expr>,
        name: String,
        pos: Position,
    },
    OptionalIndex {
        object: Box<Expr>,
        index: Box<Expr>,
        pos: Position,
    },
    OptionalCall {
        callee: Box<Expr>,
        args: Vec<Expr>,
        pos: Position,
    },
    Array {
        elements: Vec<Expr>,
        pos: Position,
    },
    Object {
        pairs: Vec<(ObjectKey, Expr)>,
        pos: Position,
    },
    Spread {
        expr: Box<Expr>,
        pos: Position,
    },
    New {
        class: Box<Expr>,
        args: Vec<Expr>,
        pos: Position,
    },
    This {
        pos: Position,
    },
    Super {
        pos: Position,
    },
    Lambda {
        params: Vec<Param>,
        body: Box<Stmt>,
        pos: Position,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        pos: Position,
    },
}

/// Object literal key — static string or computed expression
#[derive(Debug, Clone)]
pub enum ObjectKey {
    Static(String),
    Computed(Box<Expr>),
}

impl Expr {
    pub fn pos(&self) -> &Position {
        match self {
            Expr::Literal { pos, .. }
            | Expr::Var { pos, .. }
            | Expr::Assign { pos, .. }
            | Expr::CompoundAssign { pos, .. }
            | Expr::IndexAssign { pos, .. }
            | Expr::PropAssign { pos, .. }
            | Expr::Binary { pos, .. }
            | Expr::Unary { pos, .. }
            | Expr::Ternary { pos, .. }
            | Expr::Call { pos, .. }
            | Expr::Index { pos, .. }
            | Expr::Prop { pos, .. }
            | Expr::OptionalProp { pos, .. }
            | Expr::OptionalIndex { pos, .. }
            | Expr::OptionalCall { pos, .. }
            | Expr::Array { pos, .. }
            | Expr::Object { pos, .. }
            | Expr::Spread { pos, .. }
            | Expr::New { pos, .. }
            | Expr::This { pos }
            | Expr::Super { pos }
            | Expr::Lambda { pos, .. }
            | Expr::Match { pos, .. } => pos,
        }
    }
}

/// Statement AST nodes
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Stmt {
    Expr {
        expr: Expr,
        pos: Position,
    },
    Let {
        name: String,
        init: Option<Expr>,
        pos: Position,
    },
    Block {
        stmts: Vec<Stmt>,
        pos: Position,
    },
    If {
        cond: Expr,
        then_b: Box<Stmt>,
        else_b: Option<Box<Stmt>>,
        pos: Position,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
        pos: Position,
    },
    DoWhile {
        body: Box<Stmt>,
        cond: Expr,
        pos: Position,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
        pos: Position,
    },
    ForIn {
        var: String,
        iter: Expr,
        body: Box<Stmt>,
        pos: Position,
    },
    Function {
        name: String,
        params: Vec<Param>,
        body: Box<Stmt>,
        is_static: bool,
        pos: Position,
    },
    Return {
        value: Option<Expr>,
        pos: Position,
    },
    Break {
        pos: Position,
    },
    Continue {
        pos: Position,
    },
    Print {
        expr: Expr,
        pos: Position,
    },
    Import {
        path: String,
        alias: Option<String>,
        pos: Position,
    },
    Export {
        name: String,
        pos: Position,
    },
    Class {
        name: String,
        super_name: Option<String>,
        methods: Vec<Stmt>,
        pos: Position,
    },
    TryCatch {
        try_b: Box<Stmt>,
        catch_var: Option<String>,
        catch_b: Option<Box<Stmt>>,
        finally_b: Option<Box<Stmt>>,
        pos: Position,
    },
    Throw {
        value: Expr,
        pos: Position,
    },
}

impl Stmt {
    #[allow(dead_code)]
    pub fn pos(&self) -> &Position {
        match self {
            Stmt::Expr { pos, .. }
            | Stmt::Let { pos, .. }
            | Stmt::Block { pos, .. }
            | Stmt::If { pos, .. }
            | Stmt::While { pos, .. }
            | Stmt::DoWhile { pos, .. }
            | Stmt::For { pos, .. }
            | Stmt::ForIn { pos, .. }
            | Stmt::Function { pos, .. }
            | Stmt::Return { pos, .. }
            | Stmt::Break { pos }
            | Stmt::Continue { pos }
            | Stmt::Print { pos, .. }
            | Stmt::Import { pos, .. }
            | Stmt::Export { pos, .. }
            | Stmt::Class { pos, .. }
            | Stmt::TryCatch { pos, .. }
            | Stmt::Throw { pos, .. } => pos,
        }
    }
}

/// Top-level program
#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

impl Program {
    pub fn new(stmts: Vec<Stmt>) -> Self {
        Self { stmts }
    }
}
