use crate::ast::*;
use crate::error::{CustomLangError, Result};
use crate::lexer::{Lexer, Token, TokenKind};

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

    // ─── STATEMENTS ───────────────────────────────────────────────────────────

    fn statement(&mut self) -> Result<Stmt> {
        // Decorator: @name
        if self.check(&TokenKind::At) {
            return self.decorator_stmt();
        }
        match &self.peek().kind {
            TokenKind::Let => self.let_stmt(),
            TokenKind::If => self.if_stmt(),
            TokenKind::While => self.while_stmt(),
            TokenKind::Do => self.do_while_stmt(),
            TokenKind::For => self.for_stmt(),
            TokenKind::Function | TokenKind::Async => self.function_stmt_top(),
            TokenKind::Return => self.return_stmt(),
            TokenKind::Break => self.break_stmt(),
            TokenKind::Continue => self.continue_stmt(),
            TokenKind::Print => self.print_stmt(),
            TokenKind::Import => self.import_stmt(),
            TokenKind::Export => self.export_stmt(),
            TokenKind::Class => self.class_stmt(),
            TokenKind::LBrace => self.block_stmt(),
            TokenKind::Try => self.try_stmt(),
            TokenKind::Throw => self.throw_stmt(),
            TokenKind::Enum => self.enum_stmt(),
            TokenKind::Type => self.type_alias_stmt(),
            TokenKind::Interface => self.interface_stmt(),
            // Labeled statement: ident ':'
            TokenKind::Ident(_) if self.peek2_is(&TokenKind::Colon) => self.labeled_stmt(),
            _ => self.expr_stmt(),
        }
    }

    fn decorator_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos; // @
        let name = self.expect_ident("expected decorator name")?;
        // optional (args) - skip for now
        if self.check(&TokenKind::LParen) {
            self.advance();
            let mut depth = 1;
            while !self.at_end() && depth > 0 {
                match &self.peek().kind {
                    TokenKind::LParen => {
                        depth += 1;
                        self.advance();
                    }
                    TokenKind::RParen => {
                        depth -= 1;
                        self.advance();
                    }
                    _ => {
                        self.advance();
                    }
                }
            }
        }
        self.skip_newlines();
        let target = Box::new(self.statement()?);
        Ok(Stmt::Decorator { name, target, pos })
    }

    fn labeled_stmt(&mut self) -> Result<Stmt> {
        let pos = self.peek().pos;
        let label = self.expect_ident("expected label")?;
        self.advance(); // consume ':'
        self.skip_newlines();
        let body = Box::new(self.statement()?);
        Ok(Stmt::Labeled { label, body, pos })
    }

    fn let_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        // Array destructuring: let [a, b] = ...
        if self.check(&TokenKind::LBracket) {
            return self.let_destruct_array(pos);
        }
        // Object destructuring: let {a, b} = ...
        if self.check(&TokenKind::LBrace) {
            return self.let_destruct_object(pos);
        }
        let name = self.expect_ident("expected variable name after 'let'")?;
        // Skip optional type annotation: let x: Type = ...
        if self.match_tok(&TokenKind::Colon) {
            self.skip_type_annotation()?;
        }
        let init = if self.match_tok(&TokenKind::Eq) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume_terminator()?;
        Ok(Stmt::Let { name, init, pos })
    }

    fn let_destruct_array(&mut self, pos: Position) -> Result<Stmt> {
        self.advance(); // consume '['
        let mut elems = Vec::new();
        while !self.check(&TokenKind::RBracket) && !self.at_end() {
            if self.check(&TokenKind::Comma) {
                elems.push(DestructElem::Skip);
                self.advance();
                continue;
            }
            if self.match_tok(&TokenKind::DotDotDot) {
                let name = self.expect_ident("expected rest variable name")?;
                elems.push(DestructElem::Rest(name));
                break;
            }
            let name = self.expect_ident("expected variable name in destructuring")?;
            let default = if self.match_tok(&TokenKind::Eq) {
                Some(self.assignment()?)
            } else {
                None
            };
            elems.push(DestructElem::Bind { name, default });
            if !self.match_tok(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBracket, "expected ']'")?;
        self.expect(&TokenKind::Eq, "expected '=' in destructuring")?;
        let init = self.expression()?;
        self.consume_terminator()?;
        Ok(Stmt::LetDestructArray { elems, init, pos })
    }

    fn let_destruct_object(&mut self, pos: Position) -> Result<Stmt> {
        self.advance(); // consume '{'
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            let key = self.expect_ident("expected key in object destructuring")?;
            let alias = if self.match_tok(&TokenKind::Colon) {
                Some(self.expect_ident("expected alias")?)
            } else {
                None
            };
            let default = if self.match_tok(&TokenKind::Eq) {
                Some(self.assignment()?)
            } else {
                None
            };
            fields.push(DestructField {
                key,
                alias,
                default,
            });
            if !self.match_tok(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "expected '}'")?;
        self.expect(&TokenKind::Eq, "expected '=' in destructuring")?;
        let init = self.expression()?;
        self.consume_terminator()?;
        Ok(Stmt::LetDestructObject { fields, init, pos })
    }

    fn skip_type_annotation(&mut self) -> Result<()> {
        // Skip tokens until we see = or newline or { or ;
        let mut depth = 0i32;
        loop {
            match &self.peek().kind {
                TokenKind::Eq
                | TokenKind::Newline
                | TokenKind::Semicolon
                | TokenKind::LBrace
                | TokenKind::Eof => break,
                // A top-level comma ends this parameter's type; commas inside
                // generics/tuples (e.g. Map<string, number>) are at depth > 0.
                TokenKind::Comma if depth == 0 => break,
                TokenKind::Lt | TokenKind::LParen | TokenKind::LBracket => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::Gt | TokenKind::RParen | TokenKind::RBracket => {
                    if depth > 0 {
                        depth -= 1;
                        self.advance();
                    } else {
                        break;
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
        Ok(())
    }

    fn if_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        self.expect(&TokenKind::LParen, "expected '(' after 'if'")?;
        let cond = self.expression()?;
        self.expect(&TokenKind::RParen, "expected ')'")?;
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
        let pos = self.advance().pos;
        self.expect(&TokenKind::LParen, "expected '('")?;
        let cond = self.expression()?;
        self.expect(&TokenKind::RParen, "expected ')'")?;
        self.skip_newlines();
        let body = Box::new(self.statement()?);
        Ok(Stmt::While { cond, body, pos })
    }

    fn do_while_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        self.skip_newlines();
        let body = Box::new(self.statement()?);
        self.skip_newlines();
        if !self.check(&TokenKind::While) {
            return Err(self.parse_err("expected 'while'"));
        }
        self.advance();
        self.expect(&TokenKind::LParen, "expected '('")?;
        let cond = self.expression()?;
        self.expect(&TokenKind::RParen, "expected ')'")?;
        self.consume_terminator()?;
        Ok(Stmt::DoWhile { body, cond, pos })
    }

    fn for_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        self.expect(&TokenKind::LParen, "expected '('")?;

        // for (x in expr) or for (x of expr)
        if self.peek_is_ident() {
            if self.peek2_is(&TokenKind::In) {
                let var = self.expect_ident("expected variable")?;
                self.advance(); // in
                let iter = self.expression()?;
                self.expect(&TokenKind::RParen, "expected ')'")?;
                self.skip_newlines();
                let body = Box::new(self.statement()?);
                return Ok(Stmt::ForIn {
                    var,
                    iter,
                    body,
                    pos,
                });
            }
            if self.peek2_is(&TokenKind::Of) {
                let var = self.expect_ident("expected variable")?;
                self.advance(); // of
                let iter = self.expression()?;
                self.expect(&TokenKind::RParen, "expected ')'")?;
                self.skip_newlines();
                let body = Box::new(self.statement()?);
                return Ok(Stmt::ForOf {
                    var,
                    iter,
                    body,
                    pos,
                });
            }
        }

        // C-style for
        let init = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(Box::new(self.for_init()?))
        };
        self.expect(&TokenKind::Semicolon, "expected ';'")?;
        let cond = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&TokenKind::Semicolon, "expected ';'")?;
        let update = if self.check(&TokenKind::RParen) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&TokenKind::RParen, "expected ')'")?;
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
        let pos = self.advance().pos;
        let name = self.expect_ident("expected variable name")?;
        if self.match_tok(&TokenKind::Colon) {
            self.skip_type_annotation()?;
        }
        let init = if self.match_tok(&TokenKind::Eq) {
            Some(self.expression()?)
        } else {
            None
        };
        Ok(Stmt::Let { name, init, pos })
    }

    fn expr_stmt_no_term(&mut self) -> Result<Stmt> {
        let expr = self.expression()?;
        let pos = *expr.pos();
        Ok(Stmt::Expr { expr, pos })
    }

    fn function_stmt_top(&mut self) -> Result<Stmt> {
        let is_async = self.match_tok(&TokenKind::Async);
        self.function_stmt_inner(false, is_async)
    }

    fn function_stmt_inner(&mut self, is_static: bool, is_async: bool) -> Result<Stmt> {
        let pos = self.advance().pos; // 'function'
        let is_generator = self.match_tok(&TokenKind::Star);
        let name = self.expect_ident_or_keyword()?;
        let params = self.parse_params()?;
        // skip return type annotation
        if self.match_tok(&TokenKind::Arrow) {
            self.skip_type_annotation()?;
        }
        self.skip_newlines();
        let body = Box::new(self.block_stmt()?);
        Ok(Stmt::Function {
            name,
            params,
            body,
            is_static,
            is_generator,
            is_async,
            pos,
        })
    }

    fn return_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
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

    fn break_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        let label = if self.peek_is_ident()
            && !self.check(&TokenKind::Newline)
            && !self.check(&TokenKind::Semicolon)
        {
            Some(self.expect_ident("expected label")?)
        } else {
            None
        };
        self.consume_terminator()?;
        Ok(Stmt::Break { label, pos })
    }

    fn continue_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        let label = if self.peek_is_ident()
            && !self.check(&TokenKind::Newline)
            && !self.check(&TokenKind::Semicolon)
        {
            Some(self.expect_ident("expected label")?)
        } else {
            None
        };
        self.consume_terminator()?;
        Ok(Stmt::Continue { label, pos })
    }

    fn print_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        let expr = self.expression()?;
        self.consume_terminator()?;
        Ok(Stmt::Print { expr, pos })
    }

    fn import_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        // Selective: import { a, b as c } from "module"
        if self.check(&TokenKind::LBrace) {
            self.advance();
            let mut names = Vec::new();
            while !self.check(&TokenKind::RBrace) && !self.at_end() {
                let name = self.expect_ident("expected import name")?;
                let alias = if self.match_ident("as") {
                    Some(self.expect_ident("expected alias")?)
                } else {
                    None
                };
                names.push((name, alias));
                if !self.match_tok(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RBrace, "expected '}'")?;
            let _ = self.match_tok(&TokenKind::From) || self.match_ident("from");
            let path = match self.peek().kind.clone() {
                TokenKind::Str(s) => {
                    self.advance();
                    s
                }
                _ => return Err(self.parse_err("expected module path")),
            };
            self.consume_terminator()?;
            return Ok(Stmt::Import {
                path,
                alias: None,
                names,
                pos,
            });
        }
        // import * as name from "module"
        if self.match_tok(&TokenKind::Star) {
            self.match_ident("as");
            let alias = Some(self.expect_ident("expected namespace name")?);
            let _ = self.match_tok(&TokenKind::From) || self.match_ident("from");
            let path = match self.peek().kind.clone() {
                TokenKind::Str(s) => {
                    self.advance();
                    s
                }
                _ => return Err(self.parse_err("expected module path")),
            };
            self.consume_terminator()?;
            return Ok(Stmt::Import {
                path,
                alias,
                names: vec![],
                pos,
            });
        }
        // Regular: import "path" or import "path" as alias
        let path = match self.peek().kind.clone() {
            TokenKind::Str(s) => {
                self.advance();
                s
            }
            _ => return Err(self.parse_err("expected string path")),
        };
        let alias = if self.match_ident("as") {
            Some(self.expect_ident("expected alias")?)
        } else {
            None
        };
        self.consume_terminator()?;
        Ok(Stmt::Import {
            path,
            alias,
            names: vec![],
            pos,
        })
    }

    fn export_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        // export function / export let / export class
        if self.check(&TokenKind::Function)
            || self.check(&TokenKind::Async)
            || self.check(&TokenKind::Class)
        {
            let inner = self.statement()?;
            // wrap in export - extract name
            let name = match &inner {
                Stmt::Function { name, .. } | Stmt::Class { name, .. } => name.clone(),
                _ => "default".to_string(),
            };
            let _inner_name = name.clone();
            // Just produce export for the name
            return Ok(Stmt::Export { name, pos });
        }
        let name = self.expect_ident("expected identifier to export")?;
        self.consume_terminator()?;
        Ok(Stmt::Export { name, pos })
    }

    fn class_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        let name = self.expect_ident("expected class name")?;
        let super_name = if self.match_tok(&TokenKind::Extends) {
            Some(self.expect_ident("expected superclass name")?)
        } else {
            None
        };
        self.expect(&TokenKind::LBrace, "expected '{'")?;
        let mut methods = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) || self.at_end() {
                break;
            }
            // Decorator on method
            if self.check(&TokenKind::At) {
                methods.push(self.decorator_stmt()?);
                continue;
            }
            // Handle 'private' keyword (parsed as ident)
            let _is_private = if let TokenKind::Ident(id) = &self.peek().kind {
                if id == "private" {
                    self.advance();
                    true
                } else {
                    false
                }
            } else {
                false
            };
            let is_static = self.match_tok(&TokenKind::Static);
            let is_async = self.match_tok(&TokenKind::Async);
            // getter: get name() { }
            if self.check(&TokenKind::Get) && self.peek2_is_ident() {
                let gpos = self.advance().pos;
                let gname = self.expect_ident("expected getter name")?;
                self.expect(&TokenKind::LParen, "expected '('")?;
                self.expect(&TokenKind::RParen, "expected ')'")?;
                self.skip_newlines();
                let body = Box::new(self.block_stmt()?);
                let fd_name = format!("__get_{gname}__");
                methods.push(Stmt::Function {
                    name: fd_name,
                    params: vec![],
                    body,
                    is_static,
                    is_generator: false,
                    is_async,
                    pos: gpos,
                });
                continue;
            }
            // setter: set name(v) { }
            if self.check(&TokenKind::Set) && self.peek2_is_ident() {
                let spos = self.advance().pos;
                let sname = self.expect_ident("expected setter name")?;
                self.expect(&TokenKind::LParen, "expected '('")?;
                let param = self.expect_ident("expected setter parameter")?;
                self.expect(&TokenKind::RParen, "expected ')'")?;
                self.skip_newlines();
                let body = Box::new(self.block_stmt()?);
                let fd_name = format!("__set_{sname}__");
                methods.push(Stmt::Function {
                    name: fd_name,
                    params: vec![Param::simple(param)],
                    body,
                    is_static,
                    is_generator: false,
                    is_async,
                    pos: spos,
                });
                continue;
            }
            // Static field: static PI = 3.14;
            if is_static && self.peek_is_ident() && !self.check(&TokenKind::Function) {
                let fpos = self.peek().pos;
                let fname = self.expect_ident("expected field name")?;
                let init = if self.match_tok(&TokenKind::Eq) {
                    self.expression()?
                } else {
                    Expr::Literal {
                        value: Value::Null,
                        pos: fpos,
                    }
                };
                self.consume_terminator()?;
                // encode as a function named __static_field_name__ that returns the value
                let body = Box::new(Stmt::Return {
                    value: Some(init),
                    pos: fpos,
                });
                methods.push(Stmt::Function {
                    name: format!("__static_field_{fname}__"),
                    params: vec![],
                    body,
                    is_static: true,
                    is_generator: false,
                    is_async: false,
                    pos: fpos,
                });
                continue;
            }
            if self.check(&TokenKind::Function) {
                methods.push(self.function_stmt_inner(is_static, is_async)?);
            } else if !is_static && self.peek_is_ident() {
                // Instance field: fieldname = value; (private or public)
                let fpos = self.peek().pos;
                let fname = self.expect_ident("expected field name")?;
                let init = if self.match_tok(&TokenKind::Eq) {
                    self.expression()?
                } else {
                    Expr::Literal {
                        value: Value::Null,
                        pos: fpos,
                    }
                };
                self.consume_terminator()?;
                let body = Box::new(Stmt::Return {
                    value: Some(init),
                    pos: fpos,
                });
                methods.push(Stmt::Function {
                    name: format!("__field_{fname}__"),
                    params: vec![],
                    body,
                    is_static: false,
                    is_generator: false,
                    is_async: false,
                    pos: fpos,
                });
            } else {
                return Err(self.parse_err("expected method in class body"));
            }
        }
        self.expect(&TokenKind::RBrace, "expected '}'")?;
        Ok(Stmt::Class {
            name,
            super_name,
            methods,
            pos,
        })
    }

    fn try_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        self.skip_newlines();
        let try_b = Box::new(self.block_stmt()?);
        let mut catch_var = None;
        let mut catch_b = None;
        let mut finally_b = None;
        self.skip_newlines();
        if self.check(&TokenKind::Catch) {
            self.advance();
            if self.match_tok(&TokenKind::LParen) {
                catch_var = Some(self.expect_ident("expected catch var")?);
                self.expect(&TokenKind::RParen, "expected ')'")?;
            }
            self.skip_newlines();
            catch_b = Some(Box::new(self.block_stmt()?));
        }
        self.skip_newlines();
        if self.check(&TokenKind::Finally) {
            self.advance();
            self.skip_newlines();
            finally_b = Some(Box::new(self.block_stmt()?));
        }
        if catch_b.is_none() && finally_b.is_none() {
            return Err(self.parse_err("try requires catch or finally"));
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
        let pos = self.advance().pos;
        let value = self.expression()?;
        self.consume_terminator()?;
        Ok(Stmt::Throw { value, pos })
    }

    fn enum_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        let name = self.expect_ident("expected enum name")?;
        self.expect(&TokenKind::LBrace, "expected '{'")?;
        let mut variants = Vec::new();
        let mut counter = 0i64;
        loop {
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) || self.at_end() {
                break;
            }
            let vname = self.expect_ident("expected variant name")?;
            let val = if self.match_tok(&TokenKind::Eq) {
                Some(self.expression()?)
            } else {
                let v = Expr::Literal {
                    value: Value::Number(counter as f64),
                    pos,
                };
                counter += 1;
                Some(v)
            };
            variants.push((vname, val));
            if !self.match_tok(&TokenKind::Comma) {
                self.skip_newlines();
            }
        }
        self.expect(&TokenKind::RBrace, "expected '}'")?;
        Ok(Stmt::Enum {
            name,
            variants,
            pos,
        })
    }

    fn type_alias_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        let name = self.expect_ident("expected type alias name")?;
        self.match_tok(&TokenKind::Eq);
        self.skip_type_annotation()?;
        self.consume_terminator()?;
        Ok(Stmt::TypeAlias { name, pos })
    }

    fn interface_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        let name = self.expect_ident("expected interface name")?;
        // skip the whole body
        if self.check(&TokenKind::LBrace) {
            let mut depth = 0;
            loop {
                match &self.peek().kind {
                    TokenKind::LBrace => {
                        depth += 1;
                        self.advance();
                    }
                    TokenKind::RBrace => {
                        depth -= 1;
                        self.advance();
                        if depth == 0 {
                            break;
                        }
                    }
                    TokenKind::Eof => break,
                    _ => {
                        self.advance();
                    }
                }
            }
        }
        Ok(Stmt::Interface { name, pos })
    }

    fn block_stmt(&mut self) -> Result<Stmt> {
        let pos = self.advance().pos;
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) || self.at_end() {
                break;
            }
            stmts.push(self.statement()?);
        }
        self.expect(&TokenKind::RBrace, "expected '}'")?;
        Ok(Stmt::Block { stmts, pos })
    }

    fn expr_stmt(&mut self) -> Result<Stmt> {
        let expr = self.expression()?;
        let pos = *expr.pos();
        self.consume_terminator()?;
        Ok(Stmt::Expr { expr, pos })
    }

    // ─── EXPRESSIONS ──────────────────────────────────────────────────────────

    fn expression(&mut self) -> Result<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr> {
        let expr = self.ternary()?;
        if let Some(op) = self.match_compound_op() {
            let pos = self.prev_pos();
            let rhs = self.assignment()?;
            return match &expr {
                Expr::Var { name, .. } => Ok(Expr::CompoundAssign {
                    name: name.clone(),
                    op,
                    value: Box::new(rhs),
                    pos,
                }),
                Expr::Prop { object, name, .. } => {
                    let read = Expr::Prop {
                        object: object.clone(),
                        name: name.clone(),
                        pos,
                    };
                    let combined = Expr::Binary {
                        left: Box::new(read),
                        op: op.to_binary(),
                        right: Box::new(rhs),
                        pos,
                    };
                    Ok(Expr::PropAssign {
                        object: object.clone(),
                        prop: name.clone(),
                        value: Box::new(combined),
                        pos,
                    })
                }
                Expr::Index { object, index, .. } => {
                    let read = Expr::Index {
                        object: object.clone(),
                        index: index.clone(),
                        pos,
                    };
                    let combined = Expr::Binary {
                        left: Box::new(read),
                        op: op.to_binary(),
                        right: Box::new(rhs),
                        pos,
                    };
                    Ok(Expr::IndexAssign {
                        object: object.clone(),
                        index: index.clone(),
                        value: Box::new(combined),
                        pos,
                    })
                }
                _ => Err(CustomLangError::parse(
                    pos.line,
                    pos.column,
                    "invalid compound assignment target",
                )),
            };
        }
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
        let expr = self.pipe()?;
        if self.match_tok(&TokenKind::Question) {
            let pos = self.prev_pos();
            let then_e = Box::new(self.expression()?);
            self.expect(&TokenKind::Colon, "expected ':' in ternary")?;
            let else_e = Box::new(self.ternary()?);
            return Ok(Expr::Ternary {
                cond: Box::new(expr),
                then_e,
                else_e,
                pos,
            });
        }
        Ok(expr)
    }

    fn pipe(&mut self) -> Result<Expr> {
        let mut expr = self.logical_or()?;
        while self.match_tok(&TokenKind::PipeArrow) {
            let pos = self.prev_pos();
            let right = self.logical_or()?;
            expr = Expr::Pipe {
                left: Box::new(expr),
                right: Box::new(right),
                pos,
            };
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
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseOr,
                right: Box::new(self.bitwise_xor()?),
                pos,
            };
        }
        Ok(expr)
    }

    fn bitwise_xor(&mut self) -> Result<Expr> {
        let mut expr = self.bitwise_and()?;
        while self.match_tok(&TokenKind::Caret) {
            let pos = self.prev_pos();
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseXor,
                right: Box::new(self.bitwise_and()?),
                pos,
            };
        }
        Ok(expr)
    }

    fn bitwise_and(&mut self) -> Result<Expr> {
        let mut expr = self.equality()?;
        while self.match_tok(&TokenKind::Amp) {
            let pos = self.prev_pos();
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitwiseAnd,
                right: Box::new(self.equality()?),
                pos,
            };
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr> {
        let mut expr = self.relational()?;
        while let Some(op) = self.match_eq_op() {
            let pos = self.prev_pos();
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(self.relational()?),
                pos,
            };
        }
        Ok(expr)
    }

    fn relational(&mut self) -> Result<Expr> {
        let mut expr = self.shift()?;
        loop {
            let pos = self.peek().pos;
            if let Some(op) = self.match_cmp_op() {
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(self.shift()?),
                    pos,
                };
            } else if self.match_tok(&TokenKind::In) {
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::In,
                    right: Box::new(self.shift()?),
                    pos,
                };
            } else if self.match_tok(&TokenKind::Instanceof) || self.match_ident("is") {
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Instanceof,
                    right: Box::new(self.shift()?),
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
            let pos = self.peek().pos;
            if self.match_tok(&TokenKind::LtLt) {
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::ShiftLeft,
                    right: Box::new(self.term()?),
                    pos,
                };
            } else if self.match_tok(&TokenKind::GtGtGt) {
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::ShiftRightU,
                    right: Box::new(self.term()?),
                    pos,
                };
            } else if self.match_tok(&TokenKind::GtGt) {
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::ShiftRight,
                    right: Box::new(self.term()?),
                    pos,
                };
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
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(self.factor()?),
                pos,
            };
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr> {
        let mut expr = self.power()?;
        while let Some(op) = self.match_mul_op() {
            let pos = self.prev_pos();
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(self.power()?),
                pos,
            };
        }
        Ok(expr)
    }

    fn power(&mut self) -> Result<Expr> {
        let base = self.unary()?;
        if self.match_tok(&TokenKind::StarStar) {
            let pos = self.prev_pos();
            return Ok(Expr::Binary {
                left: Box::new(base),
                op: BinaryOp::Power,
                right: Box::new(self.power()?),
                pos,
            });
        }
        Ok(base)
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.match_tok(&TokenKind::Bang) {
            let pos = self.prev_pos();
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(self.unary()?),
                pos,
            });
        }
        if self.match_tok(&TokenKind::Minus) {
            let pos = self.prev_pos();
            if let TokenKind::Number(n) = self.peek().kind.clone() {
                self.advance();
                return Ok(Expr::Literal {
                    value: Value::Number(-n),
                    pos,
                });
            }
            return Ok(Expr::Unary {
                op: UnaryOp::Minus,
                expr: Box::new(self.unary()?),
                pos,
            });
        }
        if self.match_tok(&TokenKind::Tilde) {
            let pos = self.prev_pos();
            return Ok(Expr::Unary {
                op: UnaryOp::BitwiseNot,
                expr: Box::new(self.unary()?),
                pos,
            });
        }
        if self.match_ident("typeof") {
            let pos = self.prev_pos();
            return Ok(Expr::Unary {
                op: UnaryOp::Typeof,
                expr: Box::new(self.unary()?),
                pos,
            });
        }
        if self.match_tok(&TokenKind::DotDotDot) {
            let pos = self.prev_pos();
            return Ok(Expr::Spread {
                expr: Box::new(self.unary()?),
                pos,
            });
        }
        if self.match_tok(&TokenKind::Yield) {
            let pos = self.prev_pos();
            let value = if self.check(&TokenKind::Newline)
                || self.check(&TokenKind::Semicolon)
                || self.check(&TokenKind::RBrace)
            {
                None
            } else {
                Some(Box::new(self.assignment()?))
            };
            return Ok(Expr::Yield { value, pos });
        }
        if self.match_tok(&TokenKind::Await) {
            let pos = self.prev_pos();
            return Ok(Expr::Await {
                expr: Box::new(self.unary()?),
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
                self.expect(&TokenKind::RBracket, "expected ']'")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    pos,
                };
            } else if self.match_tok(&TokenKind::Dot) {
                let pos = self.prev_pos();
                let name = self.expect_ident_or_keyword()?;
                expr = Expr::Prop {
                    object: Box::new(expr),
                    name,
                    pos,
                };
            } else if self.match_tok(&TokenKind::QuestionDot) {
                let pos = self.prev_pos();
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
                    let name = self.expect_ident_or_keyword()?;
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
        // Arrow: ident =>
        if self.peek_is_ident() && self.peek2_is(&TokenKind::Arrow) {
            let pos = self.peek().pos;
            let name = self.expect_ident("expected param")?;
            self.advance(); // =>
            return self.parse_arrow_body(vec![Param::simple(name)], pos);
        }
        let tok = self.advance();
        let pos = tok.pos;
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
                if self.is_arrow_params_lookahead() {
                    let params = self.parse_params_after_lparen()?;
                    self.expect(&TokenKind::Arrow, "expected '=>'")?;
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
                        let sp = self.peek().pos;
                        self.advance();
                        let e = self.assignment()?;
                        elements.push(Expr::Spread {
                            expr: Box::new(e),
                            pos: sp,
                        });
                    } else {
                        elements.push(self.assignment()?);
                    }
                    self.skip_newlines();
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&TokenKind::RBracket, "expected ']'")?;
                Ok(Expr::Array { elements, pos })
            }
            TokenKind::LBrace => {
                let mut pairs = Vec::new();
                self.skip_newlines();
                while !self.check(&TokenKind::RBrace) && !self.at_end() {
                    if self.check(&TokenKind::DotDotDot) {
                        let sp = self.peek().pos;
                        self.advance();
                        let expr = self.assignment()?;
                        pairs.push((
                            ObjectKey::Static("__spread__".to_string()),
                            Expr::Spread {
                                expr: Box::new(expr),
                                pos: sp,
                            },
                        ));
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
                self.expect(&TokenKind::RBrace, "expected '}'")?;
                Ok(Expr::Object { pairs, pos })
            }
            TokenKind::New => {
                let class_expr = Box::new(self.call()?);
                match *class_expr {
                    Expr::Call {
                        callee,
                        args,
                        pos: cp,
                    } => Ok(Expr::New {
                        class: callee,
                        args,
                        pos: cp,
                    }),
                    other => Ok(Expr::New {
                        class: Box::new(other),
                        args: vec![],
                        pos,
                    }),
                }
            }
            TokenKind::Function => {
                let is_generator = self.match_tok(&TokenKind::Star);
                // optional name
                let _name = if self.peek_is_ident() {
                    Some(self.expect_ident("expected name")?)
                } else {
                    None
                };
                let params = self.parse_params()?;
                if self.match_tok(&TokenKind::Arrow) {
                    self.skip_type_annotation()?;
                }
                self.skip_newlines();
                let body = Box::new(self.block_stmt()?);
                Ok(Expr::Lambda {
                    params,
                    body: if is_generator {
                        Box::new(Stmt::Block {
                            stmts: vec![*body],
                            pos,
                        })
                    } else {
                        body
                    },
                    pos,
                })
            }
            TokenKind::Match => self.match_expr(pos),
            _ => Err(CustomLangError::parse(
                pos.line,
                pos.column,
                format!("unexpected token '{kind_display}'"),
            )),
        }
    }

    fn parse_object_pair(&mut self) -> Result<(ObjectKey, Expr)> {
        let pp = self.peek().pos;
        if self.match_tok(&TokenKind::LBracket) {
            let key_expr = self.expression()?;
            self.expect(&TokenKind::RBracket, "expected ']'")?;
            self.expect(&TokenKind::Colon, "expected ':'")?;
            return Ok((ObjectKey::Computed(Box::new(key_expr)), self.expression()?));
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
            // method shorthand: { greet() { ... } }
            _ => return Err(self.parse_err("expected object key")),
        };
        // Method shorthand: key(params) { body }
        if self.check(&TokenKind::LParen) {
            let params = self.parse_params()?;
            self.skip_newlines();
            let body = Box::new(self.block_stmt()?);
            let fd = Expr::Lambda {
                params,
                body,
                pos: pp,
            };
            return Ok((ObjectKey::Static(key), fd));
        }
        // Shorthand: { name } → { name: name }
        if !self.check(&TokenKind::Colon) {
            return Ok((
                ObjectKey::Static(key.clone()),
                Expr::Var { name: key, pos: pp },
            ));
        }
        self.advance(); // :
        Ok((ObjectKey::Static(key), self.expression()?))
    }

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
                        pos,
                    });
                    current.clear();
                }
                i += 2;
                let mut depth = 1usize;
                let mut expr_src = String::new();
                while i < chars.len() {
                    match chars[i] {
                        '{' => {
                            depth += 1;
                            expr_src.push('{');
                        }
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                            expr_src.push('}');
                        }
                        c => expr_src.push(c),
                    }
                    i += 1;
                }
                let tokens = Lexer::new(&expr_src).tokenize().map_err(|e| {
                    CustomLangError::parse(pos.line, pos.column, format!("template: {e}"))
                })?;
                let expr = Parser::new(tokens).expression().map_err(|e| {
                    CustomLangError::parse(pos.line, pos.column, format!("template: {e}"))
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
                pos,
            });
        }
        if parts.is_empty() {
            return Ok(Expr::Literal {
                value: Value::Str(String::new()),
                pos,
            });
        }
        let mut result = parts.remove(0);
        for part in parts {
            result = Expr::Binary {
                left: Box::new(result),
                op: BinaryOp::Add,
                right: Box::new(part),
                pos,
            };
        }
        Ok(result)
    }

    fn parse_arrow_body(&mut self, params: Vec<Param>, pos: Position) -> Result<Expr> {
        self.skip_newlines();
        if self.check(&TokenKind::LBrace) {
            Ok(Expr::Lambda {
                params,
                body: Box::new(self.block_stmt()?),
                pos,
            })
        } else {
            let expr = self.assignment()?;
            let ep = *expr.pos();
            Ok(Expr::Lambda {
                params,
                body: Box::new(Stmt::Return {
                    value: Some(expr),
                    pos: ep,
                }),
                pos,
            })
        }
    }

    fn is_arrow_params_lookahead(&self) -> bool {
        let mut depth = 1;
        let mut i = self.cur;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
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

    fn parse_params_after_lparen(&mut self) -> Result<Vec<Param>> {
        let mut params = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.at_end() {
            self.skip_newlines();
            let is_rest = self.match_tok(&TokenKind::DotDotDot);
            let name = self.expect_ident("expected parameter")?;
            if self.match_tok(&TokenKind::Colon) {
                self.skip_type_annotation()?;
            }
            let default = if !is_rest && self.match_tok(&TokenKind::Eq) {
                Some(self.assignment()?)
            } else {
                None
            };
            params.push(Param {
                name,
                default,
                is_rest,
            });
            if is_rest {
                break;
            }
            if !self.match_tok(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RParen, "expected ')'")?;
        Ok(params)
    }

    fn match_expr(&mut self, pos: Position) -> Result<Expr> {
        let expr = Box::new(self.expression()?);
        self.expect(&TokenKind::LBrace, "expected '{'")?;
        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) || self.at_end() {
                break;
            }
            let pattern = self.parse_pattern()?;
            let guard = if self.match_ident("when") {
                Some(self.expression()?)
            } else {
                None
            };
            self.expect(&TokenKind::Arrow, "expected '=>'")?;
            let body = self.expression()?;
            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });
            self.skip_newlines();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            if !self.match_tok(&TokenKind::Comma) {
                return Err(self.parse_err("expected ',' between match arms"));
            }
        }
        self.expect(&TokenKind::RBrace, "expected '}'")?;
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
                    let key = self.expect_ident("expected key")?;
                    self.expect(&TokenKind::Colon, "expected ':'")?;
                    pairs.push((key, self.parse_pattern()?));
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

    // ─── HELPERS ──────────────────────────────────────────────────────────────

    fn parse_params(&mut self) -> Result<Vec<Param>> {
        self.expect(&TokenKind::LParen, "expected '('")?;
        self.parse_params_after_lparen()
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        let mut named_pairs: Vec<(ObjectKey, Expr)> = Vec::new();
        let mut in_named = false;
        self.skip_newlines();
        while !self.check(&TokenKind::RParen) && !self.at_end() {
            self.skip_newlines();
            // Detect named arg: bare Ident followed by ':'
            let is_named = if let TokenKind::Ident(_) = &self.peek().kind {
                std::mem::discriminant(&self.peek2().kind)
                    == std::mem::discriminant(&TokenKind::Colon)
            } else {
                false
            };
            if is_named {
                in_named = true;
                let name = if let TokenKind::Ident(n) = self.advance().kind.clone() {
                    n
                } else {
                    unreachable!()
                };
                self.advance(); // consume ':'
                let val = self.assignment()?;
                let pos = *val.pos();
                named_pairs.push((ObjectKey::Static(name), val));
                let _ = pos;
            } else if in_named {
                return Err(self.parse_err("positional argument after named argument"));
            } else if self.check(&TokenKind::DotDotDot) {
                let sp = self.peek().pos;
                self.advance();
                let e = self.assignment()?;
                args.push(Expr::Spread {
                    expr: Box::new(e),
                    pos: sp,
                });
            } else {
                args.push(self.assignment()?);
            }
            self.skip_newlines();
            if !self.match_tok(&TokenKind::Comma) {
                break;
            }
        }
        self.skip_newlines();
        self.expect(&TokenKind::RParen, "expected ')'")?;
        // If there were named args, encode as a special sentinel object at end
        if !named_pairs.is_empty() {
            let pos = self.prev_pos();
            let sentinel = (
                ObjectKey::Static("__named__".to_string()),
                Expr::Literal {
                    value: Value::Bool(true),
                    pos,
                },
            );
            let mut pairs = vec![sentinel];
            pairs.extend(named_pairs);
            args.push(Expr::Object { pairs, pos });
        }
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
            self.tokens[self.cur - 1].pos
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
    fn peek2_is_ident(&self) -> bool {
        matches!(&self.peek2().kind, TokenKind::Ident(_))
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
    // Allow keywords as property names after '.'
    fn expect_ident_or_keyword(&mut self) -> Result<String> {
        let kind = self.peek().kind.clone();
        let name = match kind {
            TokenKind::Ident(name) => {
                self.advance();
                return Ok(name);
            }
            TokenKind::Let => "let",
            TokenKind::If => "if",
            TokenKind::Else => "else",
            TokenKind::While => "while",
            TokenKind::For => "for",
            TokenKind::In => "in",
            TokenKind::Of => "of",
            TokenKind::Break => "break",
            TokenKind::Continue => "continue",
            TokenKind::Function => "function",
            TokenKind::Return => "return",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::Null => "null",
            TokenKind::Print => "print",
            TokenKind::Import => "import",
            TokenKind::Export => "export",
            TokenKind::Class => "class",
            TokenKind::Extends => "extends",
            TokenKind::This => "this",
            TokenKind::New => "new",
            TokenKind::Match => "match",
            TokenKind::Do => "do",
            TokenKind::Throw => "throw",
            TokenKind::Try => "try",
            TokenKind::Catch => "catch",
            TokenKind::Finally => "finally",
            TokenKind::Super => "super",
            TokenKind::Static => "static",
            TokenKind::Instanceof => "instanceof",
            TokenKind::Yield => "yield",
            TokenKind::Async => "async",
            TokenKind::Await => "await",
            TokenKind::Type => "type",
            TokenKind::Enum => "enum",
            TokenKind::Interface => "interface",
            TokenKind::From => "from",
            TokenKind::Get => "get",
            TokenKind::Set => "set",
            _ => return Err(self.parse_err("expected property name")),
        };
        self.advance();
        Ok(name.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    fn parse_expr(src: &str) -> Expr {
        let t = Lexer::new(src).tokenize().unwrap();
        Parser::new(t).expression().unwrap()
    }
    fn parse_stmt(src: &str) -> Stmt {
        let t = Lexer::new(src).tokenize().unwrap();
        Parser::new(t).statement().unwrap()
    }
    #[test]
    fn test_number_literal() {
        match parse_expr("42") {
            Expr::Literal {
                value: Value::Number(n),
                ..
            } => assert_eq!(n, 42.0),
            _ => panic!(),
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
            _ => panic!(),
        }
    }
    #[test]
    fn test_binary_add() {
        match parse_expr("2 + 3") {
            Expr::Binary {
                op: BinaryOp::Add, ..
            } => {}
            _ => panic!(),
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
                _ => panic!(),
            },
            _ => panic!(),
        }
    }
    #[test]
    fn test_ternary() {
        match parse_expr("x > 0 ? 1 : -1") {
            Expr::Ternary { .. } => {}
            _ => panic!(),
        }
    }
    #[test]
    fn test_null_coalesce() {
        match parse_expr("x ?? 0") {
            Expr::Binary {
                op: BinaryOp::NullCoalesce,
                ..
            } => {}
            _ => panic!(),
        }
    }
    #[test]
    fn test_power() {
        match parse_expr("2 ** 10") {
            Expr::Binary {
                op: BinaryOp::Power,
                ..
            } => {}
            _ => panic!(),
        }
    }
    #[test]
    fn test_let_stmt() {
        match parse_stmt("let x = 42;") {
            Stmt::Let { name, .. } => assert_eq!(name, "x"),
            _ => panic!(),
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
            _ => panic!(),
        }
    }
    #[test]
    fn test_for_in() {
        match parse_stmt("for (item in arr) { print item; }") {
            Stmt::ForIn { var, .. } => assert_eq!(var, "item"),
            _ => panic!(),
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
            _ => panic!(),
        }
    }
    #[test]
    fn test_while_stmt() {
        match parse_stmt("while (x > 0) { x -= 1; }") {
            Stmt::While { .. } => {}
            _ => panic!(),
        }
    }
    #[test]
    fn test_do_while_stmt() {
        match parse_stmt("do { x -= 1; } while (x > 0);") {
            Stmt::DoWhile { .. } => {}
            _ => panic!(),
        }
    }
    #[test]
    fn test_function_decl() {
        match parse_stmt("function add(a, b) { return a + b; }") {
            Stmt::Function { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "a");
            }
            _ => panic!(),
        }
    }
    #[test]
    fn test_function_with_defaults() {
        match parse_stmt("function greet(name, greeting = \"Hello\") { return greeting; }") {
            Stmt::Function { params, .. } => {
                assert_eq!(params.len(), 2);
                assert!(params[1].default.is_some());
            }
            _ => panic!(),
        }
    }
    #[test]
    fn test_arrow_function() {
        match parse_expr("x => x * 2") {
            Expr::Lambda { params, .. } => {
                assert_eq!(params[0].name, "x");
            }
            _ => panic!(),
        }
    }
    #[test]
    fn test_index_assign() {
        match parse_expr("arr[0] = 42") {
            Expr::IndexAssign { .. } => {}
            _ => panic!(),
        }
    }
    #[test]
    fn test_prop_assign() {
        match parse_expr("obj.field = 42") {
            Expr::PropAssign { prop, .. } => assert_eq!(prop, "field"),
            _ => panic!(),
        }
    }
    #[test]
    fn test_optional_chain() {
        match parse_expr("obj?.name") {
            Expr::OptionalProp { name, .. } => assert_eq!(name, "name"),
            _ => panic!(),
        }
    }
    #[test]
    fn test_try_catch() {
        match parse_stmt("try { let x = 1; } catch (e) { print e; }") {
            Stmt::TryCatch {
                catch_var: Some(v), ..
            } => assert_eq!(v, "e"),
            _ => panic!(),
        }
    }
    #[test]
    fn test_throw_stmt() {
        match parse_stmt("throw \"oops\";") {
            Stmt::Throw { .. } => {}
            _ => panic!(),
        }
    }
    #[test]
    fn test_enum() {
        match parse_stmt("enum Dir { North, South }") {
            Stmt::Enum { name, variants, .. } => {
                assert_eq!(name, "Dir");
                assert_eq!(variants.len(), 2);
            }
            _ => panic!(),
        }
    }
    #[test]
    fn test_destruct_array() {
        match parse_stmt("let [a, b] = arr;") {
            Stmt::LetDestructArray { elems, .. } => assert_eq!(elems.len(), 2),
            _ => panic!(),
        }
    }
    #[test]
    fn test_destruct_object() {
        match parse_stmt("let {x, y} = point;") {
            Stmt::LetDestructObject { fields, .. } => assert_eq!(fields.len(), 2),
            _ => panic!(),
        }
    }
    #[test]
    fn test_labeled() {
        match parse_stmt("outer: for (let i = 0; i < 5; i += 1) { break outer; }") {
            Stmt::Labeled { label, .. } => assert_eq!(label, "outer"),
            _ => panic!(),
        }
    }
    #[test]
    fn test_pipe_operator() {
        match parse_expr("x |> fn") {
            Expr::Pipe { .. } => {}
            _ => panic!(),
        }
    }
}
