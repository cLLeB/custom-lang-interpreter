//! Single-pass compiler lowering the AST to [`crate::bytecode`].
//!
//! Covers the VM's core sub-language. Anything outside it (classes, enums,
//! collections, try/catch, modules, generators, closures that capture an
//! enclosing function's locals, …) returns a [`CompileError`] pointing at the
//! offending source position — the target never silently mis-compiles.

use crate::ast::{BinaryOp, Expr, Param, Position, Program, Stmt, UnaryOp, Value};
use crate::bytecode::{Chunk, Constant, Function, Op};
use std::rc::Rc;

#[derive(Debug)]
pub struct CompileError {
    pub line: usize,
    pub column: usize,
    pub msg: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "bytecode compile error at {}:{}: {}",
            self.line, self.column, self.msg
        )
    }
}

type CResult<T> = Result<T, CompileError>;

struct Local {
    name: String,
    depth: i32,
}

struct LoopCtx {
    /// Backward jump target for `continue` — the condition (while) or the update
    /// clause (for). `None` when the continue site sits *ahead* of the body
    /// (do-while), in which case `continue` emits a forward jump recorded in
    /// `continues` and patched to the condition.
    continue_target: Option<usize>,
    /// Forward `Jump` placeholders for `continue` to patch (do-while only).
    continues: Vec<usize>,
    /// Forward `Jump` placeholders to patch to the loop exit, for `break`.
    breaks: Vec<usize>,
    /// Scope depth at which the loop body opens (for popping locals on exit).
    scope_depth: i32,
}

struct FnState {
    func: Function,
    locals: Vec<Local>,
    scope_depth: i32,
    is_script: bool,
    loops: Vec<LoopCtx>,
}

impl FnState {
    fn new(name: &str, is_script: bool) -> Self {
        // Slot 0 is reserved for the callee (the function itself at runtime).
        FnState {
            func: Function {
                name: name.to_string(),
                arity: 0,
                chunk: Chunk::new(),
            },
            locals: vec![Local {
                name: String::new(),
                depth: 0,
            }],
            scope_depth: 0,
            is_script,
            loops: Vec::new(),
        }
    }

    fn here(&self) -> usize {
        self.func.chunk.code.len()
    }

    fn emit(&mut self, op: Op, line: usize) {
        self.func.chunk.emit(op, line as u32);
    }

    fn emit_byte(&mut self, b: u8, line: usize) {
        self.func.chunk.emit_byte(b, line as u32);
    }

    fn emit_u16(&mut self, v: u16, line: usize) {
        self.func.chunk.emit_u16(v, line as u32);
    }

    fn constant(&mut self, c: Constant) -> usize {
        self.func.chunk.add_constant(c)
    }

    /// Emit a jump opcode with a placeholder operand; returns the operand offset
    /// to be patched later.
    fn emit_jump(&mut self, op: Op, line: usize) -> usize {
        self.emit(op, line);
        let at = self.here();
        self.emit_u16(0xffff, line);
        at
    }

