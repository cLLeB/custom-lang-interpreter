use clap::{Arg, Command};
use std::fs;

mod ast;
mod env;
mod error;
mod interpreter;
mod lexer;
mod parser;
mod repl;
mod semantic;

use error::CustomLangError;
use repl::Repl;
use semantic::SemanticAnalyzer;

fn main() -> Result<(), CustomLangError> {
    let matches = Command::new("custom-lang")
        .version("1.0.0")
        .about("A custom programming language interpreter")
        .arg(
            Arg::new("file")
                .help("Source file to execute (.cl)")
                .value_name("FILE")
                .index(1),
        )
        .arg(
            Arg::new("repl")
                .short('r')
                .long("repl")
                .help("Start interactive REPL")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-semantic")
                .long("no-semantic")
                .help("Skip semantic analysis")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let verbose = matches.get_flag("verbose");
    let no_semantic = matches.get_flag("no-semantic");

    if matches.get_flag("repl") {
        let mut repl = Repl::new();
        repl.run()?;
    } else if let Some(filename) = matches.get_one::<String>("file") {
        execute_file(filename, verbose, no_semantic)?;
    } else {
        print_welcome();
    }

    Ok(())
}

fn execute_file(filename: &str, verbose: bool, no_semantic: bool) -> Result<(), CustomLangError> {
    let source = fs::read_to_string(filename)
        .map_err(|e| CustomLangError::io_err(format!("Cannot read '{filename}': {e}")))?;

    if verbose {
        println!(
            "File: {filename} ({} bytes, {} lines)",
            source.len(),
            source.lines().count()
        );
    }

    if let Err(e) = execute_source(&source, verbose, no_semantic) {
        eprintln!("{}", e.display_with_source(Some(&source)));
        std::process::exit(1);
    }
    Ok(())
}

fn execute_source(source: &str, verbose: bool, no_semantic: bool) -> Result<(), CustomLangError> {
    if verbose {
        println!("Tokenizing...");
    }
    let tokens = lexer::Lexer::new(source).tokenize()?;
    if verbose {
        println!("{} tokens", tokens.len());
    }

    if verbose {
        println!("Parsing...");
    }
    let program = parser::Parser::new(tokens).parse()?;
    if verbose {
        println!("{} statements", program.stmts.len());
    }

    if !no_semantic {
        if verbose {
            println!("Semantic analysis...");
        }
        SemanticAnalyzer::new().analyze(&program)?;
    }

    if verbose {
        println!("Executing...");
    }
    interpreter::Interpreter::new().interpret(&program)?;
    Ok(())
}

fn print_welcome() {
    println!("Custom Language Interpreter v1.0.0");
    println!();
    println!("Usage:");
    println!("  custom-lang <file.cl>    Execute a source file");
    println!("  custom-lang --repl       Start interactive REPL");
    println!("  custom-lang --help       Show help");
    println!();
    println!("Features:");
    println!("  Numbers, strings, booleans, null, arrays, objects");
    println!("  Variables, functions, lambdas, closures, classes");
    println!("  for/while/for-in loops, break/continue");
    println!("  Pattern matching, imports, modules");
    println!("  50+ built-in functions");
}
