use crate::error::Result;
use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

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
        println!("{}", "Custom Language REPL v0.3.0".bright_cyan().bold());
        println!(
            "{}",
            "Type 'exit' or 'quit' to exit, 'help' for commands.".dimmed()
        );
        println!();

        loop {
            match self.editor.readline(">> ") {
                Ok(line) => {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    let _ = self.editor.add_history_entry(&line);

                    match line.as_str() {
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

                    if let Err(e) = self.execute_line(&line) {
                        eprintln!("{}", e.display_with_source(Some(&line)));
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("{}", "Use 'exit' or 'quit' to exit.".yellow());
                }
                Err(ReadlineError::Eof) => {
                    println!("{}", "Goodbye!".bright_green());
                    break;
                }
                Err(err) => {
                    eprintln!("{}: {}", "Error".bright_red(), err);
                    break;
                }
            }
        }
        Ok(())
    }

    fn execute_line(&mut self, source: &str) -> Result<()> {
        let tokens = Lexer::new(source).tokenize()?;
        let program = Parser::new(tokens).parse()?;
        if let Some(val) = self.interpreter.exec_repl(&program)? {
            println!("{}", format!("=> {val}").bright_white());
        }
        Ok(())
    }

    fn print_help(&self) {
        println!("{}", "Custom Language Help".bright_cyan().bold());
        println!();
        println!("{}", "Syntax:".bright_yellow());
        println!("  let x = 42;                    -- variable");
        println!("  x += 1;                        -- compound assign");
        println!("  if (x > 0) {{ ... }} else {{ ... }} -- conditional");
        println!("  while (x > 0) {{ ... }}          -- while loop");
        println!("  for (let i=0; i<n; i+=1) {{ }}  -- for loop");
        println!("  for (item in arr) {{ }}          -- for-in loop");
        println!("  function f(a, b) {{ return a+b; }} -- function");
        println!("  let f = function(x) {{ x*2 }};  -- lambda");
        println!("  class Dog extends Animal {{ ... }} -- class");
        println!("  match x {{ 1 => ..., _ => ... }} -- pattern match");
        println!();
        println!("{}", "Built-ins:".bright_yellow());
        println!("  print, len, range, type, push, pop, filter, map, reduce");
        println!("  floor, ceil, round, sqrt, pow, min, max, abs, log");
        println!("  split, join, to_upper, to_lower, trim, contains");
        println!("  read_file, write_file, append_file, input, now, assert");
        println!();
        println!("{}", "REPL commands:".bright_yellow());
        println!("  help   clear   exit   quit");
        println!();
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}
