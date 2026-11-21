//! Phase 12 CLI contract tests against the real binary.

use std::process::{Command, Output, Stdio};

fn ummp(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(args)
        .output()
        .expect("run ummp")
}

#[test]
fn explicit_toml_config_is_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let path = dir.path().join("custom.toml");
    let log = dir.path().join("build.log");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/loader/fabric-src");
    std::fs::write(
        &path,
        r#"ai_provider = "openai"
openai_base_url = "http://localhost:11434/v1"
"#,
    )
    .unwrap();
    std::fs::write(&log, "synthetic build failure").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args([
            "--config",
            path.to_str().unwrap(),
            "--json",
            "ai-debug",
            fixture.to_str().unwrap(),
            "--log",
            log.to_str().unwrap(),
            "--from",
            "1.20.1",
            "--to",
            "1.21.1",
            "--loader",
            "fabric",
        ])
        .env("UMMP_DATA_DIR", data.path())
        .env_remove("UMMP_AI_PROVIDER")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["error"]["kind"], "refused");
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("No openai API key"));
}

#[test]
fn config_rejects_api_key_fields_without_echoing_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ummp.toml");
    std::fs::write(&path, "anthropic_api_key = \"synthetic-secret-value\"").unwrap();
    let output = ummp(&["--config", path.to_str().unwrap(), "adapters"]);
    assert_eq!(output.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("secret-like fields"));
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
fn global_json_flag_wraps_scan_view() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(["--json", "scan", dir.path().to_str().unwrap()])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["command"], "scan");
    assert!(value["result"]["findings"].is_array());
}

#[test]
fn global_json_flag_works_after_subcommand() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(["scan", dir.path().to_str().unwrap(), "--json"])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["command"], "scan");
}

#[test]
fn json_errors_have_a_stable_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(["--json", "scan", missing.to_str().unwrap()])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["error"]["kind"], "invalid_input");
    assert!(value["error"]["message"].is_string());
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
        let marker = match shell {
            "bash" => "complete -F",
            "zsh" => "#compdef ummp",
            "fish" => "complete -c ummp",
            "powershell" => "Register-ArgumentCompleter",
            _ => unreachable!(),
        };
        assert!(
            script.contains(marker),
            "{shell} completion registration missing"
        );
    }
}

#[test]
fn bash_completion_can_be_sourced_and_registered() {
    let dir = tempfile::tempdir().unwrap();
    let output = ummp(&["completions", "bash"]);
    assert_eq!(output.status.code(), Some(0));
    let script = dir.path().join("ummp.bash");
    std::fs::write(&script, output.stdout).unwrap();

    let completion = Command::new("bash")
        .arg("-c")
        .arg(r#"source "$1"; complete -p ummp"#)
        .arg("ummp-completion")
        .arg(&script)
        .output()
        .expect("bash must be available for the completion smoke test");
    assert_eq!(
        completion.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&completion.stderr)
    );
    let registration = String::from_utf8_lossy(&completion.stdout);
    assert!(registration.contains("_ummp"), "{registration}");
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
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/golden/cli/scan-envelope.json"
    ))
    .unwrap();
    let keys = |value: &serde_json::Value| {
        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    assert_eq!(keys(&actual), keys(&golden));
    assert_eq!(actual["schema"], 1);
    assert_eq!(actual["command"], "scan");
    assert!(actual["result"].is_object());
}

#[test]
fn adapters_json_has_stable_versioned_result() {
    let data = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(["--json", "adapters"])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["command"], "adapters");
    let result = value["result"].as_object().unwrap();
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/golden/cli/adapters-result.json"
    ))
    .unwrap();
    let keys = |value: &serde_json::Value| {
        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    assert_eq!(keys(&value["result"]), keys(&golden));
    let adapters = result["adapters"].as_array().unwrap();
    assert!(!adapters.is_empty());
    for adapter in adapters {
        assert!(adapter["id"].is_string());
        assert!(adapter["level"].is_string());
    }
}

#[test]
fn port_json_missing_input_uses_invalid_input_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing-mod");
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args([
            "--json",
            "port",
            missing.to_str().unwrap(),
            "--to",
            "1.21.1",
        ])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["error"]["kind"], "invalid_input");
}

#[test]
fn rollback_json_missing_project_uses_invalid_input_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing-project");
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(["--json", "rollback", missing.to_str().unwrap()])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["error"]["kind"], "invalid_input");
}

#[test]
fn build_json_missing_project_uses_invalid_input_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing-project");
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(["--json", "build", missing.to_str().unwrap()])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["error"]["kind"], "invalid_input");
}

