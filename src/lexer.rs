use crate::ast::Position;
use crate::error::{CustomLangError, Result};

/// All token kinds produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    Str(String),
    Ident(String),

    // Keywords
    Let,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
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

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Dot,
    Eq,        // =
    EqEq,      // ==
    Bang,      // !
    BangEq,    // !=
    Lt,        // <
    LtEq,      // <=
    Gt,        // >
    GtEq,      // >=
    AndAnd,    // &&
    OrOr,      // ||
    Arrow,     // =>
    PlusEq,    // +=
    MinusEq,   // -=
    StarEq,    // *=
    SlashEq,   // /=
    PercentEq, // %=

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: Position,
}

impl Token {
    pub fn new(kind: TokenKind, pos: Position) -> Self {
        Self { kind, pos }
    }
}

pub struct Lexer {
    src: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            src: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();
            if self.at_end() {
                break;
            }

            let line = self.line;
            let col = self.col;
            let p = Position::new(line, col);

            let ch = self.advance();

            match ch {
                '\n' => {
                    tokens.push(Token::new(TokenKind::Newline, p));
                    self.line += 1;
                    self.col = 1;
                    continue; // col was already incremented by advance; reset
                }
                '+' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::PlusEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Plus, p));
                    }
                }
                '-' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::MinusEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Minus, p));
                    }
                }
                '*' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::StarEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Star, p));
                    }
                }
                '/' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::SlashEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Slash, p));
                    }
                }
                '%' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::PercentEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Percent, p));
                    }
                }
                '.' => tokens.push(Token::new(TokenKind::Dot, p)),
                '(' => tokens.push(Token::new(TokenKind::LParen, p)),
                ')' => tokens.push(Token::new(TokenKind::RParen, p)),
                '{' => tokens.push(Token::new(TokenKind::LBrace, p)),
                '}' => tokens.push(Token::new(TokenKind::RBrace, p)),
                '[' => tokens.push(Token::new(TokenKind::LBracket, p)),
                ']' => tokens.push(Token::new(TokenKind::RBracket, p)),
                ',' => tokens.push(Token::new(TokenKind::Comma, p)),
                ';' => tokens.push(Token::new(TokenKind::Semicolon, p)),
                ':' => tokens.push(Token::new(TokenKind::Colon, p)),
                '=' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::EqEq, p));
                    } else if self.peek_is('>') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::Arrow, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Eq, p));
                    }
                }
                '!' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::BangEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Bang, p));
                    }
                }
                '<' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::LtEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Lt, p));
                    }
                }
                '>' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::GtEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Gt, p));
                    }
                }
                '&' => {
                    if self.peek_is('&') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::AndAnd, p));
                    } else {
                        return Err(CustomLangError::lex(
                            line,
                            col,
                            "unexpected '&'; did you mean '&&'?",
                        ));
                    }
                }
                '|' => {
                    if self.peek_is('|') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::OrOr, p));
                    } else {
                        return Err(CustomLangError::lex(
                            line,
                            col,
                            "unexpected '|'; did you mean '||'?",
                        ));
                    }
                }
                '"' | '\'' => {
                    let s = self.read_string(ch, line, col)?;
                    tokens.push(Token::new(TokenKind::Str(s), p));
                }
                c if c.is_ascii_digit() => {
                    let n = self.read_number(c)?;
                    tokens.push(Token::new(TokenKind::Number(n), p));
                }
                c if c.is_alphanumeric() || c == '_' => {
                    let id = self.read_ident(c);
                    let kind = Self::keyword_or_ident(id);
                    tokens.push(Token::new(kind, p));
                }
                c => {
                    return Err(CustomLangError::lex(
                        line,
                        col,
                        format!("unexpected character '{c}'"),
                    ));
                }
            }
        }

        tokens.push(Token::new(
            TokenKind::Eof,
            Position::new(self.line, self.col),
        ));
        Ok(tokens)
    }

    // ── helpers ─────────────────────────────────────────────────────────────

    fn at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn advance(&mut self) -> char {
        let c = self.src[self.pos];
        self.pos += 1;
        if c != '\n' {
            self.col += 1;
        }
        c
    }

    fn peek(&self) -> char {
        if self.at_end() {
            '\0'
        } else {
            self.src[self.pos]
        }
    }

    fn peek2(&self) -> char {
        if self.pos + 1 >= self.src.len() {
            '\0'
        } else {
            self.src[self.pos + 1]
        }
    }

    fn peek_is(&self, c: char) -> bool {
        self.peek() == c
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '/' if self.peek2() == '/' => {
                    // Line comment
                    while !self.at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                '/' if self.peek2() == '*' => {
                    // Block comment
                    self.advance(); // /
                    self.advance(); // *
                    loop {
                        if self.at_end() {
                            break;
                        }
                        if self.peek() == '\n' {
                            self.line += 1;
                            self.col = 1;
                            self.pos += 1;
                        } else if self.peek() == '*' && self.peek2() == '/' {
                            self.advance(); // *
                            self.advance(); // /
                            break;
                        } else {
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn read_string(&mut self, quote: char, start_line: usize, start_col: usize) -> Result<String> {
        let mut s = String::new();
        loop {
            if self.at_end() {
                return Err(CustomLangError::lex(
                    start_line,
                    start_col,
                    "unterminated string literal",
                ));
            }
            let c = self.src[self.pos];
            if c == quote {
                self.advance();
                break;
            }
            if c == '\\' {
                self.advance(); // consume backslash
                if self.at_end() {
                    return Err(CustomLangError::lex(
                        start_line,
                        start_col,
                        "unterminated escape sequence",
                    ));
                }
                let esc = self.advance();
                match esc {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '\\' => s.push('\\'),
                    '\'' => s.push('\''),
                    '"' => s.push('"'),
                    '0' => s.push('\0'),
                    'u' => {
                        // Unicode escape \uXXXX
                        let hex = self.read_hex_digits(4, start_line, start_col)?;
                        let cp = u32::from_str_radix(&hex, 16).expect("valid hex");
                        let ch = char::from_u32(cp).ok_or_else(|| {
                            CustomLangError::lex(
                                start_line,
                                start_col,
                                format!("invalid unicode codepoint U+{hex}"),
                            )
                        })?;
                        s.push(ch);
                    }
                    other => s.push(other), // pass through unknown escapes
                }
            } else {
                if c == '\n' {
                    self.line += 1;
                    self.col = 1;
                    self.pos += 1;
                } else {
                    self.advance();
                }
                s.push(c);
            }
        }
        Ok(s)
    }

    fn read_hex_digits(&mut self, count: usize, line: usize, col: usize) -> Result<String> {
        let mut hex = String::new();
        for _ in 0..count {
            if self.at_end() || !self.peek().is_ascii_hexdigit() {
                return Err(CustomLangError::lex(
                    line,
                    col,
                    "expected hex digit in unicode escape",
                ));
            }
            hex.push(self.advance());
        }
        Ok(hex)
    }

    fn read_number(&mut self, first: char) -> Result<f64> {
        let mut s = String::new();
        s.push(first);
        while !self.at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            // Avoid consuming a second dot
            if self.peek() == '.' && s.contains('.') {
                break;
            }
            s.push(self.advance());
        }
        // Scientific notation: 1e10, 2.5e-3
        if !self.at_end() && (self.peek() == 'e' || self.peek() == 'E') {
            s.push(self.advance());
            if !self.at_end() && (self.peek() == '+' || self.peek() == '-') {
                s.push(self.advance());
            }
            while !self.at_end() && self.peek().is_ascii_digit() {
                s.push(self.advance());
            }
        }
        s.parse::<f64>().map_err(|_| {
            CustomLangError::lex(self.line, self.col, format!("invalid number literal '{s}'"))
        })
    }

    fn read_ident(&mut self, first: char) -> String {
        let mut s = String::new();
        s.push(first);
        while !self.at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            s.push(self.advance());
        }
        s
    }

    fn keyword_or_ident(s: String) -> TokenKind {
        match s.as_str() {
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "print" => TokenKind::Print,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "class" => TokenKind::Class,
            "extends" => TokenKind::Extends,
            "this" => TokenKind::This,
            "new" => TokenKind::New,
            "match" => TokenKind::Match,
            _ => TokenKind::Ident(s),
        }
    }
}

// ─────────────────────────────── TESTS ──────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| !matches!(k, TokenKind::Newline | TokenKind::Eof))
            .collect()
    }

    #[test]
    fn test_numbers() {
        let tokens = lex("42 3.14 1e10");
        assert_eq!(tokens[0], TokenKind::Number(42.0));
        #[allow(clippy::approx_constant)]
        let expected = 3.14_f64;
        assert_eq!(tokens[1], TokenKind::Number(expected));
        assert_eq!(tokens[2], TokenKind::Number(1e10));
    }

    #[test]
    fn test_strings_with_escapes() {
        let tokens = lex(r#""hello\nworld""#);
        assert_eq!(tokens[0], TokenKind::Str("hello\nworld".to_string()));
    }

    #[test]
    fn test_single_quote_strings() {
        let tokens = lex("'hello'");
        assert_eq!(tokens[0], TokenKind::Str("hello".to_string()));
    }

    #[test]
    fn test_keywords() {
        let tokens = lex("let if else while for break continue function return");
        assert_eq!(tokens[0], TokenKind::Let);
        assert_eq!(tokens[1], TokenKind::If);
        assert_eq!(tokens[3], TokenKind::While);
        assert_eq!(tokens[4], TokenKind::For);
        assert_eq!(tokens[5], TokenKind::Break);
        assert_eq!(tokens[6], TokenKind::Continue);
    }

    #[test]
    fn test_compound_operators() {
        let tokens = lex("+= -= *= /= %=");
        assert_eq!(tokens[0], TokenKind::PlusEq);
        assert_eq!(tokens[1], TokenKind::MinusEq);
        assert_eq!(tokens[2], TokenKind::StarEq);
        assert_eq!(tokens[3], TokenKind::SlashEq);
        assert_eq!(tokens[4], TokenKind::PercentEq);
    }

    #[test]
    fn test_comparison_ops() {
        let tokens = lex("== != < <= > >=");
        assert_eq!(tokens[0], TokenKind::EqEq);
        assert_eq!(tokens[1], TokenKind::BangEq);
        assert_eq!(tokens[2], TokenKind::Lt);
        assert_eq!(tokens[3], TokenKind::LtEq);
        assert_eq!(tokens[4], TokenKind::Gt);
        assert_eq!(tokens[5], TokenKind::GtEq);
    }

    #[test]
    fn test_block_comment() {
        let tokens = lex("1 /* this is a comment */ 2");
        assert_eq!(tokens[0], TokenKind::Number(1.0));
        assert_eq!(tokens[1], TokenKind::Number(2.0));
    }

    #[test]
    fn test_line_comment() {
        let tokens = lex("1 // comment\n2");
        assert_eq!(tokens[0], TokenKind::Number(1.0));
        assert_eq!(tokens[1], TokenKind::Number(2.0));
    }

    #[test]
    fn test_position_tracking() {
        let mut lexer = Lexer::new("let x = 42;\nlet y = 3;");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].pos.line, 1);
        let second_let = tokens.iter().find(|t| t.pos.line == 2).unwrap();
        assert_eq!(second_let.pos.line, 2);
    }
}
