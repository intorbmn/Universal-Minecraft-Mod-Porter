//! Out-of-process plugin protocol (Phase 12.E).
use crate::adapter::{AdapterContext, Handling, Level, TechAdapter, TechFinding};
use crate::build::process::{run_with_input, scrubbed_env, ProcSpec, ProcStatus};
use crate::filesystem::resolve_within;
use crate::migration::{EditSet, FileEdit, MigrationError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const PLUGIN_MANIFEST: &str = "ummp-plugin.toml";
pub const PLUGIN_API: u32 = 1;
pub const PLUGIN_TIMEOUT: Duration = Duration::from_secs(30);
pub const PLUGIN_STDOUT_LIMIT: usize = 8 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind { Adapter, Rules }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub command: Vec<String>,
    pub ummp_api: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub ummp_api: u32,
    pub directory: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid plugin manifest: {0}")]
    Manifest(String),
    #[error("plugin protocol: {0}")]
    Protocol(String),
    #[error("plugin API {found} is incompatible; UMMP supports API {supported}")]
    Api { found: u32, supported: u32 },
    #[error("plugin timed out after {0}s")]
    Timeout(u64),
    #[error("plugin stdout exceeded {0} bytes")]
    OutputLimit(usize),
}

#[derive(Serialize)]
struct PluginFile { path: String, content: String }
#[derive(Deserialize)]
struct PluginEdit { path: PathBuf, original: String, updated: String }

pub struct PluginManager { root: PathBuf }

impl PluginManager {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self { Self { root: root.into() } }

    pub fn default_dir() -> Result<PathBuf, crate::app::AppError> {
        Ok(crate::app::credentials::default_data_dir()?.join("plugins"))
    }

    pub fn list(&self) -> Result<Vec<PluginInfo>, PluginError> {
        if !self.root.exists() { return Ok(Vec::new()); }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() { out.push(PluginAdapter::load(&entry.path())?.info()); }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn check(&self, directory: &Path) -> Result<PluginInfo, PluginError> {
        let plugin = PluginAdapter::load(directory)?;
        plugin.hello()?;
        Ok(plugin.info())
    }

    pub fn adapters(&self) -> Result<Vec<Box<dyn TechAdapter>>, PluginError> {
        if !self.root.exists() { return Ok(Vec::new()); }
        let mut out: Vec<Box<dyn TechAdapter>> = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() { out.push(Box::new(PluginAdapter::load(&entry.path())?)); }
        }
        Ok(out)
    }
}

struct PluginAdapter { info: PluginInfo, program: PathBuf, args: Vec<String> }

