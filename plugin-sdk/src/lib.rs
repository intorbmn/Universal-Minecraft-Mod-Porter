//! SDK for UMMP plugins (protocol API 1, see `docs/PLUGIN_PROTOCOL.md`).
//!
//! A plugin is a small executable. UMMP starts it once per call, writes one JSON request
//! line to stdin and reads one JSON response line from stdout. Implement
//! [`PluginAdapter`] and call [`run`] from `main`.
//!
//! Plugins are untrusted and are not sandboxed by the operating system. They propose edits
//! only; UMMP validates every path and applies the edits through its checkpointed engine.
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

/// Protocol version this SDK speaks. UMMP refuses plugins that report another value.
pub const API: u32 = 1;

/// A project file sent by UMMP. `path` always uses `/` separators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub path: String,
    pub content: String,
}

/// The migration target of an `edits` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub minecraft: String,
    pub loader: String,
}

/// One request line from UMMP.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "method", rename_all = "lowercase")]
pub enum Request {
    Hello,
    Findings {
        files: Vec<ProjectFile>,
    },
    Edits {
        files: Vec<ProjectFile>,
        target: Target,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub id: String,
    pub ummp_api: u32,
}

/// How UMMP treats a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Handling {
    Auto,
    Assisted,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub tech: String,
    pub handling: Handling,
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
}

impl Finding {
    #[must_use]
    pub fn new(tech: &str, handling: Handling, file: &str, message: &str) -> Self {
        Self {
            tech: tech.into(),
            handling,
            file: file.into(),
            line: None,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn at_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }
}

/// A proposed whole-file change. `original` must equal the file as UMMP sent it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edit {
    pub path: String,
    pub original: String,
    pub updated: String,
}

impl Edit {
    /// Replace every occurrence of `from`; `None` when the file does not contain it.
    #[must_use]
    pub fn replace_all(file: &ProjectFile, from: &str, to: &str) -> Option<Self> {
        (!from.is_empty() && file.content.contains(from)).then(|| Self {
            path: file.path.clone(),
            original: file.content.clone(),
            updated: file.content.replace(from, to),
        })
    }
}

/// What a plugin implements. Only `id` is required.
pub trait PluginAdapter {
    /// Must equal `name` in `ummp-plugin.toml`.
    fn id(&self) -> &str;

    fn findings(&self, _files: &[ProjectFile]) -> Vec<Finding> {
        Vec::new()
    }

    fn edits(&self, _files: &[ProjectFile], _target: &Target) -> Vec<Edit> {
        Vec::new()
    }
}

#[derive(Debug)]
pub struct SdkError(String);

impl std::fmt::Display for SdkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SdkError {}

/// Answer one request line with one response line (no trailing newline).
pub fn handle(plugin: &impl PluginAdapter, line: &str) -> Result<String, SdkError> {
    let request: Request =
        serde_json::from_str(line.trim()).map_err(|e| SdkError(format!("bad request: {e}")))?;
    let json = match request {
        Request::Hello => serde_json::to_string(&Hello {
            id: plugin.id().to_owned(),
            ummp_api: API,
        }),
        Request::Findings { files } => serde_json::to_string(&plugin.findings(&files)),
        Request::Edits { files, target } => serde_json::to_string(&plugin.edits(&files, &target)),
    };
    json.map_err(|e| SdkError(format!("cannot encode response: {e}")))
}

/// Main loop: read one line from stdin, write one line to stdout. Returns the process exit
/// code (0 on success). Errors go to stderr, which UMMP does not parse.
pub fn run(plugin: &impl PluginAdapter) -> std::process::ExitCode {
    let mut line = String::new();
    let read = std::io::stdin().lock().read_line(&mut line);
    if let Err(e) = read {
        eprintln!("cannot read request: {e}");
        return std::process::ExitCode::FAILURE;
    }
    match handle(plugin, &line) {
        Ok(response) => {
            let mut out = std::io::stdout().lock();
            if writeln!(out, "{response}")
                .and_then(|()| out.flush())
                .is_err()
            {
                return std::process::ExitCode::FAILURE;
            }
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}
