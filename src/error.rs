//! # Error Handling System
//!
//! Comprehensive error handling for the Custom Language Interpreter with:
//! - Detailed error messages with source context
//! - Color-coded output for better readability
//! - Source location tracking for precise error reporting
//! - Multiple error categories for different compilation phases
//!
//! ## Error Categories
//!
//! - **LexError**: Tokenization errors (invalid characters, malformed tokens)
//! - **ParseError**: Syntax errors (invalid grammar, unexpected tokens)
//! - **SemanticError**: Type checking and semantic analysis errors
//! - **RuntimeError**: Execution-time errors (undefined variables, type mismatches)
//! - **TypeError**: Type-related errors during runtime
//! - **IoError**: File system and I/O related errors

use colored::*;
use thiserror::Error;

/// Custom error types for the programming language interpreter
#[derive(Error, Debug, Clone)]
pub enum CustomLangError {
    #[error("Lexical error at line {line}, column {column}: {message}")]
    LexError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Semantic error: {message}")]
    SemanticError { message: String },

    #[error("Runtime error: {message}")]
    RuntimeError { message: String },

    #[error("Type error: {message}")]
    TypeError { message: String },

    #[error("Undefined variable: {name}")]
    UndefinedVariable {
        name: String,
        suggestion: Option<String>,
    },

    #[error("Undefined function: {name}")]
    UndefinedFunction {
        name: String,
        suggestion: Option<String>,
    },

    #[error("Division by zero")]
    DivisionByZero,

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Internal error: {0}")]
    #[allow(dead_code)]
    InternalError(String),
}

impl CustomLangError {
    pub fn lex_error(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::LexError {
            line,
            column,
            message: message.into(),
        }
    }

    pub fn parse_error(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::ParseError {
            line,
            column,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    pub fn semantic_error(message: impl Into<String>) -> Self {
        Self::SemanticError {
            message: message.into(),
        }
    }

    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self::RuntimeError {
            message: message.into(),
        }
    }

    pub fn type_error(message: impl Into<String>) -> Self {
        Self::TypeError {
            message: message.into(),
        }
    }

    pub fn undefined_variable(name: impl Into<String>) -> Self {
        Self::UndefinedVariable {
            name: name.into(),
            suggestion: None,
        }
    }

    pub fn undefined_variable_with_suggestion(name: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::UndefinedVariable {
            name: name.into(),
            suggestion: Some(suggestion.into()),
        }
    }

    #[allow(dead_code)]
    pub fn undefined_function(name: impl Into<String>) -> Self {
        Self::UndefinedFunction {
            name: name.into(),
            suggestion: None,
        }
    }

    pub fn undefined_function_with_suggestion(name: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::UndefinedFunction {
            name: name.into(),
            suggestion: Some(suggestion.into()),
        }
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, CustomLangError>;

impl CustomLangError {
    /// Display a detailed error message with context
    pub fn display_detailed(&self, source_code: Option<&str>) -> String {
        let error_msg = format!("{}: {}", "Error".bright_red().bold(), self);

        let suggestion = self.get_suggestion();
        let suggestion_text = if let Some(suggestion) = suggestion {
            format!("\n{}: {}", "Suggestion".bright_yellow().bold(), suggestion.bright_white())
        } else {
            String::new()
        };

        match self {
            CustomLangError::LexError { line, column, .. }
            | CustomLangError::ParseError { line, column, .. } => {
                if let Some(source) = source_code {
                    format!(
                        "{}\n{}{}",
                        error_msg,
                        self.format_source_context(source, *line, *column),
                        suggestion_text
                    )
                } else {
                    format!("{}{}", error_msg, suggestion_text)
                }
            }
            _ => format!("{}{}", error_msg, suggestion_text),
        }
    }

    fn get_suggestion(&self) -> Option<&str> {
        match self {
            CustomLangError::UndefinedVariable { suggestion, .. } => suggestion.as_deref(),
            CustomLangError::UndefinedFunction { suggestion, .. } => suggestion.as_deref(),
            _ => None,
        }
    }

    fn format_source_context(
        &self,
        source: &str,
        error_line: usize,
        error_column: usize,
    ) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let mut result = String::new();

        // Show context around the error (2 lines before and after)
        let start_line = error_line.saturating_sub(3);
        let end_line = (error_line + 2).min(lines.len());

        result.push_str(&format!("\n{}\n", "Source context:".bright_blue().bold()));

        for (i, line) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line)
        {
            let line_num = i + 1;
            let line_num_str = format!("{line_num:4}");

            if line_num == error_line {
                // Highlight the error line
                result.push_str(&format!(
                    "{} {} {}\n",
                    line_num_str.bright_red().bold(),
                    "|".bright_red().bold(),
                    line.bright_white()
                ));

                // Add pointer to the error column
                let spaces = " ".repeat(6 + error_column.saturating_sub(1));
                result.push_str(&format!("{}{}\n", spaces, "^".bright_red().bold()));
            } else {
                // Normal context line
                result.push_str(&format!(
                    "{} {} {}\n",
                    line_num_str.dimmed(),
                    "|".dimmed(),
                    line.dimmed()
                ));
            }
        }

        result
    }
}

/// Helper functions for generating suggestions
impl CustomLangError {
    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();

        if len1 == 0 { return len2; }
        if len2 == 0 { return len1; }

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 { matrix[i][0] = i; }
        for j in 0..=len2 { matrix[0][j] = j; }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1.chars().nth(i - 1) == s2.chars().nth(j - 1) { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }

    /// Find the most similar name from a list of candidates
    pub fn find_similar_name(target: &str, candidates: &[String]) -> Option<String> {
        if candidates.is_empty() { return None; }

        let mut best_match = None;
        let mut best_distance = usize::MAX;

        for candidate in candidates {
            let distance = Self::levenshtein_distance(target, candidate);
            // Only suggest if the distance is reasonable (less than half the target length)
            if distance < target.len() / 2 + 2 && distance < best_distance {
                best_distance = distance;
                best_match = Some(candidate.clone());
            }
        }

        best_match
    }

    /// Generate helpful suggestions for common error patterns
    pub fn generate_suggestion(error_type: &str, context: &str) -> Option<String> {
        match error_type {
            "undefined_variable" => {
                Some(format!("Did you mean to declare the variable first? Use: let {} = value;", context))
            }
            "undefined_function" => {
                Some("Check if the function name is spelled correctly or if it's a built-in function.".to_string())
            }
            "type_mismatch" => {
                Some("Check the types of values you're using. Use type() function to inspect values.".to_string())
            }
            "syntax_error" => {
                Some("Check for missing semicolons, parentheses, or brackets.".to_string())
            }
            _ => None,
        }
    }
}