    fn patch_jump(&mut self, at: usize, err_pos: Position) -> CResult<()> {
        let target = self.here();
        let jump = target - (at + 2);
        if jump > u16::MAX as usize {
            return Err(CompileError {
                line: err_pos.line,
                column: err_pos.column,
                msg: "jump too large for bytecode (function too big)".to_string(),
            });
        }
        self.func.chunk.code[at] = (jump & 0xff) as u8;
        self.func.chunk.code[at + 1] = (jump >> 8) as u8;
        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize, line: usize, err_pos: Position) -> CResult<()> {
        self.emit(Op::Loop, line);
        let offset = self.here() - loop_start + 2;
        if offset > u16::MAX as usize {
            return Err(CompileError {
                line: err_pos.line,
                column: err_pos.column,
                msg: "loop body too large for bytecode".to_string(),
            });
        }
        self.emit_u16(offset as u16, line);
        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Close the innermost scope, emitting `Pop` for each local it held.
    fn end_scope(&mut self, line: usize) {
        self.scope_depth -= 1;
        while let Some(l) = self.locals.last() {
            if l.depth > self.scope_depth {
                self.emit(Op::Pop, line);
                self.locals.pop();
            } else {
                break;
            }
        }
    }

    fn add_local(&mut self, name: &str) {
        self.locals.push(Local {
            name: name.to_string(),
            depth: self.scope_depth,
        });
    }

    fn resolve_local(&self, name: &str) -> Option<usize> {
        // Skip slot 0 (reserved). Search inner-to-outer.
        self.locals
            .iter()
            .enumerate()
            .skip(1)
            .rev()
            .find(|(_, l)| l.name == name)
            .map(|(i, _)| i)
    }

    /// True when a `let`/`function` at the current position binds a global.
    fn is_global_scope(&self) -> bool {
        self.is_script && self.scope_depth == 0
    }
}

/// Compile a whole program into the top-level `<main>` function.
pub fn compile_program(program: &Program) -> CResult<Function> {
    let mut s = FnState::new("<main>", true);
    for stmt in &program.stmts {
        compile_stmt(&mut s, stmt)?;
    }
    let last = program.stmts.last().map(|st| st.pos().line).unwrap_or(0);
    s.emit(Op::Null, last);
    s.emit(Op::Return, last);
    Ok(s.func)
}

fn unsupported<T>(pos: &Position, what: &str) -> CResult<T> {
    Err(CompileError {
        line: pos.line,
        column: pos.column,
        msg: format!("{what} is not yet supported by the bytecode target"),
    })
}

fn compile_stmt(s: &mut FnState, stmt: &Stmt) -> CResult<()> {
    match stmt {
        Stmt::Expr { expr, .. } => {
            compile_expr(s, expr)?;
            s.emit(Op::Pop, expr.pos().line);
            Ok(())
        }
        Stmt::Print { expr, pos } => {
            compile_expr(s, expr)?;
            s.emit(Op::Print, pos.line);
            Ok(())
        }
        Stmt::Let {
            name, init, pos, ..
        } => {
            match init {
                Some(e) => compile_expr(s, e)?,
                None => s.emit(Op::Null, pos.line),
            }
            define_variable(s, name, pos);
            Ok(())
        }
        Stmt::Block { stmts, pos } => {
            s.begin_scope();
            for st in stmts {
                compile_stmt(s, st)?;
            }
            s.end_scope(pos.line);
            Ok(())
        }
        Stmt::If {
            cond,
            then_b,
            else_b,
            pos,
        } => compile_if(s, cond, then_b, else_b.as_deref(), pos),
        Stmt::While { cond, body, pos } => compile_while(s, cond, body, pos),
        Stmt::DoWhile { body, cond, pos } => compile_do_while(s, body, cond, pos),
        Stmt::For {
            init,
            cond,
            update,
            body,
            pos,
        } => compile_for(
            s,
            init.as_deref(),
            cond.as_ref(),
            update.as_ref(),
            body,
            pos,
        ),
        Stmt::Function {
            name,
            params,
            body,
            is_generator,
            is_async,
            pos,
            ..
        } => {
            if *is_generator {
                return unsupported(pos, "generator functions");
            }
            if *is_async {
                return unsupported(pos, "async functions");
            }
            let func = compile_function(name, params, body, pos)?;
            let idx = s.constant(Constant::Function(Rc::new(func)));
            emit_constant(s, idx, pos.line);
            define_variable(s, name, pos);
            Ok(())
        }
        Stmt::Return { value, pos } => {
            match value {
                Some(e) => compile_expr(s, e)?,
                None => s.emit(Op::Null, pos.line),
            }
            s.emit(Op::Return, pos.line);
            Ok(())
        }
        Stmt::Break {
            label: Some(_),
            pos,
        } => unsupported(pos, "labeled break"),
        Stmt::Continue {
            label: Some(_),
            pos,
        } => unsupported(pos, "labeled continue"),
        Stmt::Break { pos, .. } => compile_break(s, pos),
        Stmt::Continue { pos, .. } => compile_continue(s, pos),
        other => unsupported(other.pos(), stmt_kind(other)),
    }
}

fn stmt_kind(stmt: &Stmt) -> &'static str {
    match stmt {
        Stmt::Class { .. } => "classes",
        Stmt::Enum { .. } => "enums",
        Stmt::TryCatch { .. } => "try/catch",
        Stmt::Throw { .. } => "throw",
        Stmt::Import { .. } | Stmt::Export { .. } => "modules",
        Stmt::ForIn { .. } | Stmt::ForOf { .. } => "for-in/for-of loops",
        Stmt::LetDestructArray { .. } | Stmt::LetDestructObject { .. } => "destructuring",
        Stmt::Labeled { .. } => "labeled statements",
        _ => "this statement",
    }
}

