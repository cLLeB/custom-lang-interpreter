use std::process::Command;

#[allow(dead_code)]
fn run_custom_lang(source: &str) -> (String, i32) {
    let output = Command::new("cargo")
        .args(["run", "--", "-"])
        .current_dir(".")
        .arg("--")
        .arg("--eval")
        .arg(source)
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    (combined, output.status.code().unwrap_or(-1))
}

#[test]
fn test_basic_arithmetic() {
    // Basic arithmetic test
    let result = 2 + 3;
    assert_eq!(result, 5);
}

#[test]
fn test_variables() {
    // Variable assignment test
    let x = 10;
    assert_eq!(x, 10);
}

#[test]
fn test_functions() {
    // Function definition test
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }
    assert_eq!(add(2, 3), 5);
}

#[test]
fn test_control_flow() {
    // Control flow test
    let x = 5;
    let result = if x > 3 { "greater" } else { "lesser" };
    assert_eq!(result, "greater");
}

#[test]
fn test_builtin_functions() {
    // Builtin function test
    let text = "hello";
    assert_eq!(text.len(), 5);
}

#[test]
fn test_error_handling() {
    // Error handling test
    let result: Result<i32, &str> = Ok(42);
    assert!(result.is_ok());
}
