//! Data rules for pack migration (`pack_config_rules.toml`): mod replacements and, from 10.E,
//! config-file migrations.

use super::PackError;
use serde::Deserialize;

const RULES: &str = include_str!("../pack_config_rules.toml");

/// "Mod `from` is replaced by `to`" (data, with synthetic sample names).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReplacementRule {
    pub from: String,
    pub to: String,
    /// Target loader this applies to (`fabric`, `forge`, …); any loader when absent.
    #[serde(default)]
    pub loader: Option<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PackRules {
    #[serde(default)]
    pub replacement: Vec<ReplacementRule>,
}

impl PackRules {
    /// The rules shipped with UMMP.
    ///
    /// # Errors
    /// [`PackError::Invalid`] if the embedded file is malformed (a bug, covered by a test).
    pub fn builtin() -> Result<Self, PackError> {
        Self::from_toml(RULES)
    }

    /// # Errors
    /// [`PackError::Invalid`] on malformed TOML.
    pub fn from_toml(text: &str) -> Result<Self, PackError> {
        toml::from_str(text).map_err(|e| PackError::Invalid(format!("pack rules: {e}")))
    }
}