fn define_variable(s: &mut FnState, name: &str, pos: &Position) {
    if s.is_global_scope() {
        let idx = s.constant(Constant::Str(Rc::from(name)));
        s.emit(Op::DefineGlobal, pos.line);
        s.emit_u16(idx as u16, pos.line);
    } else {
        // Value is already on the stack; it becomes this local's slot.
        s.add_local(name);
    }
}

fn emit_constant(s: &mut FnState, idx: usize, line: usize) {
    s.emit(Op::Constant, line);
    s.emit_u16(idx as u16, line);
}

fn compile_function(
    name: &str,
    params: &[Param],
    body: &Stmt,
    pos: &Position,
) -> CResult<Function> {
    let mut fs = FnState::new(name, false);
    fs.func.arity = params.len();
    fs.begin_scope();
    for p in params {
        if p.is_rest {
            return unsupported(pos, "rest parameters");
        }
        if p.default.is_some() {
            return unsupported(pos, "default parameters in the bytecode target");
        }
        fs.add_local(&p.name);
    }
    // A function body is a Block; compile its statements in the param scope.
    match body {
        Stmt::Block { stmts, .. } => {
            for st in stmts {
                compile_stmt(&mut fs, st)?;
            }
        }
        single => compile_stmt(&mut fs, single)?,
    }
    // Implicit `return null` if control falls off the end.
    fs.emit(Op::Null, pos.line);
    fs.emit(Op::Return, pos.line);
    Ok(fs.func)
}

fn compile_if(
    s: &mut FnState,
    cond: &Expr,
    then_b: &Stmt,
    else_b: Option<&Stmt>,
    pos: &Position,
) -> CResult<()> {
    compile_expr(s, cond)?;
    let then_jump = s.emit_jump(Op::JumpIfFalse, pos.line);
    s.emit(Op::Pop, pos.line);
    compile_stmt(s, then_b)?;
    let else_jump = s.emit_jump(Op::Jump, pos.line);
    s.patch_jump(then_jump, *pos)?;
    s.emit(Op::Pop, pos.line);
    if let Some(eb) = else_b {
        compile_stmt(s, eb)?;
    }
    s.patch_jump(else_jump, *pos)?;
    Ok(())
}

fn compile_while(s: &mut FnState, cond: &Expr, body: &Stmt, pos: &Position) -> CResult<()> {
    let loop_start = s.here();
    compile_expr(s, cond)?;
    let exit_jump = s.emit_jump(Op::JumpIfFalse, pos.line);
    s.emit(Op::Pop, pos.line);
    s.loops.push(LoopCtx {
        continue_target: Some(loop_start),
        continues: Vec::new(),
        breaks: Vec::new(),
        scope_depth: s.scope_depth,
    });
    compile_stmt(s, body)?;
    s.emit_loop(loop_start, pos.line, *pos)?;
    s.patch_jump(exit_jump, *pos)?;
    s.emit(Op::Pop, pos.line);
    finish_loop(s, *pos)?;
    Ok(())
}

