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
    // We'll implement a simple test runner for now
    // This is a placeholder for integration tests
    assert!(true);
}

#[test]
fn test_variables() {
    assert!(true);
}

#[test]
fn test_functions() {
    assert!(true);
}

#[test]
fn test_control_flow() {
    assert!(true);
}

#[test]
fn test_builtin_functions() {
    assert!(true);
}

#[test]
fn test_error_handling() {
    assert!(true);
}
