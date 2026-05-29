use crate::ast::Position;
use crate::error::{CustomLangError, Result};

/// All token kinds produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    Str(String),
    TemplateLiteral(String), // raw backtick string content
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
    Do,
    Throw,
    Try,
    Catch,
    Finally,
    Super,
    Static,
    Instanceof,

    // Arithmetic operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,   // **

    // Bitwise operators
    Amp,        // &
    Pipe,       // |
    Caret,      // ^
    Tilde,      // ~
    LtLt,       // <<
    GtGt,       // >>
    GtGtGt,     // >>>

    // Logical operators
    AndAnd,     // &&
    OrOr,       // ||

    // Null/optional operators
    Question,           // ?
    QuestionQuestion,   // ??
    QuestionDot,        // ?.

    // Comparison
    Dot,
    Eq,         // =
    EqEq,       // ==
    Bang,       // !
    BangEq,     // !=
    Lt,         // <
    LtEq,       // <=
    Gt,         // >
    GtEq,       // >=
    Arrow,      // =>

    // Compound assignment
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    PercentEq,  // %=
    StarStarEq, // **=
    AmpEq,      // &=
    PipeEq,     // |=
    CaretEq,    // ^=
    LtLtEq,     // <<=
    GtGtEq,     // >>=
    AndAndEq,   // &&=
    OrOrEq,     // ||=
    QuestionQuestionEq, // ??=

    // Spread/rest
    DotDotDot,  // ...

    // Pipe operator
    PipeArrow,  // |>

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
                    continue;
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
                    if self.peek_is('*') {
                        self.advance();
                        if self.peek_is('=') {
                            self.advance();
                            tokens.push(Token::new(TokenKind::StarStarEq, p));
                        } else {
                            tokens.push(Token::new(TokenKind::StarStar, p));
                        }
                    } else if self.peek_is('=') {
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
                '.' => {
                    if self.peek_is('.') && self.peek2() == '.' {
                        self.advance();
                        self.advance();
                        tokens.push(Token::new(TokenKind::DotDotDot, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Dot, p));
                    }
                }
                '&' => {
                    if self.peek_is('&') {
                        self.advance();
                        if self.peek_is('=') {
                            self.advance();
                            tokens.push(Token::new(TokenKind::AndAndEq, p));
                        } else {
                            tokens.push(Token::new(TokenKind::AndAnd, p));
                        }
                    } else if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::AmpEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Amp, p));
                    }
                }
                '|' => {
                    if self.peek_is('|') {
                        self.advance();
                        if self.peek_is('=') {
                            self.advance();
                            tokens.push(Token::new(TokenKind::OrOrEq, p));
                        } else {
                            tokens.push(Token::new(TokenKind::OrOr, p));
                        }
                    } else if self.peek_is('>') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::PipeArrow, p));
                    } else if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::PipeEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Pipe, p));
                    }
                }
                '^' => {
                    if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::CaretEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Caret, p));
                    }
                }
                '~' => tokens.push(Token::new(TokenKind::Tilde, p)),
                '(' => tokens.push(Token::new(TokenKind::LParen, p)),
                ')' => tokens.push(Token::new(TokenKind::RParen, p)),
                '{' => tokens.push(Token::new(TokenKind::LBrace, p)),
                '}' => tokens.push(Token::new(TokenKind::RBrace, p)),
                '[' => tokens.push(Token::new(TokenKind::LBracket, p)),
                ']' => tokens.push(Token::new(TokenKind::RBracket, p)),
                ',' => tokens.push(Token::new(TokenKind::Comma, p)),
                ';' => tokens.push(Token::new(TokenKind::Semicolon, p)),
                ':' => tokens.push(Token::new(TokenKind::Colon, p)),
                '?' => {
                    if self.peek_is('?') {
                        self.advance();
                        if self.peek_is('=') {
                            self.advance();
                            tokens.push(Token::new(TokenKind::QuestionQuestionEq, p));
                        } else {
                            tokens.push(Token::new(TokenKind::QuestionQuestion, p));
                        }
                    } else if self.peek_is('.') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::QuestionDot, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Question, p));
                    }
                }
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
                    if self.peek_is('<') {
                        self.advance();
                        if self.peek_is('=') {
                            self.advance();
                            tokens.push(Token::new(TokenKind::LtLtEq, p));
                        } else {
                            tokens.push(Token::new(TokenKind::LtLt, p));
                        }
                    } else if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::LtEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Lt, p));
                    }
                }
                '>' => {
                    if self.peek_is('>') {
                        self.advance();
                        if self.peek_is('>') {
                            self.advance();
                            if self.peek_is('=') {
                                self.advance();
                                // >>>= not currently in token enum, treat as >>> then =
                                tokens.push(Token::new(TokenKind::GtGtGt, p.clone()));
                                tokens.push(Token::new(TokenKind::Eq, p));
                            } else {
                                tokens.push(Token::new(TokenKind::GtGtGt, p));
                            }
                        } else if self.peek_is('=') {
                            self.advance();
                            tokens.push(Token::new(TokenKind::GtGtEq, p));
                        } else {
                            tokens.push(Token::new(TokenKind::GtGt, p));
                        }
                    } else if self.peek_is('=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::GtEq, p));
                    } else {
                        tokens.push(Token::new(TokenKind::Gt, p));
                    }
                }
                '"' => {
                    // Check for heredoc """
                    if self.peek_is('"') && self.peek2() == '"' {
                        self.advance(); // second "
                        self.advance(); // third "
                        let s = self.read_heredoc(line, col)?;
                        tokens.push(Token::new(TokenKind::Str(s), p));
                    } else {
                        let s = self.read_string('"', line, col)?;
                        tokens.push(Token::new(TokenKind::Str(s), p));
                    }
                }
                '\'' => {
                    let s = self.read_string('\'', line, col)?;
                    tokens.push(Token::new(TokenKind::Str(s), p));
                }
                '`' => {
                    let s = self.read_template_literal(line, col)?;
                    tokens.push(Token::new(TokenKind::TemplateLiteral(s), p));
                }
                c if c.is_ascii_digit() => {
                    let n = self.read_number(c, line, col)?;
                    tokens.push(Token::new(TokenKind::Number(n), p));
                }
                c if c.is_alphanumeric() || c == '_' => {
                    let id = self.read_ident(c);
                    // Check for raw string: r"..." or r'...'
                    if id == "r" && (self.peek_is('"') || self.peek_is('\'')) {
                        let quote = self.advance();
                        let s = self.read_raw_string(quote, line, col)?;
                        tokens.push(Token::new(TokenKind::Str(s), p));
                    } else {
                        let kind = Self::keyword_or_ident(id);
                        tokens.push(Token::new(kind, p));
                    }
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
                    while !self.at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                '/' if self.peek2() == '*' => {
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
                self.advance();
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
                    other => s.push(other),
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

    /// Read raw string — no escape processing
    fn read_raw_string(&mut self, quote: char, start_line: usize, start_col: usize) -> Result<String> {
        let mut s = String::new();
        loop {
            if self.at_end() {
                return Err(CustomLangError::lex(start_line, start_col, "unterminated raw string"));
            }
            let c = self.src[self.pos];
            if c == quote {
                self.advance();
                break;
            }
            if c == '\n' {
                self.line += 1;
                self.col = 1;
                self.pos += 1;
            } else {
                self.advance();
            }
            s.push(c);
        }
        Ok(s)
    }

    /// Read heredoc: content between """ and """
    fn read_heredoc(&mut self, start_line: usize, start_col: usize) -> Result<String> {
        let mut s = String::new();
        loop {
            if self.at_end() {
                return Err(CustomLangError::lex(start_line, start_col, "unterminated heredoc"));
            }
            // Check for closing """
            if self.peek() == '"' && self.peek2() == '"' && self.pos + 2 < self.src.len() && self.src[self.pos + 2] == '"' {
                self.advance();
                self.advance();
                self.advance();
                break;
            }
            let c = self.src[self.pos];
            if c == '\n' {
                self.line += 1;
                self.col = 1;
                self.pos += 1;
            } else {
                self.advance();
            }
            s.push(c);
        }
        // Trim leading/trailing newline from heredoc
        let s = s.trim_matches('\n').to_string();
        Ok(s)
    }

    /// Read template literal — returns raw content including ${...} markers
    fn read_template_literal(&mut self, start_line: usize, start_col: usize) -> Result<String> {
        let mut s = String::new();
        loop {
            if self.at_end() {
                return Err(CustomLangError::lex(start_line, start_col, "unterminated template literal"));
            }
            let c = self.src[self.pos];
            if c == '`' {
                self.advance();
                break;
            }
            if c == '\n' {
                self.line += 1;
                self.col = 1;
                self.pos += 1;
                s.push(c);
                continue;
            }
            // Pass through ${ and } as-is — parser handles interpolation
            if c == '\\' {
                self.advance();
                if !self.at_end() {
                    let esc = self.src[self.pos];
                    self.advance();
                    match esc {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '\\' => s.push('\\'),
                        '`' => s.push('`'),
                        '$' => s.push('$'),
                        c => { s.push('\\'); s.push(c); }
                    }
                }
                continue;
            }
            self.advance();
            s.push(c);
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

    fn read_number(&mut self, first: char, line: usize, col: usize) -> Result<f64> {
        // Handle 0x, 0o, 0b prefixes
        if first == '0' && !self.at_end() {
            match self.peek() {
                'x' | 'X' => {
                    self.advance(); // consume x
                    let mut hex = String::new();
                    while !self.at_end() && self.peek().is_ascii_hexdigit() {
                        hex.push(self.advance());
                    }
                    return i64::from_str_radix(&hex, 16)
                        .map(|n| n as f64)
                        .map_err(|_| CustomLangError::lex(line, col, format!("invalid hex literal '0x{hex}'")));
                }
                'o' | 'O' => {
                    self.advance();
                    let mut oct = String::new();
                    while !self.at_end() && matches!(self.peek(), '0'..='7') {
                        oct.push(self.advance());
                    }
                    return i64::from_str_radix(&oct, 8)
                        .map(|n| n as f64)
                        .map_err(|_| CustomLangError::lex(line, col, format!("invalid octal literal '0o{oct}'")));
                }
                'b' | 'B' => {
                    self.advance();
                    let mut bin = String::new();
                    while !self.at_end() && matches!(self.peek(), '0' | '1') {
                        bin.push(self.advance());
                    }
                    return i64::from_str_radix(&bin, 2)
                        .map(|n| n as f64)
                        .map_err(|_| CustomLangError::lex(line, col, format!("invalid binary literal '0b{bin}'")));
                }
                _ => {}
            }
        }

        let mut s = String::new();
        s.push(first);
        while !self.at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            if self.peek() == '.' && s.contains('.') {
                break;
            }
            s.push(self.advance());
        }
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
            CustomLangError::lex(line, col, format!("invalid number literal '{s}'"))
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
            "do" => TokenKind::Do,
            "throw" => TokenKind::Throw,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "super" => TokenKind::Super,
            "static" => TokenKind::Static,
            "instanceof" => TokenKind::Instanceof,
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
    fn test_hex_oct_bin() {
        let tokens = lex("0xFF 0o17 0b1010");
        assert_eq!(tokens[0], TokenKind::Number(255.0));
        assert_eq!(tokens[1], TokenKind::Number(15.0));
        assert_eq!(tokens[2], TokenKind::Number(10.0));
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
    fn test_new_keywords() {
        let tokens = lex("do try catch finally throw super static instanceof");
        assert_eq!(tokens[0], TokenKind::Do);
        assert_eq!(tokens[1], TokenKind::Try);
        assert_eq!(tokens[2], TokenKind::Catch);
        assert_eq!(tokens[3], TokenKind::Finally);
        assert_eq!(tokens[4], TokenKind::Throw);
        assert_eq!(tokens[5], TokenKind::Super);
        assert_eq!(tokens[6], TokenKind::Static);
        assert_eq!(tokens[7], TokenKind::Instanceof);
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
    fn test_new_operators() {
        let tokens = lex("** ?? ?. ... |>");
        assert_eq!(tokens[0], TokenKind::StarStar);
        assert_eq!(tokens[1], TokenKind::QuestionQuestion);
        assert_eq!(tokens[2], TokenKind::QuestionDot);
        assert_eq!(tokens[3], TokenKind::DotDotDot);
        assert_eq!(tokens[4], TokenKind::PipeArrow);
    }

    #[test]
    fn test_bitwise_operators() {
        let tokens = lex("& | ^ ~ << >>");
        assert_eq!(tokens[0], TokenKind::Amp);
        assert_eq!(tokens[1], TokenKind::Pipe);
        assert_eq!(tokens[2], TokenKind::Caret);
        assert_eq!(tokens[3], TokenKind::Tilde);
        assert_eq!(tokens[4], TokenKind::LtLt);
        assert_eq!(tokens[5], TokenKind::GtGt);
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

    #[test]
    fn test_template_literal() {
        let tokens = lex("`hello ${name}`");
        match &tokens[0] {
            TokenKind::TemplateLiteral(s) => assert!(s.contains("${name}")),
            _ => panic!("expected template literal"),
        }
    }
}