fn compile_do_while(s: &mut FnState, body: &Stmt, cond: &Expr, pos: &Position) -> CResult<()> {
    let loop_start = s.here();
    s.loops.push(LoopCtx {
        // `continue` jumps forward to the condition re-check (patched below).
        continue_target: None,
        continues: Vec::new(),
        breaks: Vec::new(),
        scope_depth: s.scope_depth,
    });
    compile_stmt(s, body)?;
    // The condition re-check is the `continue` landing point.
    let pending = std::mem::take(&mut s.loops.last_mut().unwrap().continues);
    for c in pending {
        s.patch_jump(c, *pos)?;
    }
    compile_expr(s, cond)?;
    let exit_jump = s.emit_jump(Op::JumpIfFalse, pos.line);
    s.emit(Op::Pop, pos.line);
    s.emit_loop(loop_start, pos.line, *pos)?;
    s.patch_jump(exit_jump, *pos)?;
    s.emit(Op::Pop, pos.line);
    finish_loop(s, *pos)?;
    Ok(())
}

fn compile_for(
    s: &mut FnState,
    init: Option<&Stmt>,
    cond: Option<&Expr>,
    update: Option<&Expr>,
    body: &Stmt,
    pos: &Position,
) -> CResult<()> {
    s.begin_scope();
    if let Some(i) = init {
        compile_stmt(s, i)?;
    }
    let mut loop_start = s.here();
    let exit_jump = if let Some(c) = cond {
        compile_expr(s, c)?;
        let j = s.emit_jump(Op::JumpIfFalse, pos.line);
        s.emit(Op::Pop, pos.line); // pop the (true) condition
        Some(j)
    } else {
        None
    };

    // With an update clause, lay the update out *before* the body in the byte
    // stream and jump over it on the first iteration. The body's back-edge then
    // targets the update, so `continue` runs the update before re-testing the
    // condition — a real C-style `for`, not an infinite loop.
    if let Some(u) = update {
        let body_jump = s.emit_jump(Op::Jump, pos.line);
        let update_start = s.here();
        compile_expr(s, u)?;
        s.emit(Op::Pop, pos.line);
        s.emit_loop(loop_start, pos.line, *pos)?; // update → condition
        loop_start = update_start; // body loops back to the update
        s.patch_jump(body_jump, *pos)?;
    }

    s.loops.push(LoopCtx {
        continue_target: Some(loop_start),
        continues: Vec::new(),
        breaks: Vec::new(),
        scope_depth: s.scope_depth,
    });
    compile_stmt(s, body)?;
    s.emit_loop(loop_start, pos.line, *pos)?;
    if let Some(j) = exit_jump {
        s.patch_jump(j, *pos)?;
        s.emit(Op::Pop, pos.line); // pop the (false) condition
    }
    finish_loop(s, *pos)?;
    s.end_scope(pos.line);
    Ok(())
}

fn finish_loop(s: &mut FnState, pos: Position) -> CResult<()> {
    let ctx = s.loops.pop().expect("loop context");
    for b in ctx.breaks {
        s.patch_jump(b, pos)?;
    }
    Ok(())
}

fn compile_break(s: &mut FnState, pos: &Position) -> CResult<()> {
    if s.loops.is_empty() {
        return Err(CompileError {
            line: pos.line,
            column: pos.column,
            msg: "'break' outside of a loop".to_string(),
        });
    }
    pop_to_loop_depth(s, pos.line);
    let j = s.emit_jump(Op::Jump, pos.line);
    s.loops.last_mut().unwrap().breaks.push(j);
    Ok(())
}

fn compile_continue(s: &mut FnState, pos: &Position) -> CResult<()> {
    let target = match s.loops.last() {
        Some(c) => c.continue_target,
        None => {
            return Err(CompileError {
                line: pos.line,
                column: pos.column,
                msg: "'continue' outside of a loop".to_string(),
            })
        }
    };
    pop_to_loop_depth(s, pos.line);
    match target {
        // while / for: the continue site is behind us → backward jump.
        Some(t) => s.emit_loop(t, pos.line, *pos)?,
        // do-while: the condition re-check is ahead → forward jump, patched later.
        None => {
            let j = s.emit_jump(Op::Jump, pos.line);
            s.loops.last_mut().unwrap().continues.push(j);
        }
    }
    Ok(())
}