impl PluginAdapter {
    fn load(directory: &Path) -> Result<Self, PluginError> {
        let directory = directory.canonicalize()
            .map_err(|e| PluginError::Manifest(format!("plugin directory unavailable: {e}")))?;
        let manifest_path = directory.join(PLUGIN_MANIFEST);
        let meta = std::fs::symlink_metadata(&manifest_path)
            .map_err(|e| PluginError::Manifest(format!("{PLUGIN_MANIFEST}: {e}")))?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(PluginError::Manifest("manifest must be a regular file".into()));
        }
        let manifest: PluginManifest = toml::from_str(&std::fs::read_to_string(manifest_path)?)
            .map_err(|e| PluginError::Manifest(e.to_string()))?;
        if manifest.name.trim().is_empty() || manifest.version.trim().is_empty()
            || manifest.command.is_empty() || manifest.command[0].trim().is_empty() {
            return Err(PluginError::Manifest("name, version and command are required".into()));
        }
        if manifest.ummp_api != PLUGIN_API {
            return Err(PluginError::Api { found: manifest.ummp_api, supported: PLUGIN_API });
        }
        let candidate = resolve_within(&directory, Path::new(&manifest.command[0]))
            .map_err(|_| PluginError::Manifest("command must stay inside plugin folder".into()))?;
        let program = candidate.canonicalize()
            .map_err(|e| PluginError::Manifest(format!("plugin executable unavailable: {e}")))?;
        if !program.starts_with(&directory) || !program.is_file() || forbidden(&program) {
            return Err(PluginError::Manifest("command must resolve to an allowed file inside plugin folder".into()));
        }
        Ok(Self {
            info: PluginInfo { name: manifest.name, version: manifest.version, kind: manifest.kind,
                ummp_api: manifest.ummp_api, directory },
            program, args: manifest.command.into_iter().skip(1).collect(),
        })
    }

    fn info(&self) -> PluginInfo { self.info.clone() }

    fn invoke(&self, request: &Value) -> Result<Value, PluginError> {
        let mut input = serde_json::to_vec(request).map_err(|e| PluginError::Protocol(e.to_string()))?;
        input.push(b'\n');
        let spec = ProcSpec { program: self.program.clone(), args: self.args.clone(),
            cwd: self.info.directory.clone(), env: scrubbed_env(), timeout: PLUGIN_TIMEOUT };
        let (status, output) = match run_with_input(&spec, &input, PLUGIN_STDOUT_LIMIT) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => return Err(PluginError::OutputLimit(PLUGIN_STDOUT_LIMIT)),
            Err(e) => return Err(PluginError::Io(e)),
        };
        match status {
            ProcStatus::TimedOut => return Err(PluginError::Timeout(PLUGIN_TIMEOUT.as_secs())),
            ProcStatus::Exited(0) => {}
            ProcStatus::Exited(code) => return Err(PluginError::Protocol(format!("plugin exited with code {code}"))),
            ProcStatus::WatchMatched => return Err(PluginError::Protocol("unexpected process status".into())),
        }
        let text = std::str::from_utf8(&output).map_err(|_| PluginError::Protocol("response is not UTF-8".into()))?;
        let mut lines = text.lines().filter(|line| !line.trim().is_empty());
        let line = lines.next().ok_or_else(|| PluginError::Protocol("plugin returned no response".into()))?;
        if lines.next().is_some() { return Err(PluginError::Protocol("expected exactly one JSON response line".into())); }
        serde_json::from_str(line).map_err(|e| PluginError::Protocol(e.to_string()))
    }

    fn hello(&self) -> Result<(), PluginError> {
        let reply = self.invoke(&json!({"method":"hello"}))?;
        let api = reply.get("ummp_api").and_then(Value::as_u64).and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| PluginError::Protocol("hello response has no ummp_api".into()))?;
        if api != PLUGIN_API { return Err(PluginError::Api { found: api, supported: PLUGIN_API }); }
        if reply.get("id").and_then(Value::as_str) != Some(&self.info.name) {
            return Err(PluginError::Protocol("hello id does not match manifest name".into()));
        }
        Ok(())
    }

    fn files(&self, cx: &AdapterContext<'_>) -> Result<Vec<PluginFile>, PluginError> {
        let mut files = Vec::new();
        for rel in cx.files {
            if forbidden(rel) || sensitive(rel) { continue; }
            let path = resolve_within(cx.root, rel)
                .map_err(|_| PluginError::Protocol("project file path escapes root".into()))?;
            let meta = std::fs::metadata(&path)?;
            if meta.len() > MAX_FILE_BYTES { continue; }
            if let Ok(content) = std::fs::read_to_string(path) {
                files.push(PluginFile { path: rel.to_string_lossy().replace('\\', "/"), content });
            }
        }
        Ok(files)
    }

    fn validate_path(&self, root: &Path, rel: &Path, files: &[PathBuf]) -> Result<PathBuf, PluginError> {
        if rel.as_os_str().is_empty() || rel.is_absolute() || forbidden(rel) || sensitive(rel)
            || !files.iter().any(|path| path == rel) {
            return Err(PluginError::Protocol(format!("plugin returned a disallowed path: {}", rel.display())));
        }
        let absolute = resolve_within(root, rel)
            .map_err(|_| PluginError::Protocol("plugin path escapes project root".into()))?;
        let canonical_root = root.canonicalize()?;
        let canonical = absolute.canonicalize()?;
        if !canonical.starts_with(canonical_root) || !canonical.is_file() {
            return Err(PluginError::Protocol("plugin path resolves outside project root".into()));
        }
        Ok(canonical)
    }
}

