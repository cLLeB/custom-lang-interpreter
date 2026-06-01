//! Language conformance suite.
//!
//! Every `conformance/**/*.cl` program is executed through the real
//! `custom-lang` binary and its stdout is checked against a golden
//! `<file>.cl.out` sidecar. This is the de-facto spec: behavior is defined by
//! the corpus, and any change to it must be a deliberate, reviewed diff.
//!
//! Header directives (lines in the leading comment block):
//!   // args: <extra cli args>   extra flags passed to the interpreter run
//!   // vm: run                  ALSO run on the bytecode VM; its stdout must
//!                               equal the golden (interpreter/VM parity).
//!
//! Regenerate goldens after an *intentional* behavior change, then review the
//! diff before committing:
//!   CONFORMANCE_BLESS=1 cargo test --test conformance

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_custom-lang");

fn collect_cl(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_cl(&p, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some("cl") {
            out.push(p);
        }
    }
}

/// Read a `// key: value` directive from the file's leading comment block.
fn directive(src: &str, key: &str) -> Option<String> {
    src.lines()
        .take_while(|l| {
            let t = l.trim_start();
            t.starts_with("//") || t.is_empty()
        })
        .find_map(|l| {
            let body = l.trim_start().trim_start_matches('/').trim();
            let rest = body.strip_prefix(key)?.trim_start();
            rest.strip_prefix(':').map(|v| v.trim().to_string())
        })
}

fn normalize(s: &[u8]) -> String {
    String::from_utf8_lossy(s).replace("\r\n", "\n")
}

struct RunResult {
    stdout: String,
    stderr: String,
    ok: bool,
}

fn run(args: &[&str]) -> RunResult {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("failed to spawn custom-lang binary");
    RunResult {
        stdout: normalize(&out.stdout),
        stderr: normalize(&out.stderr),
        ok: out.status.success(),
    }
}

#[test]
fn conformance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("conformance");
    let bless = std::env::var("CONFORMANCE_BLESS").is_ok();

    let mut files = Vec::new();
    collect_cl(&root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "no conformance programs found under {}",
        root.display()
    );

    let mut failures: Vec<String> = Vec::new();

    for file in &files {
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .display()
            .to_string()
            .replace('\\', "/");
        let src = fs::read_to_string(file).expect("read .cl source");
        let path_str = file.to_str().expect("utf-8 path");
        let golden_path = PathBuf::from(format!("{path_str}.out"));

        // Interpreter run (the reference engine).
        let extra = directive(&src, "args").unwrap_or_default();
        let mut interp_args: Vec<&str> = extra.split_whitespace().collect();
        interp_args.push(path_str);
        let interp = run(&interp_args);

        // Rejection tests: `// expect-error: <substring>` asserts the program is
        // refused (non-zero exit) with a diagnostic containing the substring.
        // No golden / no VM run.
        if let Some(needle) = directive(&src, "expect-error") {
            if interp.ok {
                failures.push(format!(
                    "{rel}: expected rejection (error containing {needle:?}) but it ran successfully"
                ));
            } else if !interp.stderr.contains(&needle) {
                failures.push(format!(
                    "{rel}: rejected as expected, but diagnostic missing {needle:?}; stderr:\n{}",
                    interp.stderr
                ));
            }
            continue;
        }

        if bless {
            if !interp.ok {
                failures.push(format!(
                    "{rel}: interpreter exited non-zero during bless; stderr:\n{}",
                    interp.stderr
                ));
            }
            fs::write(&golden_path, &interp.stdout).expect("write golden");
        } else {
            let golden = match fs::read_to_string(&golden_path) {
                Ok(g) => g.replace("\r\n", "\n"),
                Err(_) => {
                    failures.push(format!(
                        "{rel}: missing golden file (run `CONFORMANCE_BLESS=1 cargo test --test conformance`)"
                    ));
                    continue;
                }
            };
            if !interp.ok {
                failures.push(format!(
                    "{rel}: interpreter exited non-zero; stderr:\n{}",
                    interp.stderr
                ));
                continue;
            }
            if interp.stdout != golden {
                failures.push(format!(
                    "{rel}: interpreter output mismatch\n--- expected ---\n{golden}\n--- actual ---\n{}",
                    interp.stdout
                ));
                continue;
            }
        }

        // VM parity (opt-in): the bytecode VM must reproduce the golden exactly.
        if directive(&src, "vm").as_deref() == Some("run") {
            let vm = run(&["--vm", path_str]);
            let golden = fs::read_to_string(&golden_path)
                .map(|g| g.replace("\r\n", "\n"))
                .unwrap_or_default();
            if !vm.ok {
                failures.push(format!("{rel}: VM exited non-zero; stderr:\n{}", vm.stderr));
            } else if vm.stdout != golden {
                failures.push(format!(
                    "{rel}: VM output differs from golden (interpreter/VM parity break)\n--- golden ---\n{golden}\n--- vm ---\n{}",
                    vm.stdout
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{} conformance failure(s) across {} program(s):\n\n{}\n",
            failures.len(),
            files.len(),
            failures.join("\n\n")
        );
    }
}
