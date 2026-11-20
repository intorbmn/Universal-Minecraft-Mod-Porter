//! CLI configuration and precedence: flags > environment > file > built-ins.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct CliConfig {
    pub output_dir: Option<PathBuf>,
    pub sandbox_repo: Option<String>,
    pub provider: Option<String>,
}

impl CliConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::current_dir()?.join("ummp.toml");
        let path = if path.is_file() {
            Some(path)
        } else {
            user_config_dir().map(|dir| dir.join("ummp.toml")).filter(|p| p.is_file())
        };
        let mut config: Self = match path {
            Some(path) => ummp_core::config::load_any(&path)?,
            None => Self::default(),
        };
        if let Some(value) = nonempty_env("UMMP_OUTPUT_DIR") {
            config.output_dir = Some(PathBuf::from(value));
        }
        if let Some(value) = nonempty_env("UMMP_BUILD_REPO") {
            config.sandbox_repo = Some(value);
        }
        if let Some(value) = nonempty_env("UMMP_PROVIDER") {
            config.provider = Some(value);
        }
        Ok(config)
    }

    #[must_use]
    pub fn output_dir(&self, cli: Option<PathBuf>) -> Option<PathBuf> {
        cli.or_else(|| self.output_dir.clone())
    }

    #[must_use]
    pub fn sandbox_repo(&self, cli: Option<String>) -> Option<String> {
        cli.or_else(|| self.sandbox_repo.clone())
    }

    #[must_use]
    pub fn provider(&self, cli: Option<String>) -> String {
        cli.or_else(|| self.provider.clone())
            .unwrap_or_else(|| "anthropic".into())
    }
}

fn nonempty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.trim().is_empty())
}

fn user_config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(PathBuf::from).map(|p| p.join("ummp"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|p| p.join("Library/Application Support/ummp"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from).map(|p| p.join(".config")))
            .map(|p| p.join("ummp"))
    }
    #[cfg(not(any(windows, unix)))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::CliConfig;
    use std::path::PathBuf;

    #[test]
    fn cli_values_override_config_values() {
        let config = CliConfig {
            output_dir: Some(PathBuf::from("from-file")),
            sandbox_repo: Some("file/repo".into()),
            provider: Some("openai".into()),
        };
        assert_eq!(config.output_dir(Some("from-cli".into())), Some("from-cli".into()));
        assert_eq!(config.sandbox_repo(Some("cli/repo".into())).as_deref(), Some("cli/repo"));
        assert_eq!(config.provider(Some("anthropic".into())), "anthropic");
    }

    #[test]
    fn config_values_are_used_when_cli_values_are_absent() {
        let config = CliConfig {
            output_dir: Some(PathBuf::from("from-file")),
            sandbox_repo: Some("file/repo".into()),
            provider: Some("openai".into()),
        };
        assert_eq!(config.output_dir(None), Some("from-file".into()));
        assert_eq!(config.sandbox_repo(None).as_deref(), Some("file/repo"));
        assert_eq!(config.provider(None), "openai");
    }

    #[test]
    fn config_file_default_is_overridden_by_flag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ummp.toml");
        std::fs::write(&path, "output_dir = \"from-file\"\nsandbox_repo = \"file/repo\"\nprovider = \"openai\"\n").unwrap();
        let config: CliConfig = ummp_core::config::load_any(&path).unwrap();
        assert_eq!(config.output_dir(None), Some(PathBuf::from("from-file")));
        assert_eq!(config.output_dir(Some(PathBuf::from("from-cli"))), Some(PathBuf::from("from-cli")));
    }

    #[test]
    fn built_in_provider_is_anthropic() {
        assert_eq!(CliConfig::default().provider(None), "anthropic");
    }
}
