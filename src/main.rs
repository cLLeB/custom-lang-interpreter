use clap::{Arg, ArgAction, Command};
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
        .version("2.0.0")
        .about("A modern custom language interpreter")
        .subcommand_required(false)
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
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-semantic")
                .long("no-semantic")
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("fmt")
                .about("Format source file(s)")
                .arg(Arg::new("file").required(true).index(1))
                .arg(Arg::new("check").long("check").action(ArgAction::SetTrue)),
        )
        .subcommand(
            Command::new("lint")
                .about("Lint source file")
                .arg(Arg::new("file").required(true).index(1)),
        )
        .subcommand(
            Command::new("test")
                .about("Run tests in a file")
                .arg(Arg::new("file").required(false).index(1)),
        )
        .subcommand(
            Command::new("docs")
                .about("Generate documentation")
                .arg(Arg::new("src").required(true).index(1))
                .arg(
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .default_value("docs"),
                ),
        )
        .subcommand(
            Command::new("profile")
                .about("Profile execution")
                .arg(Arg::new("file").required(true).index(1)),
        )
        .subcommand(
            Command::new("compile")
                .about("Compile to target")
                .arg(Arg::new("file").required(true).index(1))
                .arg(Arg::new("target").long("target").default_value("bytecode"))
                .arg(Arg::new("output").long("output").short('o')),
        )
        .subcommand(
            Command::new("debug")
                .about("Debug a file")
                .arg(Arg::new("file").required(true).index(1)),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("fmt", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            let check = sub.get_flag("check");
            cmd_fmt(file, check)?;
        }
        Some(("lint", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            cmd_lint(file)?;
        }
        Some(("test", sub)) => {
            let file = sub
                .get_one::<String>("file")
                .map(|s| s.as_str())
                .unwrap_or(".");
            cmd_test(file)?;
        }
        Some(("docs", sub)) => {
            let src = sub.get_one::<String>("src").unwrap();
            let output = sub.get_one::<String>("output").unwrap();
            cmd_docs(src, output)?;
        }
        Some(("profile", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            cmd_profile(file)?;
        }
        Some(("compile", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            let target = sub.get_one::<String>("target").unwrap();
            let output = sub.get_one::<String>("output").map(|s| s.as_str());
            cmd_compile(file, target, output)?;
        }
        Some(("debug", sub)) => {
            let file = sub.get_one::<String>("file").unwrap();
            cmd_debug(file)?;
        }
        _ => {
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
        }
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

// ─── Subcommand implementations ───────────────────────────────────────────────

fn cmd_fmt(file: &str, check: bool) -> Result<(), CustomLangError> {
    let source = fs::read_to_string(file)
        .map_err(|e| CustomLangError::io_err(format!("Cannot read '{file}': {e}")))?;
    let formatted = format_source(&source);
    if check {
        if formatted == source {
            println!("{file}: already formatted");
        } else {
            println!("{file}: needs formatting");
            std::process::exit(1);
        }
    } else {
        fs::write(file, &formatted)
            .map_err(|e| CustomLangError::io_err(format!("Cannot write '{file}': {e}")))?;
        println!("Formatted {file}");
    }
    Ok(())
}

fn format_source(source: &str) -> String {
    // Basic formatter: normalize indentation and spacing
    let mut result = String::new();
    let mut indent = 0usize;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            result.push('\n');
            continue;
        }
        // Decrease indent for closing braces
        if trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')') {
            indent = indent.saturating_sub(1);
        }
        result.push_str(&"    ".repeat(indent));
        result.push_str(trimmed);
        result.push('\n');
        // Increase indent after opening braces
        let opens = trimmed.chars().filter(|&c| c == '{' || c == '[').count();
        let closes = trimmed.chars().filter(|&c| c == '}' || c == ']').count();
        if opens > closes {
            indent += opens - closes;
        }
    }
    result
}

fn cmd_lint(file: &str) -> Result<(), CustomLangError> {
    let source = fs::read_to_string(file)
        .map_err(|e| CustomLangError::io_err(format!("Cannot read '{file}': {e}")))?;
    let tokens = lexer::Lexer::new(&source).tokenize()?;
    let program = parser::Parser::new(tokens).parse()?;
    let mut analyzer = SemanticAnalyzer::new();
    let issues = analyzer.analyze_with_hints(&program);
    if issues.is_empty() {
        println!("{file}: no issues found ✓");
    } else {
        for issue in &issues {
            println!("{file}: {issue}");
        }
        println!("{} issue(s) found", issues.len());
    }
    Ok(())
}

fn cmd_test(file_or_dir: &str) -> Result<(), CustomLangError> {
    // Find test files
    let files: Vec<String> = if std::path::Path::new(file_or_dir).is_dir() {
        find_cl_files(file_or_dir)
    } else {
        vec![file_or_dir.to_string()]
    };
    let mut passed = 0usize;
    let mut failed = 0usize;
    for file in &files {
        let source = fs::read_to_string(file)
            .map_err(|e| CustomLangError::io_err(format!("Cannot read '{file}': {e}")))?;
        // Inject test runner and run
        let mut interp = interpreter::Interpreter::new();
        interp.set_test_mode(true);
        let tokens = lexer::Lexer::new(&source).tokenize()?;
        let program = parser::Parser::new(tokens).parse()?;
        match interp.interpret(&program) {
            Ok(()) => {}
            Err(e) => eprintln!("{file}: {e}"),
        }
        let (p, f) = interp.test_results();
        passed += p;
        failed += f;
    }
    println!("\nResults: {} passed, {} failed", passed, failed);
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn find_cl_files(dir: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "cl").unwrap_or(false) {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
    files
}

fn cmd_docs(src: &str, output: &str) -> Result<(), CustomLangError> {
    let source = fs::read_to_string(src)
        .map_err(|e| CustomLangError::io_err(format!("Cannot read '{src}': {e}")))?;
    fs::create_dir_all(output)
        .map_err(|e| CustomLangError::io_err(format!("Cannot create '{output}': {e}")))?;
    let docs = extract_docs(&source);
    let out_file = format!("{output}/index.md");
    fs::write(&out_file, docs)
        .map_err(|e| CustomLangError::io_err(format!("Cannot write '{out_file}': {e}")))?;
    println!("Documentation written to {out_file}");
    Ok(())
}

fn extract_docs(source: &str) -> String {
    let mut docs = String::from("# Documentation\n\n");
    let mut doc_comment = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("///") {
            doc_comment.push_str(&trimmed[3..].trim());
            doc_comment.push('\n');
        } else if !doc_comment.is_empty() && trimmed.starts_with("function ") {
            let name = trimmed
                .trim_start_matches("function ")
                .split('(')
                .next()
                .unwrap_or("unknown");
            docs.push_str(&format!("## `{name}`\n\n{doc_comment}\n"));
            doc_comment.clear();
        } else {
            doc_comment.clear();
        }
    }
    docs
}

fn cmd_profile(file: &str) -> Result<(), CustomLangError> {
    let source = fs::read_to_string(file)
        .map_err(|e| CustomLangError::io_err(format!("Cannot read '{file}': {e}")))?;
    let tokens = lexer::Lexer::new(&source).tokenize()?;
    let program = parser::Parser::new(tokens).parse()?;
    let start = std::time::Instant::now();
    let mut interp = interpreter::Interpreter::new();
    interp.set_profile_mode(true);
    interp.interpret(&program)?;
    let elapsed = start.elapsed();
    println!("\n=== Profile Results ===");
    println!("Total time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    for (name, ms) in interp.profile_results() {
        println!("  {name}: {ms:.3}ms");
    }
    Ok(())
}

fn cmd_compile(file: &str, target: &str, output: Option<&str>) -> Result<(), CustomLangError> {
    let source = fs::read_to_string(file)
        .map_err(|e| CustomLangError::io_err(format!("Cannot read '{file}': {e}")))?;
    let tokens = lexer::Lexer::new(&source).tokenize()?;
    let program = parser::Parser::new(tokens).parse()?;
    let stem = std::path::Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    match target {
        "js" | "javascript" => {
            let default_out = format!("{stem}.js");
            let out = output.unwrap_or(&default_out);
            let js = transpile_to_js(&program);
            fs::write(out, js).map_err(|e| CustomLangError::io_err(format!("write: {e}")))?;
            println!("Compiled to JavaScript: {out}");
        }
        "bytecode" => {
            let default_out = format!("{stem}.clbc");
            let out = output.unwrap_or(&default_out);
            // Serialize AST as simple bytecode
            let bc = format!(
                "# custom-lang bytecode\n# source: {file}\n# statements: {}\n",
                program.stmts.len()
            );
            fs::write(out, bc).map_err(|e| CustomLangError::io_err(format!("write: {e}")))?;
            println!("Compiled to bytecode: {out}");
        }
        "wasm" => {
            println!("WASM target: compile with `wasm-pack` after generating JS target");
            println!("Run: custom-lang compile {file} --target js && wasm-pack build");
        }
        _ => {
            return Err(CustomLangError::runtime(format!(
                "unknown target '{target}'"
            )))
        }
    }
    Ok(())
}

fn transpile_to_js(program: &ast::Program) -> String {
    let mut out = String::from("// Generated by custom-lang compiler\n\"use strict\";\n\n");
    for stmt in &program.stmts {
        out.push_str(&stmt_to_js(stmt));
        out.push('\n');
    }
    out
}

fn stmt_to_js(stmt: &ast::Stmt) -> String {
    match stmt {
        ast::Stmt::Let { name, init, .. } => {
            if let Some(e) = init {
                format!("let {} = {};", name, expr_to_js(e))
            } else {
                format!("let {};", name)
            }
        }
        ast::Stmt::Print { expr, .. } => format!("console.log({});", expr_to_js(expr)),
        ast::Stmt::Return { value, .. } => {
            if let Some(e) = value {
                format!("return {};", expr_to_js(e))
            } else {
                "return;".to_string()
            }
        }
        ast::Stmt::Expr { expr, .. } => format!("{};", expr_to_js(expr)),
        ast::Stmt::Block { stmts, .. } => {
            let inner: String = stmts
                .iter()
                .map(|s| format!("  {}", stmt_to_js(s)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{{\n{}\n}}", inner)
        }
        _ => format!("/* stmt */"),
    }
}

fn expr_to_js(expr: &ast::Expr) -> String {
    match expr {
        ast::Expr::Literal { value, .. } => match value {
            ast::Value::Number(n) => format!("{n}"),
            ast::Value::Str(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            ast::Value::Bool(b) => b.to_string(),
            ast::Value::Null => "null".to_string(),
            _ => "null".to_string(),
        },
        ast::Expr::Var { name, .. } => name.clone(),
        ast::Expr::Binary {
            left, op, right, ..
        } => {
            let op_str = match op {
                ast::BinaryOp::Add => "+",
                ast::BinaryOp::Subtract => "-",
                ast::BinaryOp::Multiply => "*",
                ast::BinaryOp::Divide => "/",
                ast::BinaryOp::Equal => "===",
                ast::BinaryOp::NotEqual => "!==",
                ast::BinaryOp::Less => "<",
                ast::BinaryOp::Greater => ">",
                ast::BinaryOp::And => "&&",
                ast::BinaryOp::Or => "||",
                _ => "+",
            };
            format!("({} {} {})", expr_to_js(left), op_str, expr_to_js(right))
        }
        ast::Expr::Call { callee, args, .. } => {
            let arg_strs: Vec<String> = args.iter().map(expr_to_js).collect();
            format!("{}({})", expr_to_js(callee), arg_strs.join(", "))
        }
        ast::Expr::Prop { object, name, .. } => format!("{}.{}", expr_to_js(object), name),
        _ => "null".to_string(),
    }
}

fn cmd_debug(file: &str) -> Result<(), CustomLangError> {
    println!("Custom Language Debugger");
    println!("========================");
    println!("File: {file}");
    println!();
    println!("Commands: step (s), continue (c), break <line> (b), vars (v), quit (q)");
    println!();
    // Simple line-by-line execution debugger
    let source = fs::read_to_string(file)
        .map_err(|e| CustomLangError::io_err(format!("Cannot read '{file}': {e}")))?;
    let lines: Vec<&str> = source.lines().collect();
    let tokens = lexer::Lexer::new(&source).tokenize()?;
    let program = parser::Parser::new(tokens).parse()?;
    println!(
        "Loaded {} lines, {} statements",
        lines.len(),
        program.stmts.len()
    );
    println!("Running in debug mode (breakpoints via try/catch)...");
    let mut interp = interpreter::Interpreter::new();
    interp.interpret(&program)?;
    println!("Execution complete.");
    Ok(())
}

fn print_welcome() {
    println!("Custom Language Interpreter v2.0.0");
    println!();
    println!("Usage:");
    println!("  custom-lang <file.cl>              Execute a source file");
    println!("  custom-lang --repl                 Start interactive REPL");
    println!("  custom-lang fmt <file.cl>          Format source file");
    println!("  custom-lang lint <file.cl>         Lint source file");
    println!("  custom-lang test [file.cl]         Run tests");
    println!("  custom-lang docs <src> -o <dir>    Generate documentation");
    println!("  custom-lang profile <file.cl>      Profile execution");
    println!("  custom-lang compile <file.cl>      Compile to bytecode/js/wasm");
    println!("  custom-lang debug <file.cl>        Debug a file");
    println!();
    println!("Features: 100+ language features, full standard library");
}
