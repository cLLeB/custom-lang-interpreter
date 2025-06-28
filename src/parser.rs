//! # Syntax Analysis (Parsing)
//!
//! Recursive descent parser that converts tokens into an Abstract Syntax Tree (AST).
//! Implements the grammar rules for the Custom Language and provides detailed
//! error reporting for syntax errors.
//!
//! ## Grammar Overview
//! - Statements: variable declarations, expressions, control flow
//! - Expressions: arithmetic, logical, function calls, assignments
//! - Precedence: follows standard mathematical operator precedence
//! - Error Recovery: attempts to continue parsing after errors when possible

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
            TokenType::Import => self.import_statement(),
            TokenType::Export => self.export_statement(),
            TokenType::Class => self.class_declaration(),
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

    fn import_statement(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'import'

        // Parse module path (string literal)
        let module_path = match self.peek().token_type.clone() {
            TokenType::String(path) => {
                self.advance();
                path
            }
            _ => {
                return Err(CustomLangError::parse_error(
                    self.peek().position.line,
                    self.peek().position.column,
                    "Expected string literal for module path",
                ))
            }
        };

        // Optional alias: import "module" as alias
        let alias = if self.match_token(&TokenType::Identifier("as".to_string())) {
            match self.peek().token_type.clone() {
                TokenType::Identifier(name) => {
                    self.advance();
                    Some(name)
                }
                _ => {
                    return Err(CustomLangError::parse_error(
                        self.peek().position.line,
                        self.peek().position.column,
                        "Expected identifier after 'as'",
                    ))
                }
            }
        } else {
            None
        };

        self.consume_semicolon_or_newline()?;
        Ok(Stmt::Import {
            module_path,
            alias,
            pos,
        })
    }

    fn export_statement(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'export'

        // Parse exported name (identifier)
        let name = match self.peek().token_type.clone() {
            TokenType::Identifier(name) => {
                self.advance();
                name
            }
            _ => {
                return Err(CustomLangError::parse_error(
                    self.peek().position.line,
                    self.peek().position.column,
                    "Expected identifier for export name",
                ))
            }
        };

        self.consume_semicolon_or_newline()?;
        Ok(Stmt::Export { name, pos })
    }

    fn class_declaration(&mut self) -> Result<Stmt> {
        let pos = self.advance().position.clone(); // consume 'class'

        // Parse class name
        let name = match self.peek().token_type.clone() {
            TokenType::Identifier(name) => {
                self.advance();
                name
            }
            _ => {
                return Err(CustomLangError::parse_error(
                    self.peek().position.line,
                    self.peek().position.column,
                    "Expected class name",
                ))
            }
        };

        // Optional inheritance: class Child extends Parent
        let superclass = if self.match_token(&TokenType::Extends) {
            match self.peek().token_type.clone() {
                TokenType::Identifier(superclass_name) => {
                    self.advance();
                    Some(superclass_name)
                }
                _ => {
                    return Err(CustomLangError::parse_error(
                        self.peek().position.line,
                        self.peek().position.column,
                        "Expected superclass name after 'extends'",
                    ))
                }
            }
        } else {
            None
        };

        // Parse class body
        self.consume(&TokenType::LeftBrace, "Expected '{' before class body")?;

        let mut methods = Vec::new();
        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Skip newlines inside class body
            if self.check(&TokenType::Newline) {
                self.advance();
                continue;
            }

            // Parse method (must be a function)
            if self.check(&TokenType::Function) {
                methods.push(self.function_declaration()?);
            } else {
                return Err(CustomLangError::parse_error(
                    self.peek().position.line,
                    self.peek().position.column,
                    "Expected method declaration in class body",
                ));
            }
        }

        self.consume(&TokenType::RightBrace, "Expected '}' after class body")?;
        Ok(Stmt::Class {
            name,
            superclass,
            methods,
            pos,
        })
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
            } else if self.match_token(&TokenType::Dot) {
                let pos = self.previous().position.clone();
                let property = match self.peek().token_type.clone() {
                    TokenType::Identifier(name) => {
                        self.advance();
                        name
                    }
                    _ => {
                        return Err(CustomLangError::parse_error(
                            self.peek().position.line,
                            self.peek().position.column,
                            "Expected property name after '.'",
                        ))
                    }
                };
                expr = Expr::PropertyAccess {
                    object: Box::new(expr),
                    property,
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
            TokenType::LeftBrace => {
                let mut pairs = Vec::new();

                if !self.check(&TokenType::RightBrace) {
                    loop {
                        // Parse key (must be identifier or string)
                        let key = match self.peek().token_type.clone() {
                            TokenType::Identifier(name) => {
                                self.advance();
                                name
                            }
                            TokenType::String(s) => {
                                self.advance();
                                s
                            }
                            _ => {
                                return Err(CustomLangError::parse_error(
                                    self.peek().position.line,
                                    self.peek().position.column,
                                    "Expected property name (identifier or string)",
                                ))
                            }
                        };

                        self.consume(&TokenType::Colon, "Expected ':' after property name")?;
                        let value = self.expression()?;
                        pairs.push((key, value));

                        if !self.match_token(&TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.consume(
                    &TokenType::RightBrace,
                    "Expected '}' after object properties",
                )?;
                Ok(Expr::Object { pairs, pos })
            }
            TokenType::New => {
                // Parse: new ClassName(args...)
                let class_name = match self.peek().token_type.clone() {
                    TokenType::Identifier(name) => {
                        self.advance();
                        name
                    }
                    _ => {
                        return Err(CustomLangError::parse_error(
                            self.peek().position.line,
                            self.peek().position.column,
                            "Expected class name after 'new'",
                        ))
                    }
                };

                self.consume(&TokenType::LeftParen, "Expected '(' after class name")?;

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
                Ok(Expr::New {
                    class_name,
                    args,
                    pos,
                })
            }
            TokenType::This => Ok(Expr::This { pos }),
            TokenType::Match => self.match_expression(pos),
            _ => Err(CustomLangError::parse_error(
                pos.line,
                pos.column,
                "Expected expression",
            )),
        }
    }

    fn match_expression(&mut self, pos: Position) -> Result<Expr> {
        // Parse: match expr { pattern => body, ... }
        let expr = Box::new(self.expression()?);

        self.consume(&TokenType::LeftBrace, "Expected '{' after match expression")?;

        let mut arms = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            // Skip newlines
            if self.check(&TokenType::Newline) {
                self.advance();
                continue;
            }

            // Parse pattern
            let pattern = self.parse_pattern()?;

            // Expect arrow
            self.consume(&TokenType::Arrow, "Expected '=>' after pattern")?;

            // Parse body expression
            let body = self.expression()?;

            arms.push(MatchArm { pattern, body });

            // Skip any newlines
            while self.check(&TokenType::Newline) {
                self.advance();
            }

            // Check for end of match first
            if self.check(&TokenType::RightBrace) {
                break;
            }

            // Expect comma if not at end
            if !self.match_token(&TokenType::Comma) {
                return Err(CustomLangError::parse_error(
                    self.peek().position.line,
                    self.peek().position.column,
                    "Expected ',' after match arm",
                ));
            }
        }

        self.consume(&TokenType::RightBrace, "Expected '}' after match arms")?;

        Ok(Expr::Match { expr, arms, pos })
    }

    fn parse_pattern(&mut self) -> Result<Pattern> {
        let token = self.advance();

        match &token.token_type {
            TokenType::Number(n) => Ok(Pattern::Literal(Value::Number(*n))),
            TokenType::String(s) => Ok(Pattern::Literal(Value::String(s.clone()))),
            TokenType::True => Ok(Pattern::Literal(Value::Boolean(true))),
            TokenType::False => Ok(Pattern::Literal(Value::Boolean(false))),
            TokenType::Null => Ok(Pattern::Literal(Value::Null)),
            TokenType::Underscore => Ok(Pattern::Wildcard),
            TokenType::Identifier(name) => Ok(Pattern::Variable(name.clone())),
            TokenType::LeftBracket => {
                // Array pattern: [pattern1, pattern2, ...]
                let mut patterns = Vec::new();

                if !self.check(&TokenType::RightBracket) {
                    loop {
                        patterns.push(self.parse_pattern()?);
                        if !self.match_token(&TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.consume(&TokenType::RightBracket, "Expected ']' after array pattern")?;
                Ok(Pattern::Array(patterns))
            }
            TokenType::LeftBrace => {
                // Object pattern: {key: pattern, ...}
                let mut pairs = Vec::new();

                if !self.check(&TokenType::RightBrace) {
                    loop {
                        let key = match self.peek().token_type.clone() {
                            TokenType::Identifier(name) => {
                                self.advance();
                                name
                            }
                            TokenType::String(s) => {
                                self.advance();
                                s
                            }
                            _ => {
                                return Err(CustomLangError::parse_error(
                                    self.peek().position.line,
                                    self.peek().position.column,
                                    "Expected property name in object pattern",
                                ))
                            }
                        };

                        self.consume(
                            &TokenType::Colon,
                            "Expected ':' after property name in pattern",
                        )?;
                        let pattern = self.parse_pattern()?;
                        pairs.push((key, pattern));

                        if !self.match_token(&TokenType::Comma) {
                            break;
                        }
                    }
                }

                self.consume(&TokenType::RightBrace, "Expected '}' after object pattern")?;
                Ok(Pattern::Object(pairs))
            }
            _ => Err(CustomLangError::parse_error(
                token.position.line,
                token.position.column,
                "Expected pattern",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_expression(source: &str) -> Result<Expr> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.expression()
    }

    fn parse_statement(source: &str) -> Result<Stmt> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.statement()
    }

    #[test]
    fn test_parse_number_literal() {
        let expr = parse_expression("42").unwrap();
        match expr {
            Expr::Literal {
                value: Value::Number(n),
                ..
            } => assert_eq!(n, 42.0),
            _ => panic!("Expected number literal"),
        }
    }

    #[test]
    fn test_parse_string_literal() {
        let expr = parse_expression(r#""hello""#).unwrap();
        match expr {
            Expr::Literal {
                value: Value::String(s),
                ..
            } => assert_eq!(s, "hello"),
            _ => panic!("Expected string literal"),
        }
    }

    #[test]
    fn test_parse_binary_expression() {
        let expr = parse_expression("2 + 3").unwrap();
        match expr {
            Expr::Binary {
                op: BinaryOp::Add, ..
            } => {}
            _ => panic!("Expected binary addition expression"),
        }
    }

    #[test]
    fn test_parse_variable_declaration() {
        let stmt = parse_statement("let x = 42;").unwrap();
        match stmt {
            Stmt::VarDeclaration { name, .. } => assert_eq!(name, "x"),
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn test_parse_function_declaration() {
        let stmt = parse_statement("function add(a, b) { return a + b; }").unwrap();
        match stmt {
            Stmt::Function { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params, vec!["a", "b"]);
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_parse_if_statement() {
        let stmt = parse_statement("if (x > 0) { print x; }").unwrap();
        match stmt {
            Stmt::If { .. } => {}
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        let expr = parse_expression("2 + 3 * 4").unwrap();
        // Should parse as 2 + (3 * 4), not (2 + 3) * 4
        match expr {
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => {
                // Left should be 2
                match left.as_ref() {
                    Expr::Literal {
                        value: Value::Number(n),
                        ..
                    } => assert_eq!(*n, 2.0),
                    _ => panic!("Expected number literal on left"),
                }
                // Right should be 3 * 4
                match right.as_ref() {
                    Expr::Binary {
                        op: BinaryOp::Multiply,
                        ..
                    } => {}
                    _ => panic!("Expected multiplication on right"),
                }
            }
            _ => panic!("Expected addition at top level"),
        }
    }
}
