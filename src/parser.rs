use crate::ast::*;
use crate::error::{CustomLangError, Result};
use crate::lexer::{Token, TokenType};

/// Recursive descent parser for the custom language
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            // Skip newlines at the top level
            if self.check(&TokenType::Newline) {
                self.advance();
                continue;
            }

            statements.push(self.statement()?);
        }

        Ok(Program::new(statements))
    }

    fn statement(&mut self) -> Result<Stmt> {
        match &self.peek().token_type {
            TokenType::Let => self.var_declaration(),
            TokenType::If => self.if_statement(),
            TokenType::While => self.while_statement(),
            TokenType::Function => self.function_declaration(),
            TokenType::Return => self.return_statement(),
            TokenType::Print => self.print_statement(),
            TokenType::LeftBrace => self.block_statement(),
            _ => self.expression_statement(),
        }
    }

    fn var_declaration(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'let'

        let name = if let TokenType::Identifier(name) = &self.advance().token_type {
            name.clone()
        } else {
            return Err(CustomLangError::parse_error(
                pos.line,
                pos.column,
                "Expected variable name after 'let'",
            ));
        };

        let initializer = if self.match_token(&TokenType::Equal) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume_semicolon_or_newline()?;
        Ok(Stmt::VarDeclaration {
            name,
            initializer,
            pos,
        })
    }

    fn if_statement(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'if'

        self.consume(&TokenType::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.expression()?;
        self.consume(&TokenType::RightParen, "Expected ')' after if condition")?;

        let then_stmt = Box::new(self.statement()?);
        let else_stmt = if self.match_token(&TokenType::Else) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_stmt,
            else_stmt,
            pos,
        })
    }

    fn while_statement(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'while'

        self.consume(&TokenType::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.expression()?;
        self.consume(&TokenType::RightParen, "Expected ')' after while condition")?;

        let body = Box::new(self.statement()?);

        Ok(Stmt::While {
            condition,
            body,
            pos,
        })
    }

    fn function_declaration(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'function'

        let name = if let TokenType::Identifier(name) = &self.advance().token_type {
            name.clone()
        } else {
            return Err(CustomLangError::parse_error(
                pos.line,
                pos.column,
                "Expected function name",
            ));
        };

        self.consume(&TokenType::LeftParen, "Expected '(' after function name")?;

        let mut params = Vec::new();
        if !self.check(&TokenType::RightParen) {
            loop {
                if let TokenType::Identifier(param) = &self.advance().token_type {
                    params.push(param.clone());
                } else {
                    return Err(CustomLangError::parse_error(
                        self.previous().position.line,
                        self.previous().position.column,
                        "Expected parameter name",
                    ));
                }

                if !self.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(&TokenType::RightParen, "Expected ')' after parameters")?;
        let body = Box::new(self.statement()?);

        Ok(Stmt::Function {
            name,
            params,
            body,
            pos,
        })
    }

    fn return_statement(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'return'

        let value = if self.check(&TokenType::Semicolon) || self.check(&TokenType::Newline) {
            None
        } else {
            Some(self.expression()?)
        };

        self.consume_semicolon_or_newline()?;
        Ok(Stmt::Return { value, pos })
    }

    fn print_statement(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'print'

        let expr = self.expression()?;
        self.consume_semicolon_or_newline()?;

        Ok(Stmt::Print { expr, pos })
    }

    fn block_statement(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume '{'

        let mut statements = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Skip newlines inside blocks
            if self.check(&TokenType::Newline) {
                self.advance();
                continue;
            }
            statements.push(self.statement()?);
        }

        self.consume(&TokenType::RightBrace, "Expected '}' after block")?;
        Ok(Stmt::Block { statements, pos })
    }

    fn expression_statement(&mut self) -> Result<Stmt> {
        let expr = self.expression()?;
        let pos = expr.position().clone();
        self.consume_semicolon_or_newline()?;
        Ok(Stmt::Expression { expr, pos })
    }

    fn expression(&mut self) -> Result<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr> {
        let expr = self.or()?;

        if self.match_token(&TokenType::Equal) {
            let pos = self.previous().position.clone();
            let value = self.assignment()?;

            if let Expr::Identifier { name, .. } = expr {
                return Ok(Expr::Assignment {
                    name,
                    value: Box::new(value),
                    pos,
                });
            }

            return Err(CustomLangError::parse_error(
                pos.line,
                pos.column,
                "Invalid assignment target",
            ));
        }

        Ok(expr)
    }

    fn or(&mut self) -> Result<Expr> {
        let mut expr = self.and()?;

        while self.match_token(&TokenType::OrOr) {
            let pos = self.previous().position.clone();
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

        while self.match_token(&TokenType::AndAnd) {
            let pos = self.previous().position.clone();
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

        while let Some(op) = self.match_equality_op() {
            let pos = self.previous().position.clone();
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

        while let Some(op) = self.match_comparison_op() {
            let pos = self.previous().position.clone();
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

        while let Some(op) = self.match_term_op() {
            let pos = self.previous().position.clone();
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

        while let Some(op) = self.match_factor_op() {
            let pos = self.previous().position.clone();
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
        if let Some(op) = self.match_unary_op() {
            let pos = self.previous().position.clone();
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
                pos,
            });
        }

        self.call()
    }

    fn call(&mut self) -> Result<Expr> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(&TokenType::LeftParen) {
                let pos = self.previous().position.clone();
                let mut args = Vec::new();

                if !self.check(&TokenType::RightParen) {
                    loop {
                        args.push(self.expression()?);
                        if !self.match_token(&TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.consume(&TokenType::RightParen, "Expected ')' after arguments")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    pos,
                };
            } else if self.match_token(&TokenType::LeftBracket) {
                let pos = self.previous().position.clone();
                let index = self.expression()?;
                self.consume(&TokenType::RightBracket, "Expected ']' after array index")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                    pos,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr> {
        let token = self.advance();
        let pos = token.position.clone();

        match &token.token_type {
            TokenType::True => Ok(Expr::Literal {
                value: Value::Boolean(true),
                pos,
            }),
            TokenType::False => Ok(Expr::Literal {
                value: Value::Boolean(false),
                pos,
            }),
            TokenType::Null => Ok(Expr::Literal {
                value: Value::Null,
                pos,
            }),
            TokenType::Number(n) => Ok(Expr::Literal {
                value: Value::Number(*n),
                pos,
            }),
            TokenType::String(s) => Ok(Expr::Literal {
                value: Value::String(s.clone()),
                pos,
            }),
            TokenType::Identifier(name) => Ok(Expr::Identifier {
                name: name.clone(),
                pos,
            }),
            TokenType::LeftParen => {
                let expr = self.expression()?;
                self.consume(&TokenType::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            TokenType::LeftBracket => {
                let mut elements = Vec::new();

                if !self.check(&TokenType::RightBracket) {
                    loop {
                        elements.push(self.expression()?);
                        if !self.match_token(&TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.consume(
                    &TokenType::RightBracket,
                    "Expected ']' after array elements",
                )?;
                Ok(Expr::Array { elements, pos })
            }
            _ => Err(CustomLangError::parse_error(
                pos.line,
                pos.column,
                "Expected expression",
            )),
        }
    }

    // Helper methods
    fn match_equality_op(&mut self) -> Option<BinaryOp> {
        if self.match_token(&TokenType::EqualEqual) {
            Some(BinaryOp::Equal)
        } else if self.match_token(&TokenType::BangEqual) {
            Some(BinaryOp::NotEqual)
        } else {
            None
        }
    }

    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        if self.match_token(&TokenType::Greater) {
            Some(BinaryOp::Greater)
        } else if self.match_token(&TokenType::GreaterEqual) {
            Some(BinaryOp::GreaterEqual)
        } else if self.match_token(&TokenType::Less) {
            Some(BinaryOp::Less)
        } else if self.match_token(&TokenType::LessEqual) {
            Some(BinaryOp::LessEqual)
        } else {
            None
        }
    }

    fn match_term_op(&mut self) -> Option<BinaryOp> {
        if self.match_token(&TokenType::Plus) {
            Some(BinaryOp::Add)
        } else if self.match_token(&TokenType::Minus) {
            Some(BinaryOp::Subtract)
        } else {
            None
        }
    }

    fn match_factor_op(&mut self) -> Option<BinaryOp> {
        if self.match_token(&TokenType::Star) {
            Some(BinaryOp::Multiply)
        } else if self.match_token(&TokenType::Slash) {
            Some(BinaryOp::Divide)
        } else if self.match_token(&TokenType::Percent) {
            Some(BinaryOp::Modulo)
        } else {
            None
        }
    }

    fn match_unary_op(&mut self) -> Option<UnaryOp> {
        if self.match_token(&TokenType::Bang) {
            Some(UnaryOp::Not)
        } else if self.match_token(&TokenType::Minus) {
            Some(UnaryOp::Minus)
        } else {
            None
        }
    }

    fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, token_type: &TokenType, message: &str) -> Result<&Token> {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            let pos = self.peek().position.clone();
            Err(CustomLangError::parse_error(pos.line, pos.column, message))
        }
    }

    fn consume_semicolon_or_newline(&mut self) -> Result<()> {
        if self.check(&TokenType::Semicolon) || self.check(&TokenType::Newline) || self.is_at_end()
        {
            if !self.is_at_end() {
                self.advance();
            }
            Ok(())
        } else {
            let pos = self.peek().position.clone();
            Err(CustomLangError::parse_error(
                pos.line,
                pos.column,
                "Expected ';' or newline",
            ))
        }
    }
}
