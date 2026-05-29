use colored::*;
use thiserror::Error;

/// All error categories in the interpreter pipeline
#[derive(Error, Debug, Clone)]
pub enum CustomLangError {
    #[error("Lex error at {line}:{col}: {msg}")]
    LexError {
        line: usize,
        col: usize,
        msg: String,
    },

    #[error("Parse error at {line}:{col}: {msg}")]
    ParseError {
        line: usize,
        col: usize,
        msg: String,
    },

    #[error("Semantic error: {msg}")]
    SemanticError { msg: String },

    #[error("Runtime error: {msg}")]
    RuntimeError { msg: String },

    #[error("Type error: {msg}")]
    TypeError { msg: String },

    #[error("Undefined variable '{name}'{}", format_suggestion(.suggestion))]
    UndefinedVariable {
        name: String,
        suggestion: Option<String>,
    },

    #[allow(dead_code)]
    #[error("Undefined function '{name}'{}", format_suggestion(.suggestion))]
    UndefinedFunction {
        name: String,
        suggestion: Option<String>,
    },

    #[error("Division by zero")]
    DivisionByZero,

    #[error("I/O error: {msg}")]
    IoError { msg: String },

    #[error("Stack overflow: maximum call depth exceeded")]
    StackOverflow,

    #[error("unhandled exception")]
    ThrownException,
}

fn format_suggestion(s: &Option<String>) -> String {
    match s {
        Some(hint) => format!(" — {hint}"),
        None => String::new(),
    }
}

pub type Result<T> = std::result::Result<T, CustomLangError>;

impl CustomLangError {
    // ── constructors ────────────────────────────────────────────────────────

    pub fn lex(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Self::LexError {
            line,
            col,
            msg: msg.into(),
        }
    }

    pub fn parse(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Self::ParseError {
            line,
            col,
            msg: msg.into(),
        }
    }

    pub fn semantic(msg: impl Into<String>) -> Self {
        Self::SemanticError { msg: msg.into() }
    }

    pub fn runtime(msg: impl Into<String>) -> Self {
        Self::RuntimeError { msg: msg.into() }
    }

    pub fn type_err(msg: impl Into<String>) -> Self {
        Self::TypeError { msg: msg.into() }
    }

    pub fn undef_var(name: impl Into<String>, suggestion: Option<String>) -> Self {
        Self::UndefinedVariable {
            name: name.into(),
            suggestion,
        }
    }

    #[allow(dead_code)]
    pub fn undef_fn(name: impl Into<String>, suggestion: Option<String>) -> Self {
        Self::UndefinedFunction {
            name: name.into(),
            suggestion,
        }
    }

    pub fn io_err(msg: impl Into<String>) -> Self {
        Self::IoError { msg: msg.into() }
    }

    // ── display ─────────────────────────────────────────────────────────────

    /// Format with source-context snippet when available
    pub fn display_with_source(&self, source: Option<&str>) -> String {
        let header = format!("{}: {}", "Error".bright_red().bold(), self);

        let suggestion_text = self.suggestion_text();

        match self {
            Self::LexError { line, col, .. } | Self::ParseError { line, col, .. } => {
                if let Some(src) = source {
                    format!(
                        "{}\n{}{}",
                        header,
                        self.source_snippet(src, *line, *col),
                        suggestion_text
                    )
                } else {
                    format!("{header}{suggestion_text}")
                }
            }
            _ => format!("{header}{suggestion_text}"),
        }
    }

    fn suggestion_text(&self) -> String {
        let hint = match self {
            Self::UndefinedVariable { suggestion, .. }
            | Self::UndefinedFunction { suggestion, .. } => suggestion.as_deref(),
            _ => None,
        };
        match hint {
            Some(h) => format!("\n{}: {}", "Hint".bright_yellow().bold(), h.bright_white()),
            None => String::new(),
        }
    }

    fn source_snippet(&self, src: &str, err_line: usize, err_col: usize) -> String {
        let lines: Vec<&str> = src.lines().collect();
        let mut out = String::new();
        out.push_str(&format!("\n{}\n", "Source context:".bright_blue().bold()));

        let start = err_line.saturating_sub(2);
        let end = (err_line + 1).min(lines.len());

        for (i, line) in lines[start..end].iter().enumerate() {
            let ln = start + i + 1;
            if ln == err_line {
                out.push_str(&format!(
                    "{} {} {}\n",
                    format!("{ln:4}").bright_red().bold(),
                    "|".bright_red().bold(),
                    line.bright_white()
                ));
                let spaces = " ".repeat(6 + err_col.saturating_sub(1));
                out.push_str(&format!("{}{}\n", spaces, "^".bright_red().bold()));
            } else {
                out.push_str(&format!(
                    "{} {} {}\n",
                    format!("{ln:4}").dimmed(),
                    "|".dimmed(),
                    line.dimmed()
                ));
            }
        }
        out
    }

    // ── fuzzy name matching ──────────────────────────────────────────────────

    pub fn find_similar(target: &str, candidates: &[String]) -> Option<String> {
        candidates
            .iter()
            .filter_map(|c| {
                let d = levenshtein(target, c);
                if d <= target.len() / 2 + 1 {
                    Some((d, c))
                } else {
                    None
                }
            })
            .min_by_key(|(d, _)| *d)
            .map(|(_, c)| c.clone())
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}
