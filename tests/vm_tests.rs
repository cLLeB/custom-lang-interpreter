//! End-to-end tests for the bytecode compiler + VM (`compiler` → `bytecode` →
//! `vm`). Each test compiles real source, runs it, and asserts captured
//! `print` output — plus serialization round-trips and honest error paths.

use custom_lang::lexer::Lexer;
use custom_lang::parser::Parser;
use custom_lang::{bytecode, compiler, vm};
use std::rc::Rc;

fn compile(src: &str) -> Result<bytecode::Function, String> {
    let tokens = Lexer::new(src).tokenize().expect("lex failed");
    let program = Parser::new(tokens).parse().expect("parse failed");
    compiler::compile_program(&program).map_err(|e| e.to_string())
}

/// Compile and run on the VM, returning captured output.
fn run(src: &str) -> String {
    let main = compile(src).expect("compile failed");
    vm::Vm::new().run(Rc::new(main)).expect("vm run failed")
}

/// Compile, serialize to .clbc bytes, read back, then run — proving the binary
/// format round-trips.
fn run_roundtrip(src: &str) -> String {
    let main = compile(src).expect("compile failed");
    let bytes = bytecode::serialize(&main);
    let reloaded = bytecode::deserialize(&bytes).expect("deserialize failed");
    vm::Vm::new().run(Rc::new(reloaded)).expect("vm run failed")
}

fn compile_error(src: &str) -> String {
    compile(src).expect_err("expected a compile error")
}

fn run_error(src: &str) -> String {
    let main = compile(src).expect("compile failed");
    vm::Vm::new()
        .run(Rc::new(main))
        .expect_err("expected a runtime error")
        .to_string()
}

// ── Arithmetic & literals ────────────────────────────────────────────────────

#[test]
fn vm_arithmetic_and_precedence() {
    assert_eq!(run("print(1 + 2 * 3)"), "7\n");
    assert_eq!(run("print((1 + 2) * 3)"), "9\n");
    assert_eq!(run("print(2 ** 8)"), "256\n");
    assert_eq!(run("print(17 % 5)"), "2\n");
    assert_eq!(run("print(15 / 4)"), "3.75\n");
    assert_eq!(run("print(-5 + 3)"), "-2\n");
}

#[test]
fn vm_strings_and_bools() {
    assert_eq!(run("print(\"a\" + \"b\" + 1)"), "ab1\n");
    assert_eq!(run("print(1 + \"x\")"), "1x\n");
    assert_eq!(run("print(true)"), "true\n");
    assert_eq!(run("print(!false)"), "true\n");
    assert_eq!(run("print(null)"), "null\n");
}

#[test]
fn vm_comparisons_and_equality() {
    assert_eq!(run("print(3 < 5)"), "true\n");
    assert_eq!(run("print(3 >= 5)"), "false\n");
    assert_eq!(run("print(\"a\" < \"b\")"), "true\n");
    assert_eq!(run("print(2 == 2)"), "true\n");
    assert_eq!(run("print(2 != 3)"), "true\n");
    assert_eq!(run("print(\"x\" == \"x\")"), "true\n");
}

#[test]
fn vm_logical_value_semantics() {
    // && / || return an operand, matching the tree-walker.
    assert_eq!(run("print(0 || 5)"), "5\n");
    assert_eq!(run("print(7 || 0)"), "7\n");
    assert_eq!(run("print(true && 9)"), "9\n");
    assert_eq!(run("print(false && 9)"), "false\n");
}

#[test]
fn vm_ternary() {
    assert_eq!(run("print(10 > 5 ? \"yes\" : \"no\")"), "yes\n");
    assert_eq!(run("print(1 > 5 ? \"yes\" : \"no\")"), "no\n");
}

// ── Variables & scope ────────────────────────────────────────────────────────

#[test]
fn vm_globals_and_assignment() {
    assert_eq!(run("let x = 10\nx = x + 5\nprint(x)"), "15\n");
    assert_eq!(run("let x = 1\nx += 4\nprint(x)"), "5\n");
}

#[test]
fn vm_block_scope_shadowing() {
    let src = "let x = 1\n{\n  let x = 2\n  print(x)\n}\nprint(x)";
    assert_eq!(run(src), "2\n1\n");
}

// ── Control flow ─────────────────────────────────────────────────────────────

#[test]
fn vm_if_else() {
    assert_eq!(
        run("if (3 > 2) { print(\"a\") } else { print(\"b\") }"),
        "a\n"
    );
    assert_eq!(
        run("if (1 > 2) { print(\"a\") } else { print(\"b\") }"),
        "b\n"
    );
}

