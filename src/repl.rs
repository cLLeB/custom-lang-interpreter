use crate::error::{CustomLangError, Result};
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

/// Read-Eval-Print Loop for interactive usage
pub struct Repl {
    interpreter: Interpreter,
    editor: DefaultEditor,
}

impl Repl {
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(),
            editor: DefaultEditor::new().expect("Failed to create readline editor"),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        println!("{}", "Custom Language REPL v0.1.0".bright_cyan());
        println!(
            "{}",
            "Type 'exit' or 'quit' to exit, 'help' for help".dimmed()
        );
        println!();

        loop {
            match self.editor.readline(">> ") {
                Ok(line) => {
                    let line = line.trim();

                    if line.is_empty() {
                        continue;
                    }

                    // Add to history
                    let _ = self.editor.add_history_entry(line);

                    // Handle special commands
                    match line {
                        "exit" | "quit" => {
                            println!("{}", "Goodbye!".bright_green());
                            break;
                        }
                        "help" => {
                            self.print_help();
                            continue;
                        }
                        "clear" => {
                            print!("\x1B[2J\x1B[1;1H");
                            continue;
                        }
                        _ => {}
                    }

                    // Execute the line
                    if let Err(e) = self.execute_line(line) {
                        println!("{}", e.display_detailed(Some(line)));
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("{}", "Use 'exit' or 'quit' to exit".yellow());
                }
                Err(ReadlineError::Eof) => {
                    println!("{}", "Goodbye!".bright_green());
                    break;
                }
                Err(err) => {
                    println!("{}: {}", "Error".bright_red(), err);
                    break;
                }
            }
        }

        Ok(())
    }

    fn execute_line(&mut self, line: &str) -> Result<()> {
        // Tokenize
        let mut lexer = Lexer::new(line);
        let tokens = lexer
            .tokenize()
            .map_err(|e| CustomLangError::runtime_error(format!("Lexer error: {e}")))?;

        // Parse
        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| CustomLangError::runtime_error(format!("Parser error: {e}")))?;

        // Execute
        self.interpreter
            .interpret(&program)
            .map_err(|e| CustomLangError::runtime_error(format!("Runtime error: {e}")))?;

        Ok(())
    }

    fn print_help(&self) {
        println!("{}", "Custom Language Help".bright_cyan().bold());
        println!();
        println!("{}", "Language Features:".bright_yellow());
        println!("  • Variables: let x = 42;");
        println!("  • Arithmetic: +, -, *, /, %");
        println!("  • Comparison: ==, !=, <, <=, >, >=");
        println!("  • Logical: &&, ||, !");
        println!("  • Control flow: if/else, while");
        println!("  • Functions: function name(params) {{ ... }}");
        println!("  • Print: print expression;");
        println!();
        println!("{}", "Data Types:".bright_yellow());
        println!("  • Numbers: 42, 3.14");
        println!("  • Strings: \"hello world\"");
        println!("  • Booleans: true, false");
        println!("  • Null: null");
        println!();
        println!("{}", "REPL Commands:".bright_yellow());
        println!("  • help    - Show this help");
        println!("  • clear   - Clear the screen");
        println!("  • exit    - Exit the REPL");
        println!("  • quit    - Exit the REPL");
        println!();
        println!("{}", "Examples:".bright_yellow());
        println!("  let x = 10;");
        println!("  let y = x * 2;");
        println!("  print x + y;");
        println!("  if (x > 5) print \"x is greater than 5\";");
        println!();
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}
