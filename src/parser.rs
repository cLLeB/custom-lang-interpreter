use crate::ast::*;
use crate::error::{CustomLangError, Result};
use crate::lexer::{Lexer, Token, TokenKind};

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
            TokenKind::Do => self.do_while_stmt(),
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
            TokenKind::Try => self.try_stmt(),
            TokenKind::Throw => self.throw_stmt(),
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

    fn do_while_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'do'
        self.skip_newlines();
        let body = Box::new(self.statement()?);
        self.skip_newlines();
        // expect 'while'
        if !self.check(&TokenKind::While) {
            return Err(self.parse_err("expected 'while' after do block"));
        }
        self.advance(); // consume 'while'
        self.expect(&TokenKind::LParen, "expected '(' after 'while'")?;
        let cond = self.expression()?;
        self.expect(&TokenKind::RParen, "expected ')' after do-while condition")?;
        self.consume_terminator()?;
        Ok(Stmt::DoWhile { body, cond, pos })
    }

    fn try_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'try'
        self.skip_newlines();
        let try_b = Box::new(self.block_stmt()?);

        let mut catch_var = None;
        let mut catch_b = None;
        let mut finally_b = None;

        self.skip_newlines();
        if self.check(&TokenKind::Catch) {
            self.advance(); // consume 'catch'
            if self.match_tok(&TokenKind::LParen) {
                catch_var = Some(self.expect_ident("expected catch variable name")?);
                self.expect(&TokenKind::RParen, "expected ')' after catch variable")?;
            }
            self.skip_newlines();
            catch_b = Some(Box::new(self.block_stmt()?));
        }

        self.skip_newlines();
        if self.check(&TokenKind::Finally) {
            self.advance(); // consume 'finally'
            self.skip_newlines();
            finally_b = Some(Box::new(self.block_stmt()?));
        }

        if catch_b.is_none() && finally_b.is_none() {
            return Err(self.parse_err("try must be followed by catch or finally"));
        }

        Ok(Stmt::TryCatch {
            try_b,
            catch_var,
            catch_b,
            finally_b,
            pos,
        })
    }

    fn throw_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'throw'
        let value = self.expression()?;
        self.consume_terminator()?;
        Ok(Stmt::Throw { value, pos })
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
        self.function_stmt_with_static(false)
    }

    fn function_stmt_with_static(&mut self, is_static: bool) -> Result<Stmt> {
        let pos = self.advance().pos.clone(); // consume 'function'
        let name = self.expect_ident("expected function name")?;
        let params = self.parse_params()?;
        self.skip_newlines();
        let body = Box::new(self.block_stmt()?);
        Ok(Stmt::Function {
            name,
            params,
            body,
            is_static,
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
            // Handle static keyword
            let is_static = if self.check(&TokenKind::Static) {
                self.advance();
                true
            } else {
                false
            };
            if self.check(&TokenKind::Function) {
                methods.push(self.function_stmt_with_static(is_static)?);
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
    //
    // Precedence (lowest → highest):
    //   assignment → ternary → logical_or → null_coalesce → logical_and →
    //   bitwise_or → bitwise_xor → bitwise_and → equality →
    //   comparison(relational+in+instanceof) → shift →
    //   term(+/-) → factor(*/%`) → power(**) → unary → call → primary

    fn expression(&mut self) -> Result<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr> {
        let expr = self.ternary()?;

        // Compound assignment: += -= *= /= %= **= &= |= ^= <<= >>= &&= ||= ??=
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

    fn ternary(&mut self) -> Result<Expr> {
        let expr = self.logical_or()?;
        if self.match_tok(&TokenKind::Question) {
            let pos = self.prev_pos();
            let then_e = Box::new(self.expression()?);
            self.expect(&TokenKind::Colon, "expected ':' in ternary expression")?;
            let else_e = Box::new(self.ternary()?);
            return Ok(Expr::Ternary { cond: Box::new(expr), then_e, else_e, pos });
        }
        Ok(expr)
    }

    fn logical_or(&mut self) -> Result<Expr> {
        let mut expr = self.null_coalesce()?;
        while self.match_tok(&TokenKind::OrOr) {
            let pos = self.prev_pos();
            let right = self.null_coalesce()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn null_coalesce(&mut self) -> Result<Expr> {
        let mut expr = self.logical_and()?;
        while self.match_tok(&TokenKind::QuestionQuestion) {
            let pos = self.prev_pos();
            let right = self.logical_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::NullCoalesce,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn logical_and(&mut self) -> Result<Expr> {
        let mut expr = self.bitwise_or()?;
        while self.match_tok(&TokenKind::AndAnd) {
            let pos = self.prev_pos();
            let right = self.bitwise_or()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn bitwise_or(&mut self) -> Result<Expr> {
        let mut expr = self.bitwise_xor()?;
        while self.match_tok(&TokenKind::Pipe) {
            let pos = self.prev_pos();
            let right = self.bitwise_xor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseOr,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn bitwise_xor(&mut self) -> Result<Expr> {
        let mut expr = self.bitwise_and()?;
        while self.match_tok(&TokenKind::Caret) {
            let pos = self.prev_pos();
            let right = self.bitwise_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseXor,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn bitwise_and(&mut self) -> Result<Expr> {
        let mut expr = self.equality()?;
        while self.match_tok(&TokenKind::Amp) {
            let pos = self.prev_pos();
            let right = self.equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseAnd,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr> {
        let mut expr = self.relational()?;
        while let Some(op) = self.match_eq_op() {
            let pos = self.prev_pos();
            let right = self.relational()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn relational(&mut self) -> Result<Expr> {
        let mut expr = self.shift()?;
        loop {
            let pos = self.peek().pos.clone();
            if let Some(op) = self.match_cmp_op() {
                let right = self.shift()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                    pos,
                };
            } else if self.match_tok(&TokenKind::In) {
                let right = self.shift()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::In,
                    right: Box::new(right),
                    pos,
                };
            } else if self.match_tok(&TokenKind::Instanceof) {
                let right = self.shift()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Instanceof,
                    right: Box::new(right),
                    pos,
                };
            } else if self.match_ident("is") {
                let right = self.shift()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Instanceof,
                    right: Box::new(right),
                    pos,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn shift(&mut self) -> Result<Expr> {
        let mut expr = self.term()?;
        loop {
            let pos = self.peek().pos.clone();
            if self.match_tok(&TokenKind::LtLt) {
                let right = self.term()?;
                expr = Expr::Binary { left: Box::new(expr), op: BinaryOp::ShiftLeft, right: Box::new(right), pos };
            } else if self.match_tok(&TokenKind::GtGtGt) {
                let right = self.term()?;
                expr = Expr::Binary { left: Box::new(expr), op: BinaryOp::ShiftRightU, right: Box::new(right), pos };
            } else if self.match_tok(&TokenKind::GtGt) {
                let right = self.term()?;
                expr = Expr::Binary { left: Box::new(expr), op: BinaryOp::ShiftRight, right: Box::new(right), pos };
            } else {
                break;
            }
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
        let mut expr = self.power()?;
        while let Some(op) = self.match_mul_op() {
            let pos = self.prev_pos();
            let right = self.power()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                pos,
            };
        }
        Ok(expr)
    }

    fn power(&mut self) -> Result<Expr> {
        let base = self.unary()?;
        if self.match_tok(&TokenKind::StarStar) {
            let pos = self.prev_pos();
            // Right-associative: recurse into power
            let exp = self.power()?;
            return Ok(Expr::Binary {
                left: Box::new(base),
                op: BinaryOp::Power,
                right: Box::new(exp),
                pos,
            });
        }
        Ok(base)
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
        if self.match_tok(&TokenKind::Tilde) {
            let pos = self.prev_pos();
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::BitwiseNot,
                expr: Box::new(expr),
                pos,
            });
        }
        if self.match_ident("typeof") {
            let pos = self.prev_pos();
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Typeof,
                expr: Box::new(expr),
                pos,
            });
        }
        // Spread operator
        if self.match_tok(&TokenKind::DotDotDot) {
            let pos = self.prev_pos();
            let expr = self.unary()?;
            return Ok(Expr::Spread {
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
            } else if self.match_tok(&TokenKind::QuestionDot) {
                let pos = self.prev_pos();
                // obj?.prop  or  obj?.[idx]  or  obj?.(args)
                if self.match_tok(&TokenKind::LBracket) {
                    let index = self.expression()?;
                    self.expect(&TokenKind::RBracket, "expected ']'")?;
                    expr = Expr::OptionalIndex {
                        object: Box::new(expr),
                        index: Box::new(index),
                        pos,
                    };
                } else if self.match_tok(&TokenKind::LParen) {
                    let args = self.parse_args()?;
                    expr = Expr::OptionalCall {
                        callee: Box::new(expr),
                        args,
                        pos,
                    };
                } else {
                    let name = self.expect_ident("expected property name after '?.'")?;
                    expr = Expr::OptionalProp {
                        object: Box::new(expr),
                        name,
                        pos,
                    };
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr> {
        // Check for arrow function: ident => expr  (single param no parens)
        if self.peek_is_ident() && self.peek2_is(&TokenKind::Arrow) {
            let pos = self.peek().pos.clone();
            let name = self.expect_ident("expected param name")?;
            self.advance(); // consume =>
            return self.parse_arrow_body(vec![Param::simple(name)], pos);
        }

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
            TokenKind::TemplateLiteral(raw) => self.parse_template_literal(&raw, pos),
            TokenKind::Ident(name) => Ok(Expr::Var { name, pos }),
            TokenKind::This => Ok(Expr::This { pos }),
            TokenKind::Super => Ok(Expr::Super { pos }),
            TokenKind::LParen => {
                // Check if this is an arrow function: (params) =>
                if self.is_arrow_params_lookahead() {
                    let params = self.parse_params_after_lparen()?;
                    // consume =>
                    self.expect(&TokenKind::Arrow, "expected '=>' for arrow function")?;
                    return self.parse_arrow_body(params, pos);
                }
                let expr = self.expression()?;
                self.expect(&TokenKind::RParen, "expected ')'")?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                let mut elements = Vec::new();
                self.skip_newlines();
                while !self.check(&TokenKind::RBracket) && !self.at_end() {
                    if self.check(&TokenKind::DotDotDot) {
                        let spread_pos = self.peek().pos.clone();
                        self.advance(); // consume ...
                        let expr = self.assignment()?;
                        elements.push(Expr::Spread { expr: Box::new(expr), pos: spread_pos });
                    } else {
                        elements.push(self.assignment()?);
                    }
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
                    // Spread in object: {...other}
                    if self.check(&TokenKind::DotDotDot) {
                        let spread_pos = self.peek().pos.clone();
                        self.advance();
                        let expr = self.assignment()?;
                        pairs.push((ObjectKey::Computed(Box::new(Expr::Literal { value: Value::Str("__spread__".to_string()), pos: spread_pos.clone() })), Expr::Spread { expr: Box::new(expr), pos: spread_pos }));
                    } else {
                        let (key, value) = self.parse_object_pair()?;
                        pairs.push((key, value));
                    }
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
                // Support new ClassName(args) and new expr(args)
                let class_expr = Box::new(self.call()?);
                // The call() might have already consumed the (args) part if written as `new Foo(args)`
                // Actually for `new Foo(args)`, the Foo is parsed by call() which will not consume (args)
                // since we're in the middle of new parsing. Let me handle this differently.
                // Actually: new Class(...) — class is just a name (or expr), then (args)
                // Let me parse class as a primary (identifier or property access chain)
                // We already consumed 'new', so let's parse the class expression
                // then check for (args)
                // Actually the Box::new(self.call()?) above already handles member access like `new Foo.Bar()`
                // The call() will parse Foo.Bar but stop before (. Let me check...
                // self.call() will parse `Foo.Bar` because it handles `.` access, but stops before `(` only
                // if there's nothing to consume. Actually call() WILL consume `(args)` if present!
                // So by calling self.call() we already consumed the `(args)` in: new Foo(args)
                // That's wrong. Let me parse differently.
                //
                // Actually let me just re-parse: we need to check for `(args)` explicitly
                // Since call() already consumed (args)... let me handle new differently.
                //
                // This is getting complex. Let me use a simpler approach:
                // Parse new as: class_name + optional (args)
                // The class_name can be any call expression (handles new Foo.Bar())

                // We already have class_expr from self.call()
                // If class_expr is a Call, extract callee and args from it
                match *class_expr {
                    Expr::Call { callee, args, pos: call_pos } => {
                        Ok(Expr::New { class: callee, args, pos: call_pos })
                    }
                    other => {
                        // No args provided: new Foo
                        Ok(Expr::New { class: Box::new(other), args: vec![], pos })
                    }
                }
            }
            TokenKind::Function => {
                // Check if next is ident (named lambda) or just params
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

    /// Parse object key:value pair, handling shorthand, computed, and regular
    fn parse_object_pair(&mut self) -> Result<(ObjectKey, Expr)> {
        let pair_pos = self.peek().pos.clone();
        // Computed property: [expr]: value
        if self.match_tok(&TokenKind::LBracket) {
            let key_expr = self.expression()?;
            self.expect(&TokenKind::RBracket, "expected ']' after computed key")?;
            self.expect(&TokenKind::Colon, "expected ':' after computed key")?;
            let value = self.expression()?;
            return Ok((ObjectKey::Computed(Box::new(key_expr)), value));
        }

        let key = match self.peek().kind.clone() {
            TokenKind::Ident(name) => {
                self.advance();
                name
            }
            TokenKind::Str(s) => {
                self.advance();
                s
            }
            TokenKind::Number(n) => {
                self.advance();
                n.to_string()
            }
            _ => return Err(self.parse_err("expected property key (identifier or string)")),
        };

        // Shorthand property: {name} instead of {name: name}
        if !self.check(&TokenKind::Colon) {
            let value = Expr::Var { name: key.clone(), pos: pair_pos };
            return Ok((ObjectKey::Static(key), value));
        }

        self.expect(&TokenKind::Colon, "expected ':' after object key")?;
        let value = self.expression()?;
        Ok((ObjectKey::Static(key), value))
    }

    /// Parse template literal with ${...} interpolation
    fn parse_template_literal(&mut self, raw: &str, pos: Position) -> Result<Expr> {
        let chars: Vec<char> = raw.chars().collect();
        let mut parts: Vec<Expr> = Vec::new();
        let mut current = String::new();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                if !current.is_empty() {
                    parts.push(Expr::Literal {
                        value: Value::Str(current.clone()),
                        pos: pos.clone(),
                    });
                    current.clear();
                }
                i += 2; // skip ${
                let mut depth = 1usize;
                let mut expr_src = String::new();
                while i < chars.len() {
                    match chars[i] {
                        '{' => { depth += 1; expr_src.push('{'); }
                        '}' => {
                            depth -= 1;
                            if depth == 0 { i += 1; break; }
                            expr_src.push('}');
                        }
                        c => expr_src.push(c),
                    }
                    i += 1;
                }
                // Re-lex and re-parse the interpolated expression
                let tokens = Lexer::new(&expr_src).tokenize().map_err(|e| {
                    CustomLangError::parse(pos.line, pos.column, format!("in template literal: {e}"))
                })?;
                let mut inner = Parser::new(tokens);
                let expr = inner.expression().map_err(|e| {
                    CustomLangError::parse(pos.line, pos.column, format!("in template literal: {e}"))
                })?;
                parts.push(expr);
            } else {
                current.push(chars[i]);
                i += 1;
            }
        }
        if !current.is_empty() {
            parts.push(Expr::Literal {
                value: Value::Str(current),
                pos: pos.clone(),
            });
        }
        if parts.is_empty() {
            return Ok(Expr::Literal {
                value: Value::Str(String::new()),
                pos,
            });
        }
        // Build concatenation tree: each part converted to string via +
        let mut result = parts.remove(0);
        for part in parts {
            result = Expr::Binary {
                left: Box::new(result),
                op: BinaryOp::Add,
                right: Box::new(part),
                pos: pos.clone(),
            };
        }
        Ok(result)
    }

    /// Parse arrow function body: `expr` or `{ block }`
    fn parse_arrow_body(&mut self, params: Vec<Param>, pos: Position) -> Result<Expr> {
        self.skip_newlines();
        if self.check(&TokenKind::LBrace) {
            let body = Box::new(self.block_stmt()?);
            Ok(Expr::Lambda { params, body, pos })
        } else {
            // Expression body: implicitly return the expression
            let expr = self.assignment()?;
            let expr_pos = expr.pos().clone();
            let body = Box::new(Stmt::Return {
                value: Some(expr),
                pos: expr_pos,
            });
            Ok(Expr::Lambda { params, body, pos })
        }
    }

    /// Look ahead to determine if the current `(` starts arrow function params
    fn is_arrow_params_lookahead(&self) -> bool {
        // Scan forward through the tokens to find matching ) followed by =>
        let mut depth = 1;
        let mut i = self.cur;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        // Check next token (skip newlines)
                        let mut j = i + 1;
                        while j < self.tokens.len() {
                            match &self.tokens[j].kind {
                                TokenKind::Newline => j += 1,
                                TokenKind::Arrow => return true,
                                _ => return false,
                            }
                        }
                        return false;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// Parse params that come after an already-consumed `(`
    fn parse_params_after_lparen(&mut self) -> Result<Vec<Param>> {
        let mut params = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.at_end() {
            self.skip_newlines();
            let is_rest = self.match_tok(&TokenKind::DotDotDot);
            let name = self.expect_ident("expected parameter name")?;
            let default = if !is_rest && self.match_tok(&TokenKind::Eq) {
                Some(self.assignment()?)
            } else {
                None
            };
            params.push(Param { name, default, is_rest });
            if is_rest { break; }
            if !self.match_tok(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RParen, "expected ')' after parameters")?;
        Ok(params)
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
            // Optional guard: `when condition`
            let guard = if self.match_ident("when") {
                Some(self.expression()?)
            } else {
                None
            };
            self.expect(&TokenKind::Arrow, "expected '=>' after pattern")?;
            let body = self.expression()?;
            arms.push(MatchArm { pattern, guard, body });
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

    fn parse_params(&mut self) -> Result<Vec<Param>> {
        self.expect(&TokenKind::LParen, "expected '(' before parameters")?;
        self.parse_params_after_lparen()
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.at_end() {
            self.skip_newlines();
            if self.check(&TokenKind::DotDotDot) {
                let spread_pos = self.peek().pos.clone();
                self.advance();
                let expr = self.assignment()?;
                args.push(Expr::Spread { expr: Box::new(expr), pos: spread_pos });
            } else {
                args.push(self.assignment()?);
            }
            self.skip_newlines();
            if !self.match_tok(&TokenKind::Comma) {
                break;
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
        } else if self.match_tok(&TokenKind::StarStarEq) {
            Some(CompoundOp::Power)
        } else if self.match_tok(&TokenKind::AmpEq) {
            Some(CompoundOp::BitAnd)
        } else if self.match_tok(&TokenKind::PipeEq) {
            Some(CompoundOp::BitOr)
        } else if self.match_tok(&TokenKind::CaretEq) {
            Some(CompoundOp::BitXor)
        } else if self.match_tok(&TokenKind::LtLtEq) {
            Some(CompoundOp::ShiftLeft)
        } else if self.match_tok(&TokenKind::GtGtEq) {
            Some(CompoundOp::ShiftRight)
        } else if self.match_tok(&TokenKind::AndAndEq) {
            Some(CompoundOp::LogicalAnd)
        } else if self.match_tok(&TokenKind::OrOrEq) {
            Some(CompoundOp::LogicalOr)
        } else if self.match_tok(&TokenKind::QuestionQuestionEq) {
            Some(CompoundOp::NullCoalesce)
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
            } => {}
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
    fn test_ternary() {
        match parse_expr("x > 0 ? 1 : -1") {
            Expr::Ternary { .. } => {}
            _ => panic!("expected ternary"),
        }
    }

    #[test]
    fn test_null_coalesce() {
        match parse_expr("x ?? 0") {
            Expr::Binary { op: BinaryOp::NullCoalesce, .. } => {}
            _ => panic!("expected null coalesce"),
        }
    }

    #[test]
    fn test_power() {
        match parse_expr("2 ** 10") {
            Expr::Binary { op: BinaryOp::Power, .. } => {}
            _ => panic!("expected power"),
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
    fn test_do_while_stmt() {
        match parse_stmt("do { x -= 1; } while (x > 0);") {
            Stmt::DoWhile { .. } => {}
            _ => panic!("expected do-while"),
        }
    }

    #[test]
    fn test_function_decl() {
        match parse_stmt("function add(a, b) { return a + b; }") {
            Stmt::Function { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "a");
                assert_eq!(params[1].name, "b");
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_function_with_defaults() {
        match parse_stmt("function greet(name, greeting = \"Hello\") { return greeting; }") {
            Stmt::Function { params, .. } => {
                assert_eq!(params.len(), 2);
                assert!(params[0].default.is_none());
                assert!(params[1].default.is_some());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_arrow_function() {
        match parse_expr("x => x * 2") {
            Expr::Lambda { params, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "x");
            }
            _ => panic!("expected lambda"),
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

    #[test]
    fn test_optional_chain() {
        match parse_expr("obj?.name") {
            Expr::OptionalProp { name, .. } => assert_eq!(name, "name"),
            _ => panic!("expected optional prop"),
        }
    }

    #[test]
    fn test_try_catch() {
        match parse_stmt("try { let x = 1; } catch (e) { print e; }") {
            Stmt::TryCatch { catch_var: Some(v), .. } => assert_eq!(v, "e"),
            _ => panic!("expected try-catch"),
        }
    }

    #[test]
    fn test_throw_stmt() {
        match parse_stmt("throw \"oops\";") {
            Stmt::Throw { .. } => {}
            _ => panic!("expected throw"),
        }
    }
}
