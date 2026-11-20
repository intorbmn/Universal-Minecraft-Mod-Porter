//! Phase 12 CLI contract tests against the real binary.

use std::process::{Command, Output};

fn ummp(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(args)
        .output()
        .expect("run ummp")
}

#[test]
fn invalid_arguments_use_exit_code_five() {
    let output = ummp(&["--unknown-option"]);
    assert_eq!(output.status.code(), Some(5));
}

#[test]
fn help_is_successful() {
    let output = ummp(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn completions_are_generated_for_supported_shells() {
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let output = ummp(&["completions", shell]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{shell}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let script = String::from_utf8_lossy(&output.stdout);
        assert!(script.contains("ummp"), "{shell} completion was empty");
        assert!(script.contains("analyze"), "{shell} lacks subcommands");
    }
}

#[test]
fn scan_json_has_stable_versioned_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let output = ummp(&["scan", dir.path().to_str().unwrap(), "--json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let golden: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/golden/cli/scan-envelope.json"))
            .unwrap();
    let keys = |value: &serde_json::Value| {
        let mut keys = value.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
        keys.sort();
        keys
    };
    assert_eq!(keys(&actual), keys(&golden));
    assert_eq!(actual["schema"], 1);
    assert_eq!(actual["command"], "scan");
    assert!(actual["result"].is_object());
}
