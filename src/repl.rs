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
        self.print_banner();

        loop {
            match self
                .editor
                .readline(&format!("{} ", ">>>".bright_cyan().bold()))
            {
                Ok(line) => {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    let _ = self.editor.add_history_entry(&line);

                    match line.as_str() {
                        "exit" | "quit" => {
                            println!();
                            println!(
                                "  \u{1F44B}  {}",
                                "Thanks for using custom-lang! Goodbye."
                                    .bright_green()
                                    .bold()
                            );
                            println!();
                            break;
                        }
                        "help" => {
                            self.print_help();
                            continue;
                        }
                        "clear" => {
                            print!("\x1B[2J\x1B[1;1H");
                            self.print_banner();
                            continue;
                        }
                        "version" => {
                            println!("  custom-lang v0.3.0");
                            continue;
                        }
                        _ => {}
                    }

                    if let Err(e) = self.execute_line(&line) {
                        eprintln!("{}", e.display_with_source(Some(&line)));
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!(
                        "  {}",
                        "(Ctrl+C pressed — type 'exit' or 'quit' to leave)".yellow()
                    );
                }
                Err(ReadlineError::Eof) => {
                    println!();
                    println!("  {}", "Goodbye!".bright_green().bold());
                    println!();
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
            println!("{} {}", "=>".bright_cyan(), val.to_string().bright_white());
        }
        Ok(())
    }

    fn print_banner(&self) {
        println!();
        println!(
            "  {}",
            "╔══════════════════════════════════════════════════╗"
                .bright_cyan()
                .bold()
        );
        println!(
            "  {}  {}  {}",
            "║".bright_cyan().bold(),
            "       custom-lang  ·  Interactive REPL  v0.3.0       "
                .bright_white()
                .bold(),
            "║".bright_cyan().bold()
        );
        println!(
            "  {}",
            "╚══════════════════════════════════════════════════╝"
                .bright_cyan()
                .bold()
        );
        println!();
        println!(
            "  {}  {}",
            "✦".bright_yellow(),
            "A modern scripting language — type code and hit Enter".dimmed()
        );
        println!(
            "  {}  {}  {}  {}  {}",
            "✦".bright_yellow(),
            "help".bright_cyan(),
            "→ show commands".dimmed(),
            "  exit".bright_cyan(),
            "→ quit".dimmed()
        );
        println!();
        println!("  {} {}", "Try:".dimmed(), "let x = 42".bright_white());
        println!(
            "  {}  {}",
            "    ".dimmed(),
            "print(\"Hello, World!\")".bright_white()
        );
        println!(
            "  {}  {}",
            "    ".dimmed(),
            "function fib(n) { if (n <= 1) { return n } return fib(n-1) + fib(n-2) }"
                .bright_white()
        );
        println!();
    }

    fn print_help(&self) {
        println!();
        println!(
            "  {}",
            "━━━━━━━━━━━━━━━━  custom-lang help  ━━━━━━━━━━━━━━━━"
                .bright_cyan()
                .bold()
        );
        println!();

        let section = |title: &str| {
            println!("  {}", title.bright_yellow().bold());
        };
        let row = |code: &str, desc: &str| {
            println!("    {:<42}  {}", code.bright_white(), desc.dimmed());
        };

        section("Variables & Types");
        row("let x = 42", "number");
        row("let s = \"hello\"", "string");
        row("let b = true", "boolean  (true / false)");
        row("let a = [1, 2, 3]", "array");
        row("let o = { name: \"Ada\", age: 30 }", "object");
        println!();

        section("Control Flow");
        row("if (x > 0) { } else { }", "if / else");
        row("while (x > 0) { x -= 1 }", "while loop");
        row("for (let i = 0; i < n; i += 1) { }", "for loop");
        row("for (item in arr) { }", "for-in (arrays & objects)");
        println!();

        section("Functions");
        row("function add(a, b) { return a + b }", "named function");
        row("let f = function(x) { x * 2 }", "anonymous / lambda");
        row("function* gen() { yield 1; yield 2 }", "generator");
        println!();

        section("Classes");
        row("class Animal { function speak() { } }", "class definition");
        row("class Dog extends Animal { }", "inheritance");
        row("let d = new Dog()", "instantiation");
        println!();

        section("Error Handling");
        row("try { } catch (e) { } finally { }", "try / catch / finally");
        row("throw \"oops\"", "throw");
        println!();

        section("Pattern Matching");
        row(
            "match x { 1 => \"one\", _ => \"other\" }",
            "match expression",
        );
        println!();

        section("Useful Builtins");
        row("print(x)  len(x)  range(n)  get_type(x)", "general purpose");
        row(
            "push(arr, v)  pop(arr)  map(arr, f)  filter(arr, f)",
            "arrays",
        );
        row("split(s, d)  join(arr, d)  trim(s)  to_upper(s)", "strings");
        row("sqrt(x)  floor(x)  ceil(x)  abs(x)  pow(x, n)", "math");
        row(
            "now()  input(prompt)  read_file(p)  write_file(p, s)",
            "I/O",
        );
        println!();

        section("REPL Commands");
        row("help", "show this screen");
        row("clear", "clear the terminal");
        row("version", "show version");
        row("exit  /  quit", "leave the REPL");
        println!();
        println!(
            "  {}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
                .bright_cyan()
                .bold()
        );
        println!();
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}
