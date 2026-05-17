use crate::ast::*;
use crate::error::{CustomLangError, Result};
use crate::lexer::{Token, TokenKind};

/// Recursive-descent parser converting tokens → AST
pub struct Parser {
    tokens: Vec<Token>,
    cur: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cur: 0 }
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut stmts = Vec::new();
        while !self.at_end() {
            self.skip_newlines();
            if self.at_end() {
                break;
            }
            stmts.push(self.statement()?);
        }
        Ok(Program::new(stmts))
    }

    // ─────────────────────────────── STATEMENTS ───────────────────────────

    fn statement(&mut self) -> Result<Stmt> {
        match &self.peek().kind {
            TokenKind::Let => self.let_stmt(),
            TokenKind::If => self.if_stmt(),
            TokenKind::While => self.while_stmt(),
            TokenKind::For => self.for_stmt(),
            TokenKind::Function => self.function_stmt(),
            TokenKind::Return => self.return_stmt(),
            TokenKind::Break => {
                let pos = self.advance().pos.clone();
                self.consume_terminator()?;
                Ok(Stmt::Break { pos })
            }
            TokenKind::Continue => {
                let pos = self.advance().pos.clone();
                self.consume_terminator()?;
                Ok(Stmt::Continue { pos })
            }
            TokenKind::Print => self.print_stmt(),
            TokenKind::Import => self.import_stmt(),
            TokenKind::Export => self.export_stmt(),
            TokenKind::Class => self.class_stmt(),
            TokenKind::LBrace => self.block_stmt(),
            _ => self.expr_stmt(),
        }
    }

    fn let_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'let'
        let name = self.expect_ident("expected variable name after 'let'")?;
        let init = if self.match_tok(&TokenKind::Eq) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume_terminator()?;
        Ok(Stmt::Let { name, init, pos })
    }

    fn if_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'if'
        self.expect(&TokenKind::LParen, "expected '(' after 'if'")?;
        let cond = self.expression()?;
        self.expect(&TokenKind::RParen, "expected ')' after if condition")?;
        self.skip_newlines();
        let then_b = Box::new(self.statement()?);
        self.skip_newlines();
        let else_b = if self.match_tok(&TokenKind::Else) {
            self.skip_newlines();
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        Ok(Stmt::If {
            cond,
            then_b,
            else_b,
            pos,
        })
    }

    fn while_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone();
        self.expect(&TokenKind::LParen, "expected '(' after 'while'")?;
        let cond = self.expression()?;
        self.expect(&TokenKind::RParen, "expected ')' after while condition")?;
        self.skip_newlines();
        let body = Box::new(self.statement()?);
        Ok(Stmt::While { cond, body, pos })
    }

    fn for_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'for'
        self.expect(&TokenKind::LParen, "expected '(' after 'for'")?;

        // Detect for-in: "for (ident in expr)"
        if self.peek_is_ident() && self.peek2_is(&TokenKind::In) {
            let var = self.expect_ident("expected variable name")?;
            self.advance(); // consume 'in'
            let iter = self.expression()?;
            self.expect(&TokenKind::RParen, "expected ')' after for-in")?;
            self.skip_newlines();
            let body = Box::new(self.statement()?);
            return Ok(Stmt::ForIn {
                var,
                iter,
                body,
                pos,
            });
        }

        // C-style: for (init; cond; update)
        let init = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(Box::new(self.for_init()?))
        };
        self.expect(&TokenKind::Semicolon, "expected ';' in for loop")?;

        let cond = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&TokenKind::Semicolon, "expected ';' in for loop")?;

        let update = if self.check(&TokenKind::RParen) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&TokenKind::RParen, "expected ')' after for clause")?;
        self.skip_newlines();
        let body = Box::new(self.statement()?);
        Ok(Stmt::For {
            init,
            cond,
            update,
            body,
            pos,
        })
    }

    fn for_init(&mut self) -> Result<Stmt> {
        if self.check(&TokenKind::Let) {
            self.let_stmt_no_term()
        } else {
            self.expr_stmt_no_term()
        }
    }

    fn let_stmt_no_term(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone();
        let name = self.expect_ident("expected variable name after 'let'")?;
        let init = if self.match_tok(&TokenKind::Eq) {
            Some(self.expression()?)
        } else {
            None
        };
        Ok(Stmt::Let { name, init, pos })
    }

    fn expr_stmt_no_term(&mut self) -> Result<Stmt> {
        let expr = self.expression()?;
        let pos = expr.pos().clone();
        Ok(Stmt::Expr { expr, pos })
    }

    fn function_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'function'
        let name = self.expect_ident("expected function name")?;
        let params = self.parse_params()?;
        self.skip_newlines();
        let body = Box::new(self.block_stmt()?);
        Ok(Stmt::Function {
            name,
            params,
            body,
            pos,
        })
    }

    fn return_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'return'
        let value = if self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::Newline)
            || self.check(&TokenKind::RBrace)
            || self.at_end()
        {
            None
        } else {
            Some(self.expression()?)
        };
        self.consume_terminator()?;
        Ok(Stmt::Return { value, pos })
    }

    fn print_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'print'
        let expr = self.expression()?;
        self.consume_terminator()?;
        Ok(Stmt::Print { expr, pos })
    }

    fn import_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone();
        let path = match self.peek().kind.clone() {
            TokenKind::Str(s) => {
                self.advance();
                s
            }
            _ => return Err(self.parse_err("expected string literal as module path")),
        };
        let alias = if self.match_ident("as") {
            Some(self.expect_ident("expected alias name after 'as'")?)
        } else {
            None
        };
        self.consume_terminator()?;
        Ok(Stmt::Import { path, alias, pos })
    }

    fn export_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone();
        let name = self.expect_ident("expected identifier to export")?;
        self.consume_terminator()?;
        Ok(Stmt::Export { name, pos })
    }

    fn class_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'class'
        let name = self.expect_ident("expected class name")?;
        let super_name = if self.match_tok(&TokenKind::Extends) {
            Some(self.expect_ident("expected superclass name after 'extends'")?)
        } else {
            None
        };
        self.expect(&TokenKind::LBrace, "expected '{' before class body")?;
        let mut methods = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) || self.at_end() {
                break;
            }
            if self.check(&TokenKind::Function) {
                methods.push(self.function_stmt()?);
            } else {
                return Err(self.parse_err("expected method declaration in class body"));
            }
        }
        self.expect(&TokenKind::RBrace, "expected '}' after class body")?;
        Ok(Stmt::Class {
            name,
            super_name,
            methods,
            pos,
        })
    }

    fn block_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume '{'
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) || self.at_end() {
                break;
            }
            stmts.push(self.statement()?);
        }
        self.expect(&TokenKind::RBrace, "expected '}' after block")?;
        Ok(Stmt::Block { stmts, pos })
    }

    fn expr_stmt(&mut self) -> Result<Stmt> {
        let expr = self.expression()?;
        let pos = expr.pos().clone();
        self.consume_terminator()?;
        Ok(Stmt::Expr { expr, pos })
    }

    // ─────────────────────────────── EXPRESSIONS ──────────────────────────

    fn expression(&mut self) -> Result<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr> {
        let expr = self.or()?;

        // Compound assignment: += -= *= /= %=
        if let Some(op) = self.match_compound_op() {
            let pos = self.prev_pos();
            let rhs = self.assignment()?;
            match &expr {
                Expr::Var { name, .. } => {
                    return Ok(Expr::CompoundAssign {
                        name: name.clone(),
                        op,
                        value: Box::new(rhs),
                        pos,
                    });
                }
                Expr::Prop { object, name, .. } => {
                    // obj.prop += val  →  obj.prop = obj.prop + val
                    let read = Expr::Prop { object: object.clone(), name: name.clone(), pos: pos.clone() };
                    let combined = Expr::Binary {
                        left: Box::new(read),
                        op: op.to_binary(),
                        right: Box::new(rhs),
                        pos: pos.clone(),
                    };
                    return Ok(Expr::PropAssign {
                        object: object.clone(),
                        prop: name.clone(),
                        value: Box::new(combined),
                        pos,
                    });
                }
                Expr::Index { object, index, .. } => {
                    // arr[i] += val  →  arr[i] = arr[i] + val
                    let read = Expr::Index { object: object.clone(), index: index.clone(), pos: pos.clone() };
                    let combined = Expr::Binary {
                        left: Box::new(read),
                        op: op.to_binary(),
                        right: Box::new(rhs),
                        pos: pos.clone(),
                    };
                    return Ok(Expr::IndexAssign {
                        object: object.clone(),
                        index: index.clone(),
                        value: Box::new(combined),
                        pos,
                    });
                }
                _ => {
                    return Err(CustomLangError::parse(
                        pos.line,
                        pos.column,
                        "invalid compound assignment target",
                    ));
                }
            }
        }

        // Regular assignment
        if self.match_tok(&TokenKind::Eq) {
            let pos = self.prev_pos();
            let rhs = self.assignment()?;
            return match expr {
                Expr::Var { name, .. } => Ok(Expr::Assign {
                    name,
                    value: Box::new(rhs),
                    pos,
                }),
                Expr::Index { object, index, .. } => Ok(Expr::IndexAssign {
                    object,
                    index,
                    value: Box::new(rhs),
                    pos,
                }),
                Expr::Prop { object, name, .. } => Ok(Expr::PropAssign {
                    object,
                    prop: name,
                    value: Box::new(rhs),
                    pos,
                }),
                _ => Err(CustomLangError::parse(
                    pos.line,
                    pos.column,
                    "invalid assignment target",
                )),
            };
        }

        Ok(expr)
    }

    fn or(&mut self) -> Result<Expr> {
        let mut expr = self.and()?;
        while self.match_tok(&TokenKind::OrOr) {
            let pos = self.prev_pos();
            let right = self.and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn and(&mut self) -> Result<Expr> {
        let mut expr = self.equality()?;
        while self.match_tok(&TokenKind::AndAnd) {
            let pos = self.prev_pos();
            let right = self.equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr> {
        let mut expr = self.comparison()?;
        while let Some(op) = self.match_eq_op() {
            let pos = self.prev_pos();
            let right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr> {
        let mut expr = self.term()?;
        while let Some(op) = self.match_cmp_op() {
            let pos = self.prev_pos();
            let right = self.term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr> {
        let mut expr = self.factor()?;
        while let Some(op) = self.match_add_op() {
            let pos = self.prev_pos();
            let right = self.factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr> {
        let mut expr = self.unary()?;
        while let Some(op) = self.match_mul_op() {
            let pos = self.prev_pos();
            let right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.match_tok(&TokenKind::Bang) {
            let pos = self.prev_pos();
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                pos,
            });
        }
        if self.match_tok(&TokenKind::Minus) {
            let pos = self.prev_pos();
            // Optimisation: negative numeric literal
            if let TokenKind::Number(n) = self.peek().kind.clone() {
                if !self.at_end() {
                    self.advance();
                    return Ok(Expr::Literal {
                        value: Value::Number(-n),
                        pos,
                    });
                }
            }
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Minus,
                expr: Box::new(expr),
                pos,
            });
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr> {
        let mut expr = self.primary()?;
        loop {
            if self.match_tok(&TokenKind::LParen) {
                let pos = self.prev_pos();
                let args = self.parse_args()?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    pos,
                };
            } else if self.match_tok(&TokenKind::LBracket) {
                let pos = self.prev_pos();
                let index = self.expression()?;
                self.expect(&TokenKind::RBracket, "expected ']' after index")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    pos,
                };
            } else if self.match_tok(&TokenKind::Dot) {
                let pos = self.prev_pos();
                let name = self.expect_ident("expected property name after '.'")?;
                expr = Expr::Prop {
                    object: Box::new(expr),
                    name,
                    pos,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr> {
        let tok = self.advance();
        let pos = tok.pos.clone();
        let kind_display = format!("{:?}", tok.kind);
        match tok.kind.clone() {
            TokenKind::True => Ok(Expr::Literal {
                value: Value::Bool(true),
                pos,
            }),
            TokenKind::False => Ok(Expr::Literal {
                value: Value::Bool(false),
                pos,
            }),
            TokenKind::Null => Ok(Expr::Literal {
                value: Value::Null,
                pos,
            }),
            TokenKind::Number(n) => Ok(Expr::Literal {
                value: Value::Number(n),
                pos,
            }),
            TokenKind::Str(s) => Ok(Expr::Literal {
                value: Value::Str(s),
                pos,
            }),
            TokenKind::Ident(name) => Ok(Expr::Var { name, pos }),
            TokenKind::This => Ok(Expr::This { pos }),
            TokenKind::LParen => {
                let expr = self.expression()?;
                self.expect(&TokenKind::RParen, "expected ')'")?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                let mut elements = Vec::new();
                self.skip_newlines();
                while !self.check(&TokenKind::RBracket) && !self.at_end() {
                    elements.push(self.expression()?);
                    self.skip_newlines();
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&TokenKind::RBracket, "expected ']' after array")?;
                Ok(Expr::Array { elements, pos })
            }
            TokenKind::LBrace => {
                let mut pairs = Vec::new();
                self.skip_newlines();
                while !self.check(&TokenKind::RBrace) && !self.at_end() {
                    let key = match self.peek().kind.clone() {
                        TokenKind::Ident(name) => {
                            self.advance();
                            name
                        }
                        TokenKind::Str(s) => {
                            self.advance();
                            s
                        }
                        _ => {
                            return Err(
                                self.parse_err("expected property key (identifier or string)")
                            )
                        }
                    };
                    self.expect(&TokenKind::Colon, "expected ':' after object key")?;
                    let value = self.expression()?;
                    pairs.push((key, value));
                    self.skip_newlines();
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&TokenKind::RBrace, "expected '}' after object")?;
                Ok(Expr::Object { pairs, pos })
            }
            TokenKind::New => {
                let class = self.expect_ident("expected class name after 'new'")?;
                self.expect(&TokenKind::LParen, "expected '('")?;
                let args = self.parse_args()?;
                Ok(Expr::New { class, args, pos })
            }
            TokenKind::Function => {
                // Anonymous function expression: function(params) { body }
                let params = self.parse_params()?;
                self.skip_newlines();
                let body = Box::new(self.block_stmt()?);
                Ok(Expr::Lambda { params, body, pos })
            }
            TokenKind::Match => self.match_expr(pos),
            _ => Err(CustomLangError::parse(
                pos.line,
                pos.column,
                format!("unexpected token '{kind_display}'"),
            )),
        }
    }

    fn match_expr(&mut self, pos: Position) -> Result<Expr> {
        let expr = Box::new(self.expression()?);
        self.expect(&TokenKind::LBrace, "expected '{' after match expression")?;
        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) || self.at_end() {
                break;
            }
            let pattern = self.parse_pattern()?;
            self.expect(&TokenKind::Arrow, "expected '=>' after pattern")?;
            let body = self.expression()?;
            arms.push(MatchArm { pattern, body });
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            if !self.match_tok(&TokenKind::Comma) {
                return Err(self.parse_err("expected ',' between match arms"));
            }
        }
        self.expect(&TokenKind::RBrace, "expected '}' after match arms")?;
        Ok(Expr::Match { expr, arms, pos })
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
        let tok = self.advance();
        match tok.kind.clone() {
            TokenKind::Number(n) => Ok(Pattern::Number(n)),
            TokenKind::Str(s) => Ok(Pattern::Str(s)),
            TokenKind::True => Ok(Pattern::Bool(true)),
            TokenKind::False => Ok(Pattern::Bool(false)),
            TokenKind::Null => Ok(Pattern::Null),
            TokenKind::Ident(name) if name == "_" => Ok(Pattern::Wildcard),
            TokenKind::Ident(name) => Ok(Pattern::Binding(name)),
            TokenKind::LBracket => {
                let mut pats = Vec::new();
                while !self.check(&TokenKind::RBracket) && !self.at_end() {
                    pats.push(self.parse_pattern()?);
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RBracket, "expected ']'")?;
                Ok(Pattern::Array(pats))
            }
            TokenKind::LBrace => {
                let mut pairs = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.at_end() {
                    let key = self.expect_ident("expected key in object pattern")?;
                    self.expect(&TokenKind::Colon, "expected ':'")?;
                    let pat = self.parse_pattern()?;
                    pairs.push((key, pat));
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RBrace, "expected '}'")?;
                Ok(Pattern::Object(pairs))
            }
            _ => Err(CustomLangError::parse(
                tok.pos.line,
                tok.pos.column,
                "expected pattern",
            )),
        }
    }

    // ─────────────────────────────── HELPERS ──────────────────────────────

    fn parse_params(&mut self) -> Result<Vec<String>> {
        self.expect(&TokenKind::LParen, "expected '(' before parameters")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                params.push(self.expect_ident("expected parameter name")?);
                if !self.match_tok(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen, "expected ')' after parameters")?;
        Ok(params)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        self.skip_newlines();
        if !self.check(&TokenKind::RParen) {
            loop {
                self.skip_newlines();
                args.push(self.expression()?);
                self.skip_newlines();
                if !self.match_tok(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.skip_newlines();
        self.expect(&TokenKind::RParen, "expected ')' after arguments")?;
        Ok(args)
    }

    fn at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.cur]
    }

    fn peek2(&self) -> &Token {
        if self.cur + 1 < self.tokens.len() {
            &self.tokens[self.cur + 1]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.at_end() {
            self.cur += 1;
        }
        &self.tokens[self.cur - 1]
    }

    fn prev_pos(&self) -> Position {
        if self.cur > 0 {
            self.tokens[self.cur - 1].pos.clone()
        } else {
            Position::default()
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn match_tok(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_ident(&mut self, name: &str) -> bool {
        if let TokenKind::Ident(id) = &self.peek().kind {
            if id == name {
                self.advance();
                return true;
            }
        }
        false
    }

    fn peek_is_ident(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(_))
    }

    fn peek2_is(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek2().kind) == std::mem::discriminant(kind)
    }

    fn expect(&mut self, kind: &TokenKind, msg: &str) -> Result<&Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.parse_err(msg))
        }
    }

    fn expect_ident(&mut self, msg: &str) -> Result<String> {
        match self.peek().kind.clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.parse_err(msg)),
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    fn consume_terminator(&mut self) -> Result<()> {
        if self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::Newline)
            || self.check(&TokenKind::RBrace)
            || self.at_end()
        {
            if !self.check(&TokenKind::RBrace) && !self.at_end() {
                self.advance();
            }
            Ok(())
        } else {
            Err(self.parse_err("expected ';' or newline"))
        }
    }

    fn parse_err(&self, msg: impl Into<String>) -> CustomLangError {
        let pos = &self.peek().pos;
        CustomLangError::parse(pos.line, pos.column, msg)
    }

    fn match_eq_op(&mut self) -> Option<BinaryOp> {
        if self.match_tok(&TokenKind::EqEq) {
            Some(BinaryOp::Equal)
        } else if self.match_tok(&TokenKind::BangEq) {
            Some(BinaryOp::NotEqual)
        } else {
            None
        }
    }

    fn match_cmp_op(&mut self) -> Option<BinaryOp> {
        if self.match_tok(&TokenKind::Gt) {
            Some(BinaryOp::Greater)
        } else if self.match_tok(&TokenKind::GtEq) {
            Some(BinaryOp::GreaterEqual)
        } else if self.match_tok(&TokenKind::Lt) {
            Some(BinaryOp::Less)
        } else if self.match_tok(&TokenKind::LtEq) {
            Some(BinaryOp::LessEqual)
        } else {
            None
        }
    }

    fn match_add_op(&mut self) -> Option<BinaryOp> {
        if self.match_tok(&TokenKind::Plus) {
            Some(BinaryOp::Add)
        } else if self.match_tok(&TokenKind::Minus) {
            Some(BinaryOp::Subtract)
        } else {
            None
        }
    }

    fn match_mul_op(&mut self) -> Option<BinaryOp> {
        if self.match_tok(&TokenKind::Star) {
            Some(BinaryOp::Multiply)
        } else if self.match_tok(&TokenKind::Slash) {
            Some(BinaryOp::Divide)
        } else if self.match_tok(&TokenKind::Percent) {
            Some(BinaryOp::Modulo)
        } else {
            None
        }
    }

    fn match_compound_op(&mut self) -> Option<CompoundOp> {
        if self.match_tok(&TokenKind::PlusEq) {
            Some(CompoundOp::Add)
        } else if self.match_tok(&TokenKind::MinusEq) {
            Some(CompoundOp::Subtract)
        } else if self.match_tok(&TokenKind::StarEq) {
            Some(CompoundOp::Multiply)
        } else if self.match_tok(&TokenKind::SlashEq) {
            Some(CompoundOp::Divide)
        } else if self.match_tok(&TokenKind::PercentEq) {
            Some(CompoundOp::Modulo)
        } else {
            None
        }
    }
}

// ─────────────────────────────── TESTS ──────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_expr(src: &str) -> Expr {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let mut p = Parser::new(tokens);
        p.expression().unwrap()
    }

    fn parse_stmt(src: &str) -> Stmt {
        let tokens = Lexer::new(src).tokenize().unwrap();
        let mut p = Parser::new(tokens);
        p.statement().unwrap()
    }

    #[test]
    fn test_number_literal() {
        match parse_expr("42") {
            Expr::Literal {
                value: Value::Number(n),
                ..
            } => assert_eq!(n, 42.0),
            _ => panic!("expected number"),
        }
    }

    #[test]
    fn test_negative_literal() {
        match parse_expr("-5") {
            Expr::Literal {
                value: Value::Number(n),
                ..
            } => assert_eq!(n, -5.0),
            Expr::Unary {
                op: UnaryOp::Minus, ..
            } => {} // also acceptable
            _ => panic!("expected negative number"),
        }
    }

    #[test]
    fn test_binary_add() {
        match parse_expr("2 + 3") {
            Expr::Binary {
                op: BinaryOp::Add, ..
            } => {}
            _ => panic!("expected add"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        // 2 + 3 * 4 should parse as 2 + (3 * 4)
        match parse_expr("2 + 3 * 4") {
            Expr::Binary {
                op: BinaryOp::Add,
                right,
                ..
            } => match right.as_ref() {
                Expr::Binary {
                    op: BinaryOp::Multiply,
                    ..
                } => {}
                _ => panic!("expected multiply on right"),
            },
            _ => panic!("expected add at top"),
        }
    }

    #[test]
    fn test_let_stmt() {
        match parse_stmt("let x = 42;") {
            Stmt::Let { name, .. } => assert_eq!(name, "x"),
            _ => panic!("expected let"),
        }
    }

    #[test]
    fn test_compound_assign() {
        match parse_expr("x += 5") {
            Expr::CompoundAssign {
                name,
                op: CompoundOp::Add,
                ..
            } => assert_eq!(name, "x"),
            _ => panic!("expected compound assign"),
        }
    }

    #[test]
    fn test_for_in() {
        match parse_stmt("for (item in arr) { print item; }") {
            Stmt::ForIn { var, .. } => assert_eq!(var, "item"),
            _ => panic!("expected for-in"),
        }
    }

    #[test]
    fn test_for_cstyle() {
        match parse_stmt("for (let i = 0; i < 10; i += 1) { }") {
            Stmt::For {
                init: Some(_),
                cond: Some(_),
                update: Some(_),
                ..
            } => {}
            _ => panic!("expected C-style for"),
        }
    }

    #[test]
    fn test_while_stmt() {
        match parse_stmt("while (x > 0) { x -= 1; }") {
            Stmt::While { .. } => {}
            _ => panic!("expected while"),
        }
    }

    #[test]
    fn test_function_decl() {
        match parse_stmt("function add(a, b) { return a + b; }") {
            Stmt::Function { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params, vec!["a", "b"]);
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_index_assign() {
        match parse_expr("arr[0] = 42") {
            Expr::IndexAssign { .. } => {}
            _ => panic!("expected index assign"),
        }
    }

    #[test]
    fn test_prop_assign() {
        match parse_expr("obj.field = 42") {
            Expr::PropAssign { prop, .. } => assert_eq!(prop, "field"),
            _ => panic!("expected prop assign"),
        }
    }
}