/// Pop locals declared inside the current loop body before a break/continue.
fn pop_to_loop_depth(s: &mut FnState, line: usize) {
    let loop_depth = s.loops.last().map(|c| c.scope_depth).unwrap_or(0);
    let mut i = s.locals.len();
    while i > 0 {
        let l = &s.locals[i - 1];
        if l.depth > loop_depth {
            s.emit(Op::Pop, line);
            i -= 1;
        } else {
            break;
        }
    }
}

fn compile_expr(s: &mut FnState, expr: &Expr) -> CResult<()> {
    match expr {
        Expr::Literal { value, pos } => compile_literal(s, value, pos),
        Expr::Var { name, pos } => {
            if let Some(slot) = s.resolve_local(name) {
                s.emit(Op::GetLocal, pos.line);
                s.emit_byte(slot as u8, pos.line);
            } else {
                let idx = s.constant(Constant::Str(Rc::from(name.as_str())));
                s.emit(Op::GetGlobal, pos.line);
                s.emit_u16(idx as u16, pos.line);
            }
            Ok(())
        }
        Expr::Assign { name, value, pos } => {
            compile_expr(s, value)?;
            if let Some(slot) = s.resolve_local(name) {
                s.emit(Op::SetLocal, pos.line);
                s.emit_byte(slot as u8, pos.line);
            } else {
                let idx = s.constant(Constant::Str(Rc::from(name.as_str())));
                s.emit(Op::SetGlobal, pos.line);
                s.emit_u16(idx as u16, pos.line);
            }
            Ok(())
        }
        Expr::CompoundAssign {
            name,
            op,
            value,
            pos,
        } => {
            // Desugar `x op= v` into `x = x op v`.
            let var = Expr::Var {
                name: name.clone(),
                pos: *pos,
            };
            let bin = Expr::Binary {
                left: Box::new(var),
                op: op.to_binary(),
                right: value.clone(),
                pos: *pos,
            };
            let assign = Expr::Assign {
                name: name.clone(),
                value: Box::new(bin),
                pos: *pos,
            };
            compile_expr(s, &assign)
        }
        Expr::Unary {
            op,
            expr: inner,
            pos,
        } => {
            compile_expr(s, inner)?;
            match op {
                UnaryOp::Minus => s.emit(Op::Neg, pos.line),
                UnaryOp::Not => s.emit(Op::Not, pos.line),
                UnaryOp::BitwiseNot => return unsupported(pos, "bitwise-not '~'"),
                UnaryOp::Typeof => return unsupported(pos, "typeof"),
            }
            Ok(())
        }
        Expr::Binary {
            left,
            op,
            right,
            pos,
        } => compile_binary(s, left, op, right, pos),
        Expr::Ternary {
            cond,
            then_e,
            else_e,
            pos,
        } => {
            compile_expr(s, cond)?;
            let then_jump = s.emit_jump(Op::JumpIfFalse, pos.line);
            s.emit(Op::Pop, pos.line);
            compile_expr(s, then_e)?;
            let else_jump = s.emit_jump(Op::Jump, pos.line);
            s.patch_jump(then_jump, *pos)?;
            s.emit(Op::Pop, pos.line);
            compile_expr(s, else_e)?;
            s.patch_jump(else_jump, *pos)?;
            Ok(())
        }
        Expr::Call { callee, args, pos } => {
            if args.len() > u8::MAX as usize {
                return unsupported(pos, "calls with more than 255 arguments");
            }
            compile_expr(s, callee)?;
            for a in args {
                if matches!(a, Expr::Spread { .. }) {
                    return unsupported(pos, "spread arguments");
                }
                compile_expr(s, a)?;
            }
            s.emit(Op::Call, pos.line);
            s.emit_byte(args.len() as u8, pos.line);
            Ok(())
        }
        Expr::Lambda { params, body, pos } => {
            let func = compile_function("<lambda>", params, body, pos)?;
            let idx = s.constant(Constant::Function(Rc::new(func)));
            emit_constant(s, idx, pos.line);
            Ok(())
        }
        other => unsupported(other.pos(), expr_kind(other)),
    }
}