#[test]
fn vm_while_loop() {
    let src = "let i = 0\nlet sum = 0\nwhile (i < 5) { sum = sum + i\n i = i + 1 }\nprint(sum)";
    assert_eq!(run(src), "10\n");
}

#[test]
fn vm_for_loop_with_break_continue() {
    let src = "let acc = 0\nfor (let i = 0; i < 10; i = i + 1) {\n  if (i == 3) { continue }\n  if (i == 6) { break }\n  acc = acc + i\n}\nprint(acc)";
    // 0 + 1 + 2 + (skip 3) + 4 + 5 = 12, break at 6
    assert_eq!(run(src), "12\n");
}

#[test]
fn vm_do_while() {
    let src = "let i = 0\ndo { print(i)\n i = i + 1 } while (i < 3)";
    assert_eq!(run(src), "0\n1\n2\n");
}

#[test]
fn vm_do_while_with_continue() {
    // `continue` in a do-while must reach the condition re-check, not loop the
    // body forever.
    let src =
        "let i = 0\nlet acc = 0\ndo {\n  i = i + 1\n  if (i == 2) { continue }\n  acc = acc + i\n} while (i < 4)\nprint(acc)";
    // i runs 1,2,3,4; skip adding when i == 2 → 1 + 3 + 4 = 8
    assert_eq!(run(src), "8\n");
}

#[test]
fn vm_for_continue_runs_update() {
    // Regression: `continue` must execute the for-loop's update clause, or the
    // counter never advances and the VM spins forever.
    let src = "let acc = 0\nfor (let i = 0; i < 5; i = i + 1) {\n  if (i == 2) { continue }\n  acc = acc + i\n}\nprint(acc)";
    // 0 + 1 + (skip 2) + 3 + 4 = 8
    assert_eq!(run(src), "8\n");
}

// ── Functions ────────────────────────────────────────────────────────────────

#[test]
fn vm_function_call_and_recursion() {
    let src = "function fib(n) {\n  if (n < 2) { return n }\n  return fib(n - 1) + fib(n - 2)\n}\nprint(fib(10))";
    assert_eq!(run(src), "55\n");
}

#[test]
fn vm_function_locals_and_params() {
    let src = "function add(a, b) {\n  let c = a + b\n  return c\n}\nprint(add(20, 22))";
    assert_eq!(run(src), "42\n");
}

#[test]
fn vm_lambda() {
    let src = "let double = n => n * 2\nprint(double(21))";
    assert_eq!(run(src), "42\n");
}

// ── Serialization round-trip ─────────────────────────────────────────────────

#[test]
fn vm_bytecode_roundtrip_matches_direct() {
    let src = "function sq(n) { return n * n }\nlet t = 0\nfor (let i = 1; i <= 4; i = i + 1) { t = t + sq(i) }\nprint(\"sum of squares: \" + t)";
    assert_eq!(run(src), "sum of squares: 30\n");
    assert_eq!(run_roundtrip(src), run(src));
}

#[test]
fn vm_bytecode_has_magic_header() {
    let main = compile("print(1)").unwrap();
    let bytes = bytecode::serialize(&main);
    assert_eq!(&bytes[0..4], b"CLBC");
}

#[test]
fn vm_deserialize_rejects_garbage() {
    let err = bytecode::deserialize(b"not bytecode").unwrap_err();
    assert!(err.contains("magic"), "unexpected error: {err}");
}

// ── Runtime errors ───────────────────────────────────────────────────────────

#[test]
fn vm_division_by_zero() {
    assert!(run_error("print(1 / 0)").contains("division by zero"));
}

#[test]
fn vm_arity_mismatch() {
    let err = run_error("function f(a, b) { return a }\nprint(f(1))");
    assert!(err.contains("expects 2"), "unexpected error: {err}");
}

#[test]
fn vm_call_non_function() {
    assert!(run_error("let x = 5\nprint(x(1))").contains("not callable"));
}

#[test]
fn vm_undefined_variable() {
    assert!(run_error("print(nope)").contains("undefined variable"));
}

// ── Honest rejection of not-yet-supported constructs ─────────────────────────

#[test]
fn vm_rejects_unsupported_constructs() {
    for (src, needle) in [
        ("class C {}", "classes"),
        ("enum E { A }", "enums"),
        ("let a = [1, 2]", "array literals"),
        ("let o = { a: 1 }", "object literals"),
        ("try { print(1) } catch (e) { print(e) }", "try/catch"),
        ("let x = a ?? b", "null-coalescing"),
    ] {
        let err = compile_error(src);
        assert!(
            err.contains(needle) && err.contains("not yet supported"),
            "source {src:?} expected error containing {needle:?}, got: {err}"
        );
    }
}