#[test]
fn migrate_json_missing_input_uses_invalid_input_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing-mod");
    let mappings = dir.path().join("mappings.json");
    std::fs::write(&mappings, "[]").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args([
            "--json",
            "migrate",
            missing.to_str().unwrap(),
            "--from",
            "1.20.1",
            "--to",
            "1.21.1",
            "--loader",
            "neoforge",
            "--mappings",
            mappings.to_str().unwrap(),
        ])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(5),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["error"]["kind"], "invalid_input");
}

#[test]
fn pack_inspect_json_returns_a_structured_pack_result() {
    let pack = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args(["--json", "pack", "inspect", pack.path().to_str().unwrap()])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["command"], "pack");
    let golden: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/golden/cli/pack-inspect-result.json"
    ))
    .unwrap();
    let keys = |value: &serde_json::Value| {
        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    assert_eq!(keys(&value["result"]), keys(&golden));
    assert_eq!(value["result"]["pack"]["format"], "folder");
    assert!(value["result"]["pack"]["entries"].is_array());
    assert!(value["result"]["issues"].is_array());
}

#[test]
fn ai_debug_missing_key_is_a_json_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let log = dir.path().join("build.log");
    std::fs::write(&log, "synthetic build failure").unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/loader/fabric-src");
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args([
            "--json",
            "ai-debug",
            fixture.to_str().unwrap(),
            "--log",
            log.to_str().unwrap(),
            "--from",
            "1.20.1",
            "--to",
            "1.21.1",
            "--loader",
            "fabric",
        ])
        .env("UMMP_DATA_DIR", data.path())
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout: {}\\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["error"]["kind"], "refused");
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("ai-key set"));
}

#[test]
fn cli_and_environment_provider_values_override_toml() {
    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let config = dir.path().join("ummp.toml");
    let log = dir.path().join("build.log");
    std::fs::write(&config, "ai_provider = \"anthropic\"").unwrap();
    std::fs::write(&log, "synthetic failure").unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/loader/fabric-src");

    let run = |cli_provider: Option<&str>| {
        let mut args = vec![
            "--config",
            config.to_str().unwrap(),
            "--json",
            "ai-debug",
            fixture.to_str().unwrap(),
            "--log",
            log.to_str().unwrap(),
            "--from",
            "1.20.1",
            "--to",
            "1.21.1",
            "--loader",
            "fabric",
        ];
        if let Some(provider) = cli_provider {
            args.extend(["--provider", provider]);
        }
        Command::new(env!("CARGO_BIN_EXE_ummp"))
            .args(args)
            .env("UMMP_DATA_DIR", data.path())
            .env("UMMP_AI_PROVIDER", "openai")
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("OPENAI_API_KEY")
            .output()
            .unwrap()
    };

    // CLI flag overrides both the environment and TOML.
    let cli = run(Some("anthropic"));
    assert_eq!(cli.status.code(), Some(2));
    let cli_error: serde_json::Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert!(cli_error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("No anthropic API key"));

    // Environment overrides TOML when no explicit CLI value is present.
    let env = run(None);
    assert_eq!(env.status.code(), Some(2));
    let env_error: serde_json::Value = serde_json::from_slice(&env.stdout).unwrap();
    assert!(env_error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("No openai API key"));
}

#[cfg(unix)]
#[test]
fn local_build_failure_uses_exit_code_four_and_json_result() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let project = dir.path().join("broken-gradle");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("settings.gradle"),
        "rootProject.name = 'broken'",
    )
    .unwrap();
    std::fs::write(
        project.join("build.gradle"),
        "throw new GradleException('intentional build failure')",
    )
    .unwrap();
    let wrapper = project.join("gradlew");
    std::fs::write(&wrapper, "#!/bin/sh\nexit 1\n").unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args([
            "--json",
            "build",
            project.to_str().unwrap(),
            "--local",
            "--yes",
            "--i-accept-risk",
            "--stages",
            "build",
            "--timeout",
            "10",
        ])
        .env("UMMP_DATA_DIR", data.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(4),
        "stdout: {}\\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["command"], "build");
    assert!(value["result"].is_object());
}

#[test]
fn invalid_openai_base_url_does_not_save_cli_key() {
    let data = tempfile::tempdir().unwrap();
    let key_input = data.path().join("key.txt");
    std::fs::write(&key_input, "sk-openai-test-0000000000000000\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .args([
            "--json",
            "ai-key",
            "set",
            "--provider",
            "openai",
            "--base-url",
            "file:///tmp/invalid",
        ])
        .env("UMMP_DATA_DIR", data.path())
        .stdin(Stdio::from(std::fs::File::open(key_input).unwrap()))
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(5),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["kind"], "invalid_input");
    let credentials = data.path().join("credentials.json");
    assert!(
        !credentials.exists(),
        "invalid base URL must not persist the API key"
    );
}
