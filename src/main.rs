use clap::{Arg, Command};
use std::fs;

mod ast;
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
        .version("0.2.0")
        .author("Custom Language Team <team@customlang.dev>")
        .about("A custom programming language interpreter with comprehensive features")
        .long_about(
            "Custom Language Interpreter v0.1.0\n\
                     \n\
                     A modern programming language interpreter featuring:\n\
                     • Dynamic typing with numbers, strings, booleans, and null\n\
                     • Variables and expressions\n\
                     • Control flow (if/else, while loops)\n\
                     • User-defined and recursive functions\n\
                     • Built-in functions (math, string, utility)\n\
                     • Comprehensive error reporting\n\
                     • Interactive REPL mode\n\
                     • Semantic analysis and type checking",
        )
        .arg(
            Arg::new("file")
                .help("The source file to execute (.cl extension)")
                .value_name("FILE")
                .index(1),
        )
        .arg(
            Arg::new("repl")
                .short('r')
                .long("repl")
                .help("Start interactive REPL mode")
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
                .help("Skip semantic analysis (faster but less safe)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let verbose = matches.get_flag("verbose");
    let no_semantic = matches.get_flag("no-semantic");

    if matches.get_flag("repl") {
        if verbose {
            println!("🚀 Starting Custom Language REPL...");
            println!("📝 Type 'help' for available commands");
            println!(
                "🔧 Semantic analysis: {}",
                if no_semantic { "disabled" } else { "enabled" }
            );
        } else {
            println!("Starting Custom Language REPL...");
        }
        let mut repl = Repl::new();
        repl.run()?;
    } else if let Some(filename) = matches.get_one::<String>("file") {
        if verbose {
            println!("📂 Executing file: {filename}");
            println!(
                "🔧 Semantic analysis: {}",
                if no_semantic { "disabled" } else { "enabled" }
            );
        }
        execute_file(filename, verbose, no_semantic)?;
    } else {
        print_welcome_message();
    }

    Ok(())
}

fn execute_file(filename: &str, verbose: bool, no_semantic: bool) -> Result<(), CustomLangError> {
    let source = fs::read_to_string(filename)
        .map_err(|e| CustomLangError::IoError(format!("Failed to read file '{filename}': {e}")))?;

    if verbose {
        println!("📊 File size: {} bytes", source.len());
        println!("📝 Lines of code: {}", source.lines().count());
    }

    if let Err(e) = execute_source(&source, verbose, no_semantic) {
        eprintln!("{}", e.display_detailed(Some(&source)));
        std::process::exit(1);
    }

    if verbose {
        println!("✅ Execution completed successfully!");
    }

    Ok(())
}

fn execute_source(source: &str, verbose: bool, no_semantic: bool) -> Result<(), CustomLangError> {
    // Tokenize
    if verbose {
        println!("🔤 Tokenizing...");
    }
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize()?;
    if verbose {
        println!("✅ Tokenization complete: {} tokens", tokens.len());
    }

    // Parse
    if verbose {
        println!("🌳 Parsing...");
    }
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse()?;
    if verbose {
        println!(
            "✅ Parsing complete: {} statements",
            program.statements.len()
        );
    }

    // Semantic Analysis
    if !no_semantic {
        if verbose {
            println!("🔍 Semantic analysis...");
        }
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&program)?;
        if verbose {
            println!("✅ Semantic analysis complete");
        }
    } else if verbose {
        println!("⚠️  Semantic analysis skipped");
    }

    // Execute
    if verbose {
        println!("⚡ Executing...");
    }
    let mut interpreter = interpreter::Interpreter::new();
    interpreter.interpret(&program)?;
    if verbose {
        println!("✅ Execution complete");
    }

    Ok(())
}

fn print_welcome_message() {
    println!("🎉 Custom Language Interpreter v0.2.0");
    println!();
    println!("📚 A modern programming language with comprehensive features:");
    println!("   • Dynamic typing (numbers, strings, booleans, null, arrays)");
    println!("   • Variables and expressions");
    println!("   • Control flow (if/else, while loops)");
    println!("   • User-defined and recursive functions");
    println!("   • Built-in functions (math, string, utility, arrays)");
    println!("   • Arrays with indexing and manipulation");
    println!("   • Comprehensive error reporting");
    println!("   • Interactive REPL mode");
    println!("   • Semantic analysis and type checking");
    println!();
    println!("🚀 Usage:");
    println!("   custom-lang <file.cl>     Execute a source file");
    println!("   custom-lang --repl       Start interactive mode");
    println!("   custom-lang --help       Show detailed help");
    println!();
    println!("📖 Examples:");
    println!("   custom-lang examples/test.cl");
    println!("   custom-lang --repl --verbose");
    println!("   custom-lang program.cl --no-semantic");
    println!();
    println!("💡 Try the REPL for interactive experimentation!");
}
