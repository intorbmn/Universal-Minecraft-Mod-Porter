//! Phase 12 CLI contract tests against the real binary.

use std::process::{Command, Output};

fn ummp(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(args)
        .output()
        .expect("run ummp")
}

#[test]
fn explicit_toml_config_is_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("custom.toml");
    std::fs::write(&path, "ai_provider = \"openai\"\nopenai_base_url = \"http://localhost:11434/v1\"").unwrap();
    let output = ummp(&["--config", path.to_str().unwrap(), "adapters"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn config_rejects_api_key_fields_without_echoing_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ummp.toml");
    std::fs::write(&path, "anthropic_api_key = \"synthetic-secret-value\"").unwrap();
    let output = ummp(&["--config", path.to_str().unwrap(), "adapters"]);
    assert_eq!(output.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown field"));
    assert!(!stderr.contains("synthetic-secret-value"));
}

#[test]
fn missing_explicit_config_is_invalid_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.toml");
    let output = ummp(&["--config", path.to_str().unwrap(), "adapters"]);
    assert_eq!(output.status.code(), Some(5));
}

#[test]
fn invalid_arguments_use_exit_code_five() {
    let output = ummp(&["--unknown-option"]);
    assert_eq!(output.status.code(), Some(5));
}

#[test]
fn build_rejects_non_project_input_with_code_five() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing-project");
    let output = ummp(&["build", missing.to_str().unwrap()]);
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
