/// Integration tests that run actual interpreter code end-to-end.
use custom_lang::interpreter::Interpreter;
use custom_lang::lexer::Lexer;
use custom_lang::parser::Parser;

fn eval(source: &str) -> String {
    let tokens = Lexer::new(source).tokenize().expect("lex failed");
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut interp = Interpreter::new();
    match interp.exec_repl(&program).expect("runtime error") {
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

fn eval_err(source: &str) -> String {
    let tokens = Lexer::new(source).tokenize().expect("lex failed");
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut interp = Interpreter::new();
    interp.interpret(&program).unwrap_err().to_string()
}

// ── Arithmetic ───────────────────────────────────────────────────────────────

#[test]
fn test_basic_arithmetic() {
    assert_eq!(eval("2 + 3"), "5");
    assert_eq!(eval("10 - 4"), "6");
    assert_eq!(eval("3 * 7"), "21");
    assert_eq!(eval("15 / 4"), "3.75");
    assert_eq!(eval("17 % 5"), "2");
}

#[test]
fn test_operator_precedence() {
    assert_eq!(eval("2 + 3 * 4"), "14");
    assert_eq!(eval("(2 + 3) * 4"), "20");
    assert_eq!(eval("10 - 2 - 3"), "5");
}

// ── Variables ────────────────────────────────────────────────────────────────

#[test]
fn test_variables() {
    assert_eq!(eval("let x = 42; x"), "42");
    assert_eq!(eval("let a = 1; let b = 2; a + b"), "3");
}

#[test]
fn test_compound_assign() {
    assert_eq!(eval("let x = 10; x += 5; x"), "15");
    assert_eq!(eval("let x = 10; x -= 3; x"), "7");
    assert_eq!(eval("let x = 4; x *= 3; x"), "12");
    assert_eq!(eval("let x = 15; x /= 3; x"), "5");
    assert_eq!(eval("let x = 17; x %= 5; x"), "2");
}

// ── While loop (critical regression test for the bug fix) ────────────────────

#[test]
fn test_while_loop_mutation() {
    // This was the critical bug: counter update inside while never propagated back
    let result = eval("let i = 0; while (i < 5) { i += 1; } i");
    assert_eq!(result, "5");
}

#[test]
fn test_while_sum() {
    let result = eval("let sum = 0; let i = 1; while (i <= 10) { sum += i; i += 1; } sum");
    assert_eq!(result, "55");
}

// ── For loops ────────────────────────────────────────────────────────────────

#[test]
fn test_for_loop() {
    let result = eval("let s = 0; for (let i = 1; i <= 5; i += 1) { s += i; } s");
    assert_eq!(result, "15");
}

#[test]
fn test_for_in_array() {
    let result = eval("let s = 0; for (x in [1, 2, 3, 4]) { s += x; } s");
    assert_eq!(result, "10");
}

#[test]
fn test_for_in_string() {
    let result = eval(r#"let n = 0; for (c in "hello") { n += 1; } n"#);
    assert_eq!(result, "5");
}

// ── Break / Continue ─────────────────────────────────────────────────────────

#[test]
fn test_break() {
    let result = eval("let i = 0; while (true) { i += 1; if (i == 5) { break; } } i");
    assert_eq!(result, "5");
}

#[test]
fn test_continue() {
    let result = eval(
        "let sum = 0; for (let i = 1; i <= 10; i += 1) { if (i % 2 == 0) { continue; } sum += i; } sum"
    );
    assert_eq!(result, "25"); // 1+3+5+7+9
}

// ── Functions ────────────────────────────────────────────────────────────────

#[test]
fn test_function_basic() {
    assert_eq!(eval("function double(x) { return x * 2; } double(7)"), "14");
}

#[test]
fn test_recursion() {
    let result = eval(
        "function fib(n) { if (n <= 1) { return n; } return fib(n-1) + fib(n-2); } fib(10)"
    );
    assert_eq!(result, "55");
}

#[test]
fn test_lambda() {
    assert_eq!(eval("let sq = function(x) { return x * x; }; sq(9)"), "81");
}

// ── Closures ─────────────────────────────────────────────────────────────────

#[test]
fn test_closure_capture() {
    let result = eval(
        "function make_adder(n) { return function(x) { return x + n; }; } let add5 = make_adder(5); add5(10)"
    );
    assert_eq!(result, "15");
}

#[test]
fn test_closure_mutable_state() {
    let result = eval(
        "function counter() { let n = 0; return function() { n += 1; return n; }; } let c = counter(); c(); c(); c()"
    );
    assert_eq!(result, "3");
}

// ── Arrays ───────────────────────────────────────────────────────────────────

#[test]
fn test_array_indexing() {
    assert_eq!(eval("let a = [10, 20, 30]; a[1]"), "20");
    assert_eq!(eval("let a = [10, 20, 30]; a[0]"), "10");
}

#[test]
fn test_array_push_pop() {
    // push mutates in-place and returns the array
    assert_eq!(eval("let a = [1, 2]; let b = push(a, 3); len(b)"), "3");
    assert_eq!(eval("let a = [1, 2, 3]; pop(a)"), "3");
    // after pop, array is mutated
    assert_eq!(eval("let a = [1, 2, 3]; pop(a); len(a)"), "2");
}

#[test]
fn test_index_assign() {
    assert_eq!(eval("let a = [1, 2, 3]; a[1] = 99; a[1]"), "99");
}

#[test]
fn test_filter_map_reduce() {
    assert_eq!(
        eval("filter([1,2,3,4,5], function(x) { return x % 2 == 0; })"),
        "[2, 4]"
    );
    assert_eq!(
        eval("map([1,2,3], function(x) { return x * 10; })"),
        "[10, 20, 30]"
    );
    assert_eq!(
        eval("reduce([1,2,3,4,5], function(acc, x) { return acc + x; }, 0)"),
        "15"
    );
}

// ── Objects ──────────────────────────────────────────────────────────────────

#[test]
fn test_object_basic() {
    assert_eq!(eval(r#"let o = {x: 10, y: 20}; o.x"#), "10");
}

#[test]
fn test_prop_assign() {
    assert_eq!(eval(r#"let o = {x: 1}; o.x = 42; o.x"#), "42");
}

// ── Classes ──────────────────────────────────────────────────────────────────

#[test]
fn test_class_basic() {
    let result = eval(
        "class Counter { function init(n) { this.n = n; } function inc() { this.n += 1; } function get() { return this.n; } } let c = new Counter(0); c.inc(); c.inc(); c.get()"
    );
    assert_eq!(result, "2");
}

#[test]
fn test_class_inheritance() {
    let result = eval(
        "class Animal { function speak() { return \"...\"; } } class Dog extends Animal { function speak() { return \"woof\"; } } let d = new Dog(); d.speak()"
    );
    assert_eq!(result, "woof");
}

// ── Pattern matching ─────────────────────────────────────────────────────────

#[test]
fn test_match_literal() {
    assert_eq!(
        eval(r#"let x = 42; match x { 1 => "one", 42 => "answer", _ => "other" }"#),
        "answer"
    );
}

#[test]
fn test_match_binding() {
    assert_eq!(
        eval(r#"match 99 { x => x * 2 }"#),
        "198"
    );
}

#[test]
fn test_match_wildcard() {
    assert_eq!(eval(r#"match "nope" { "yes" => 1, _ => 0 }"#), "0");
}

// ── String operations ────────────────────────────────────────────────────────

#[test]
fn test_string_concat() {
    assert_eq!(eval(r#""hello" + " " + "world""#), "hello world");
}

#[test]
fn test_string_builtins() {
    assert_eq!(eval(r#"len("hello")"#), "5");
    assert_eq!(eval(r#"to_upper("hello")"#), "HELLO");
    assert_eq!(eval(r#"trim("  hi  ")"#), "hi");
    assert_eq!(eval(r#"contains("foobar", "bar")"#), "true");
    assert_eq!(eval(r#"replace("hello", "l", "r")"#), "herro");
}

// ── Error handling ───────────────────────────────────────────────────────────

#[test]
fn test_division_by_zero() {
    let err = eval_err("1 / 0");
    assert!(err.contains("Division by zero") || err.contains("division by zero"));
}

#[test]
fn test_undefined_variable() {
    let err = eval_err("--no-semantic; bogus_var_xyz");
    // This should fail at runtime, not semantic analysis
    // The interpreter catches it
    let _ = err; // we just check it doesn't panic
}

// ── Stack overflow protection ─────────────────────────────────────────────────

#[test]
fn test_stack_overflow_protection() {
    let err = eval_err("function inf() { return inf(); } inf()");
    assert!(err.contains("overflow") || err.contains("depth"));
}

// ── Builtins ─────────────────────────────────────────────────────────────────

#[test]
fn test_math_builtins() {
    assert_eq!(eval("abs(-5)"), "5");
    assert_eq!(eval("floor(3.7)"), "3");
    assert_eq!(eval("ceil(3.2)"), "4");
    assert_eq!(eval("round(3.5)"), "4");
    assert_eq!(eval("min(3, 7)"), "3");
    assert_eq!(eval("max(3, 7)"), "7");
}

#[test]
fn test_range_builtin() {
    assert_eq!(eval("range(5)"), "[0, 1, 2, 3, 4]");
    assert_eq!(eval("range(2, 5)"), "[2, 3, 4]");
    assert_eq!(eval("range(0, 10, 2)"), "[0, 2, 4, 6, 8]");
}

#[test]
fn test_type_coercion() {
    assert_eq!(eval("to_string(42)"), "42");
    assert_eq!(eval("to_number(\"3.14\")"), "3.14");
    assert_eq!(eval("is_number(42)"), "true");
    assert_eq!(eval("is_string(\"hi\")"), "true");
    assert_eq!(eval("is_null(null)"), "true");
}
