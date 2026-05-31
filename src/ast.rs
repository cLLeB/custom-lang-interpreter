use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::env::EnvRef;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
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

// ─── VALUES ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
    Null,
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<HashMap<String, Value>>>),
    Function(Rc<FnData>),
    Builtin(String),
    Class(Rc<ClassData>),
    Instance(Rc<RefCell<InstanceData>>),
    Generator(Rc<RefCell<GeneratorState>>),
    EnumVariant(Rc<EnumVariantData>),
}

/// A single enum member, e.g. `Color.Red`. Carries its owning enum's name, its
/// own variant name, its assigned value (ordinal by default, or an explicit
/// value), and its declaration-order ordinal. Displays as the bare variant name.
#[derive(Debug)]
pub struct EnumVariantData {
    pub enum_name: String,
    pub name: String,
    pub value: Value,
    pub ordinal: i64,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
    pub is_rest: bool,
}
impl Param {
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default: None,
            is_rest: false,
        }
    }
    #[allow(dead_code)]
    pub fn with_default(name: impl Into<String>, default: Expr) -> Self {
        Self {
            name: name.into(),
            default: Some(default),
            is_rest: false,
        }
    }
    #[allow(dead_code)]
    pub fn rest(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default: None,
            is_rest: true,
        }
    }
}

#[derive(Debug)]
pub struct FnData {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Box<Stmt>,
    pub closure: EnvRef,
    pub is_generator: bool,
    #[allow(dead_code)]
    pub is_async: bool,
}

#[derive(Debug)]
pub struct ClassData {
    pub name: String,
    pub methods: HashMap<String, Rc<FnData>>,
    pub static_methods: HashMap<String, Rc<FnData>>,
    pub static_fields: HashMap<String, Value>,
    pub getters: HashMap<String, Rc<FnData>>,
    pub setters: HashMap<String, Rc<FnData>>,
    pub superclass: Option<Rc<ClassData>>,
}

#[derive(Debug)]
pub struct InstanceData {
    pub class: Rc<ClassData>,
    pub fields: HashMap<String, Value>,
}

#[derive(Debug)]
pub struct GeneratorState {
    pub values: Vec<Value>,
    pub index: usize,
    pub done: bool,
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
            Value::Generator(_) => "generator",
            Value::EnumVariant(_) => "enum",
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
            (Value::EnumVariant(a), Value::EnumVariant(b)) => {
                Rc::ptr_eq(a, b) || (a.enum_name == b.enum_name && a.name == b.name)
            }
            _ => false,
        }
    }
    pub fn make_array(elements: Vec<Value>) -> Value {
        Value::Array(Rc::new(RefCell::new(elements)))
    }
    pub fn make_object(pairs: HashMap<String, Value>) -> Value {
        Value::Object(Rc::new(RefCell::new(pairs)))
    }
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
                pairs.sort();
                write!(f, "{{{}}}", pairs.join(", "))
            }
            Value::Function(fd) => write!(f, "<function {}>", fd.name),
            Value::Builtin(name) => write!(f, "<builtin {name}>"),
            Value::Class(c) => write!(f, "<class {}>", c.name),
            Value::Instance(inst) => write!(f, "<{} instance>", inst.borrow().class.name),
            Value::Generator(_) => write!(f, "<generator>"),
            Value::EnumVariant(v) => write!(f, "{}", v.name),
        }
    }
}
impl Value {
    pub fn repr(&self) -> String {
        match self {
            Value::Str(s) => format!("\"{s}\""),
            other => other.to_string(),
        }
    }
}

// ─── AST NODES ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    ShiftRightU,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    NullCoalesce,
    In,
    Instanceof,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Minus,
    Not,
    BitwiseNot,
    Typeof,
}

#[derive(Debug, Clone)]
pub enum CompoundOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    NullCoalesce,
    LogicalOr,
    LogicalAnd,
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

/// Object literal key
#[derive(Debug, Clone)]
pub enum ObjectKey {
    Static(String),
    Computed(Box<Expr>),
}

/// Destructuring element in array destructuring
#[derive(Debug, Clone)]
pub enum DestructElem {
    Bind { name: String, default: Option<Expr> },
    Skip,
    Rest(String),
}

/// Object destructuring field
#[derive(Debug, Clone)]
pub struct DestructField {
    pub key: String,
    pub alias: Option<String>,
    pub default: Option<Expr>,
}

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
    Yield {
        value: Option<Box<Expr>>,
        pos: Position,
    },
    Await {
        expr: Box<Expr>,
        pos: Position,
    },
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
        pos: Position,
    },
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
            | Expr::Match { pos, .. }
            | Expr::Yield { pos, .. }
            | Expr::Await { pos, .. }
            | Expr::Pipe { pos, .. } => pos,
        }
    }
}

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
    LetDestructArray {
        elems: Vec<DestructElem>,
        init: Expr,
        pos: Position,
    },
    LetDestructObject {
        fields: Vec<DestructField>,
        init: Expr,
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
    ForOf {
        var: String,
        iter: Expr,
        body: Box<Stmt>,
        pos: Position,
    },
    Labeled {
        label: String,
        body: Box<Stmt>,
        pos: Position,
    },
    Function {
        name: String,
        params: Vec<Param>,
        body: Box<Stmt>,
        is_static: bool,
        is_generator: bool,
        is_async: bool,
        pos: Position,
    },
    Return {
        value: Option<Expr>,
        pos: Position,
    },
    Break {
        label: Option<String>,
        pos: Position,
    },
    Continue {
        label: Option<String>,
        pos: Position,
    },
    Print {
        expr: Expr,
        pos: Position,
    },
    Import {
        path: String,
        alias: Option<String>,
        names: Vec<(String, Option<String>)>,
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
    Enum {
        name: String,
        variants: Vec<(String, Option<Expr>)>,
        pos: Position,
    },
    TypeAlias {
        name: String,
        pos: Position,
    },
    Interface {
        name: String,
        pos: Position,
    },
    Decorator {
        name: String,
        target: Box<Stmt>,
        pos: Position,
    },
}

impl Stmt {
    #[allow(dead_code)]
    pub fn pos(&self) -> &Position {
        match self {
            Stmt::Expr { pos, .. }
            | Stmt::Let { pos, .. }
            | Stmt::LetDestructArray { pos, .. }
            | Stmt::LetDestructObject { pos, .. }
            | Stmt::Block { pos, .. }
            | Stmt::If { pos, .. }
            | Stmt::While { pos, .. }
            | Stmt::DoWhile { pos, .. }
            | Stmt::For { pos, .. }
            | Stmt::ForIn { pos, .. }
            | Stmt::ForOf { pos, .. }
            | Stmt::Labeled { pos, .. }
            | Stmt::Function { pos, .. }
            | Stmt::Return { pos, .. }
            | Stmt::Break { pos, .. }
            | Stmt::Continue { pos, .. }
            | Stmt::Print { pos, .. }
            | Stmt::Import { pos, .. }
            | Stmt::Export { pos, .. }
            | Stmt::Class { pos, .. }
            | Stmt::TryCatch { pos, .. }
            | Stmt::Throw { pos, .. }
            | Stmt::Enum { pos, .. }
            | Stmt::TypeAlias { pos, .. }
            | Stmt::Interface { pos, .. }
            | Stmt::Decorator { pos, .. } => pos,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}
impl Program {
    pub fn new(stmts: Vec<Stmt>) -> Self {
        Self { stmts }
    }
}
