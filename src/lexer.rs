//! # Lexical Analysis (Tokenization)
//!
//! The lexer converts raw source code into a stream of tokens that can be
//! processed by the parser. It handles:
//! - Keywords and identifiers
//! - Numeric and string literals
//! - Operators and punctuation
//! - Comments and whitespace
//! - Error reporting for invalid characters

use crate::ast::Position;
use crate::error::{CustomLangError, Result};

/// Token types for the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Number(f64),
    String(String),
    Identifier(String),

    // Keywords
    Let,
    If,
    Else,
    While,
    Function,
    Return,
    True,
    False,
    Null,
    Print,
    Import,
    Export,
    Class,
    Extends,
    This,
    New,
    Match,
    Arrow,      // =>
    Underscore, // _

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Dot,
    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    AndAnd,
    OrOr,

    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Semicolon,
    Colon,

    // Special
    Newline,
    Eof,
}

/// Token with position information
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub position: Position,
}

impl Token {
    pub fn new(token_type: TokenType, position: Position) -> Self {
        Self {
            token_type,
            position,
        }
    }
}

/// Lexer for tokenizing source code
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();

            if self.is_at_end() {
                break;
            }

            let start_line = self.line;
            let start_column = self.column;
            let pos = Position::new(start_line, start_column);

            match self.advance() {
                '+' => tokens.push(Token::new(TokenType::Plus, pos)),
                '-' => tokens.push(Token::new(TokenType::Minus, pos)),
                '*' => tokens.push(Token::new(TokenType::Star, pos)),
                '.' => tokens.push(Token::new(TokenType::Dot, pos)),
                '/' => {
                    if self.peek() == '/' {
                        // Line comment
                        self.advance(); // consume second '/'
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        tokens.push(Token::new(TokenType::Slash, pos));
                    }
                }
                '%' => tokens.push(Token::new(TokenType::Percent, pos)),
                '(' => tokens.push(Token::new(TokenType::LeftParen, pos)),
                ')' => tokens.push(Token::new(TokenType::RightParen, pos)),
                '{' => tokens.push(Token::new(TokenType::LeftBrace, pos)),
                '}' => tokens.push(Token::new(TokenType::RightBrace, pos)),
                '[' => tokens.push(Token::new(TokenType::LeftBracket, pos)),
                ']' => tokens.push(Token::new(TokenType::RightBracket, pos)),
                ',' => tokens.push(Token::new(TokenType::Comma, pos)),
                ';' => tokens.push(Token::new(TokenType::Semicolon, pos)),
                ':' => tokens.push(Token::new(TokenType::Colon, pos)),
                '\n' => {
                    tokens.push(Token::new(TokenType::Newline, pos));
                    self.line += 1;
                    self.column = 1;
                    continue;
                }
                '=' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenType::EqualEqual, pos));
                    } else if self.peek() == '>' {
                        self.advance();
                        tokens.push(Token::new(TokenType::Arrow, pos));
                    } else {
                        tokens.push(Token::new(TokenType::Equal, pos));
                    }
                }
                '!' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenType::BangEqual, pos));
                    } else {
                        tokens.push(Token::new(TokenType::Bang, pos));
                    }
                }
                '<' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenType::LessEqual, pos));
                    } else {
                        tokens.push(Token::new(TokenType::Less, pos));
                    }
                }
                '>' => {
                    if self.peek() == '=' {
                        self.advance();
                        tokens.push(Token::new(TokenType::GreaterEqual, pos));
                    } else {
                        tokens.push(Token::new(TokenType::Greater, pos));
                    }
                }
                '&' => {
                    if self.peek() == '&' {
                        self.advance();
                        tokens.push(Token::new(TokenType::AndAnd, pos));
                    } else {
                        return Err(CustomLangError::lex_error(
                            start_line,
                            start_column,
                            "Unexpected character '&'",
                        ));
                    }
                }
                '|' => {
                    if self.peek() == '|' {
                        self.advance();
                        tokens.push(Token::new(TokenType::OrOr, pos));
                    } else {
                        return Err(CustomLangError::lex_error(
                            start_line,
                            start_column,
                            "Unexpected character '|'",
                        ));
                    }
                }
                '"' => {
                    let string_value = self.read_string()?;
                    tokens.push(Token::new(TokenType::String(string_value), pos));
                }
                c if c.is_ascii_digit() => {
                    let number = self.read_number(c)?;
                    tokens.push(Token::new(TokenType::Number(number), pos));
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let identifier = self.read_identifier(c);
                    let token_type = match identifier.as_str() {
                        "let" => TokenType::Let,
                        "if" => TokenType::If,
                        "else" => TokenType::Else,
                        "while" => TokenType::While,
                        "function" => TokenType::Function,
                        "return" => TokenType::Return,
                        "true" => TokenType::True,
                        "false" => TokenType::False,
                        "null" => TokenType::Null,
                        "print" => TokenType::Print,
                        "import" => TokenType::Import,
                        "export" => TokenType::Export,
                        "class" => TokenType::Class,
                        "extends" => TokenType::Extends,
                        "this" => TokenType::This,
                        "new" => TokenType::New,
                        "match" => TokenType::Match,
                        "_" => TokenType::Underscore,
                        _ => TokenType::Identifier(identifier),
                    };
                    tokens.push(Token::new(token_type, pos));
                }
                c => {
                    return Err(CustomLangError::lex_error(
                        start_line,
                        start_column,
                        format!("Unexpected character '{c}'"),
                    ));
                }
            }
        }

        tokens.push(Token::new(
            TokenType::Eof,
            Position::new(self.line, self.column),
        ));
        Ok(tokens)
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    fn advance(&mut self) -> char {
        let c = self.input[self.position];
        self.position += 1;
        self.column += 1;
        c
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self) -> Result<String> {
        let mut value = String::new();
        let start_line = self.line;
        let start_column = self.column - 1;

        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
                self.column = 1;
            }
            value.push(self.advance());
        }

        if self.is_at_end() {
            return Err(CustomLangError::lex_error(
                start_line,
                start_column,
                "Unterminated string",
            ));
        }

        // Consume closing quote
        self.advance();
        Ok(value)
    }

    fn read_number(&mut self, first_digit: char) -> Result<f64> {
        let mut number_str = String::new();
        number_str.push(first_digit);

        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            number_str.push(self.advance());
        }

        number_str.parse::<f64>().map_err(|_| {
            CustomLangError::lex_error(
                self.line,
                self.column - number_str.len(),
                format!("Invalid number: {number_str}"),
            )
        })
    }

    fn read_identifier(&mut self, first_char: char) -> String {
        let mut identifier = String::new();
        identifier.push(first_char);

        while !self.is_at_end() && (self.peek().is_ascii_alphanumeric() || self.peek() == '_') {
            identifier.push(self.advance());
        }

        identifier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_numbers() {
        let mut lexer = Lexer::new("42 2.5 0.5");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 4); // 3 numbers + EOF
        match &tokens[0].token_type {
            TokenType::Number(n) => assert_eq!(*n, 42.0),
            _ => panic!("Expected number token"),
        }
        match &tokens[1].token_type {
            TokenType::Number(n) => assert_eq!(*n, 2.5),
            _ => panic!("Expected number token"),
        }
    }

    #[test]
    fn test_tokenize_strings() {
        let mut lexer = Lexer::new(r#""hello" "world""#);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 3); // 2 strings + EOF
        match &tokens[0].token_type {
            TokenType::String(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected string token"),
        }
    }

    #[test]
    fn test_tokenize_identifiers() {
        let mut lexer = Lexer::new("variable_name function123");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 3); // 2 identifiers + EOF
        match &tokens[0].token_type {
            TokenType::Identifier(name) => assert_eq!(name, "variable_name"),
            _ => panic!("Expected identifier token"),
        }
    }

    #[test]
    fn test_tokenize_keywords() {
        let mut lexer = Lexer::new("let if else while function return");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 7); // 6 keywords + EOF
        assert_eq!(tokens[0].token_type, TokenType::Let);
        assert_eq!(tokens[1].token_type, TokenType::If);
        assert_eq!(tokens[2].token_type, TokenType::Else);
    }

    #[test]
    fn test_tokenize_operators() {
        let mut lexer = Lexer::new("+ - * / == != < > <= >=");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 11); // 10 operators + EOF
        assert_eq!(tokens[0].token_type, TokenType::Plus);
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[4].token_type, TokenType::EqualEqual);
    }

    #[test]
    fn test_position_tracking() {
        let mut lexer = Lexer::new("let x = 42;\nlet y = 3.14;");
        let tokens = lexer.tokenize().unwrap();

        // Check that positions are tracked correctly
        assert_eq!(tokens[0].position.line, 1);
        assert_eq!(tokens[0].position.column, 1);

        // Find token on second line
        let second_line_token = tokens.iter().find(|t| t.position.line == 2).unwrap();
        assert_eq!(second_line_token.position.line, 2);
    }
}