fn forbidden(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(),
        Some("jar" | "class"))
}
fn sensitive(path: &Path) -> bool {
    path.components().filter_map(|c| c.as_os_str().to_str()).any(|part| {
        let part = part.to_ascii_lowercase();
        part == ".env" || part.starts_with(".env.") || part.contains("credential")
            || part.contains("secret") || part.contains("token") || part == ".npmrc"
            || part == "settings.xml"
    })
}

impl TechAdapter for PluginAdapter {
    fn id(&self) -> &str { &self.info.name }
    fn level(&self) -> Level { Level::Migrate }

    fn findings(&self, cx: &AdapterContext<'_>) -> Vec<TechFinding> {
        let result = (|| {
            let files = self.files(cx)?;
            let value = self.invoke(&json!({"method":"findings","files":files}))?;
            let findings: Vec<TechFinding> = serde_json::from_value(value)
                .map_err(|e| PluginError::Protocol(e.to_string()))?;
            for finding in &findings {
                self.validate_path(cx.root, Path::new(&finding.file), cx.files)?;
            }
            Ok::<_, PluginError>(findings)
        })();
        result.unwrap_or_else(|e| vec![TechFinding { tech: self.info.name.clone(),
            handling: Handling::Manual, file: String::new(), line: None,
            message: format!("plugin failed: {e}") }])
    }

    fn edits(&self, cx: &AdapterContext<'_>) -> Result<EditSet, MigrationError> {
        let Some(plan) = cx.plan else { return Ok(EditSet::default()); };
        let files = self.files(cx).map_err(|e| MigrationError::InvalidRequest(e.to_string()))?;
        let value = self.invoke(&json!({"method":"edits","files":files,"target":{
            "minecraft":plan.request.target_minecraft,"loader":plan.request.target_loader
        }})).map_err(|e| MigrationError::InvalidRequest(e.to_string()))?;
        let edits: Vec<PluginEdit> = serde_json::from_value(value)
            .map_err(|e| MigrationError::InvalidRequest(format!("invalid plugin edits: {e}")))?;
        let mut set = EditSet::default();
        for edit in edits {
            let path = self.validate_path(cx.root, &edit.path, cx.files)
                .map_err(|e| MigrationError::InvalidRequest(e.to_string()))?;
            if std::fs::read_to_string(&path)? != edit.original {
                return Err(MigrationError::InvalidRequest(format!("plugin edit for {} is stale", edit.path.display())));
            }
            set.edits.push(FileEdit { path: edit.path, original: edit.original,
                updated: edit.updated, replacements: Vec::new() });
        }
        Ok(set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_jar_class_and_secret_paths() {
        assert!(forbidden(Path::new("a/B.JAR")));
        assert!(forbidden(Path::new("a/B.class")));
        assert!(sensitive(Path::new(".env.production")));
        assert!(sensitive(Path::new("config/credentials.json")));
        assert!(!forbidden(Path::new("src/Main.java")));
    }

    #[test]
    fn rejects_parent_executable_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(PLUGIN_MANIFEST),
            "name='demo'\nversion='1'\nkind='rules'\ncommand=['../outside']\nummp_api=1\n").unwrap();
        assert!(PluginAdapter::load(dir.path()).is_err());
    }

    #[test]
    fn refuses_api_two() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(PLUGIN_MANIFEST),
            "name='demo'\nversion='1'\nkind='rules'\ncommand=['bin/plugin']\nummp_api=2\n").unwrap();
        assert!(matches!(PluginAdapter::load(dir.path()), Err(PluginError::Api { found: 2, .. })));
    }
}
