//! AI key CLI contract tests. All keys are synthetic and isolated in temp directories.

use std::io::Write;
use std::process::{Command, Stdio};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_ummp")
}

#[test]
fn set_reads_stdin_and_status_only_shows_a_masked_key() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(binary())
        .args(["--json", "ai-key", "set", "--provider", "anthropic"])
        .env("UMMP_DATA_DIR", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"sk-ant-test-0000000000000000\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("sk-ant-test-0000000000000000"));
    let set: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(set["schema"], 1);
    assert_eq!(set["result"]["saved"], true);
    assert_eq!(set["result"]["masked"], "sk-ant-…0000");

    let status = Command::new(binary())
        .args(["--json", "ai-key", "status"])
        .env("UMMP_DATA_DIR", dir.path())
        .output()
        .unwrap();
    assert_eq!(status.status.code(), Some(0));
    let status_text = String::from_utf8(status.stdout).unwrap();
    assert!(!status_text.contains("sk-ant-test-0000000000000000"));
    let value: serde_json::Value = serde_json::from_str(&status_text).unwrap();
    assert_eq!(value["result"]["providers"][0]["masked"], "sk-ant-…0000");
}

#[test]
fn a_key_cannot_be_passed_as_a_command_line_argument() {
    let output = Command::new(binary())
        .args(["ai-key", "set", "sk-ant-test-0000000000000000"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(!text.contains("sk-ant-test-0000000000000000"));
}

#[test]
fn delete_removes_only_the_selected_provider() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(binary())
        .args(["ai-key", "set", "--provider", "openai"])
        .env("UMMP_DATA_DIR", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"sk-openai-test-0000000000000000\n").unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(0));

    let deleted = Command::new(binary())
        .args(["ai-key", "delete", "--provider", "openai"])
        .env("UMMP_DATA_DIR", dir.path())
        .output()
        .unwrap();
    assert_eq!(deleted.status.code(), Some(0));

    let status = Command::new(binary())
        .args(["--json", "ai-key", "status", "--provider", "openai"])
        .env("UMMP_DATA_DIR", dir.path())
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(value["result"]["providers"][0]["saved"], false);
}