fn expr_kind(expr: &Expr) -> &'static str {
    match expr {
        Expr::Array { .. } => "array literals",
        Expr::Object { .. } => "object literals",
        Expr::Index { .. } | Expr::IndexAssign { .. } => "indexing",
        Expr::Prop { .. } | Expr::PropAssign { .. } => "property access",
        Expr::OptionalProp { .. } | Expr::OptionalIndex { .. } | Expr::OptionalCall { .. } => {
            "optional chaining"
        }
        Expr::New { .. } => "'new' / class instantiation",
        Expr::This { .. } => "'this'",
        Expr::Super { .. } => "'super'",
        Expr::Match { .. } => "match expressions",
        Expr::Yield { .. } => "yield",
        Expr::Await { .. } => "await",
        Expr::Pipe { .. } => "pipe '|>'",
        Expr::Spread { .. } => "spread",
        _ => "this expression",
    }
}

fn compile_literal(s: &mut FnState, value: &Value, pos: &Position) -> CResult<()> {
    match value {
        Value::Number(n) => {
            let idx = s.constant(Constant::Number(*n));
            emit_constant(s, idx, pos.line);
        }
        Value::Str(st) => {
            let idx = s.constant(Constant::Str(Rc::from(st.as_str())));
            emit_constant(s, idx, pos.line);
        }
        Value::Bool(b) => s.emit(if *b { Op::True } else { Op::False }, pos.line),
        Value::Null => s.emit(Op::Null, pos.line),
        _ => return unsupported(pos, "this literal kind"),
    }
    Ok(())
}

fn compile_binary(
    s: &mut FnState,
    left: &Expr,
    op: &BinaryOp,
    right: &Expr,
    pos: &Position,
) -> CResult<()> {
    // Short-circuiting logical operators with value semantics (return an operand).
    match op {
        BinaryOp::And => {
            compile_expr(s, left)?;
            let end = s.emit_jump(Op::JumpIfFalse, pos.line);
            s.emit(Op::Pop, pos.line);
            compile_expr(s, right)?;
            s.patch_jump(end, *pos)?;
            return Ok(());
        }
        BinaryOp::Or => {
            compile_expr(s, left)?;
            let else_jump = s.emit_jump(Op::JumpIfFalse, pos.line);
            let end = s.emit_jump(Op::Jump, pos.line);
            s.patch_jump(else_jump, *pos)?;
            s.emit(Op::Pop, pos.line);
            compile_expr(s, right)?;
            s.patch_jump(end, *pos)?;
            return Ok(());
        }
        _ => {}
    }

    compile_expr(s, left)?;
    compile_expr(s, right)?;
    let single = match op {
        BinaryOp::Add => Op::Add,
        BinaryOp::Subtract => Op::Sub,
        BinaryOp::Multiply => Op::Mul,
        BinaryOp::Divide => Op::Div,
        BinaryOp::Modulo => Op::Mod,
        BinaryOp::Power => Op::Pow,
        BinaryOp::BitwiseAnd => Op::BitAnd,
        BinaryOp::BitwiseOr => Op::BitOr,
        BinaryOp::BitwiseXor => Op::BitXor,
        BinaryOp::ShiftLeft => Op::Shl,
        BinaryOp::ShiftRight => Op::Shr,
        BinaryOp::ShiftRightU => Op::ShrU,
        BinaryOp::Equal => Op::Equal,
        BinaryOp::NotEqual => Op::NotEqual,
        BinaryOp::Less => Op::Less,
        BinaryOp::LessEqual => Op::LessEqual,
        BinaryOp::Greater => Op::Greater,
        BinaryOp::GreaterEqual => Op::GreaterEqual,
        BinaryOp::NullCoalesce => return unsupported(pos, "null-coalescing '??'"),
        BinaryOp::In => return unsupported(pos, "the 'in' operator"),
        BinaryOp::Instanceof => return unsupported(pos, "'instanceof'"),
        BinaryOp::And | BinaryOp::Or => unreachable!("handled above"),
    };
    s.emit(single, pos.line);
    Ok(())
}
