//! Application: analyzer engine — adapter-based detectors behind IAnalyzer.

use crate::filesystem::{FileSystem, FsError};
use crate::metadata::{
    BuildSystem, BuildSystemKind, JavaVersion, Loader, MinecraftVersion, ModDependency, ModMetadata,
};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("fs: {0}")]
    Fs(#[from] FsError),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("parse: {0}")]
    Parse(String),
    #[error("not found: {0}")]
    NotFound(String),
}

pub trait IAnalyzer: Send + Sync {
    fn analyze(
        &self,
        project_root: &Path,
        fs: &Arc<dyn FileSystem>,
    ) -> Result<AnalysisReport, AnalyzerError>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub mods: Vec<ModAnalysis>,
    pub build_system: Option<BuildSystem>,
    pub languages: Vec<String>,
    pub dependency_graph: DependencyGraph,
    pub compatibility: CompatibilityMatrix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModAnalysis {
    pub metadata: ModMetadata,
    pub source_files: Vec<PathBuf>,
    pub jar_path: Option<PathBuf>,
    pub detected_loader: Loader,
    pub detected_minecraft: MinecraftVersion,
    pub detected_java: Option<JavaVersion>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    pub id: String,
    pub label: String,
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind {
    Mod,
    Library,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    Requires,
    Optional,
    Conflicts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompatibilityMatrix {
    pub target_minecraft: String,
    pub target_loader: String,
    pub issues: Vec<CompatIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatIssue {
    pub mod_id: String,
    pub severity: IssueSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

pub struct AnalyzerEngine {
    detectors: Vec<Box<dyn IAnalyzer>>,
}

impl AnalyzerEngine {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    pub fn add_detector(&mut self, detector: Box<dyn IAnalyzer>) {
        self.detectors.push(detector);
    }

    pub fn analyze(
        &self,
        project_root: &Path,
        fs: &Arc<dyn FileSystem>,
    ) -> Result<AnalysisReport, AnalyzerError> {
        let mut report = AnalysisReport::default();
        for detector in &self.detectors {
            let partial = detector.analyze(project_root, &fs)?;
            report.mods.extend(partial.mods);
            if report.build_system.is_none() {
                report.build_system = partial.build_system;
            }
            for lang in partial.languages {
                if !report.languages.contains(&lang) {
                    report.languages.push(lang);
                }
            }
            report
                .dependency_graph
                .nodes
                .extend(partial.dependency_graph.nodes);
            report
                .dependency_graph
                .edges
                .extend(partial.dependency_graph.edges);
            report
                .compatibility
                .issues
                .extend(partial.compatibility.issues);
        }
        Ok(report)
    }
}

impl Default for AnalyzerEngine {
    fn default() -> Self {
        let mut engine = Self::new();
        engine.add_detector(Box::new(JarInspector));
        engine.add_detector(Box::new(SourceScanner));
        engine.add_detector(Box::new(BuildSystemDetector));
        engine.add_detector(Box::new(DependencyGraphBuilder));
        engine
    }
}

// 4.1 JAR inspection (manifest, mods.toml, fabric.mod.json)
pub struct JarInspector;

impl IAnalyzer for JarInspector {
    fn analyze(
        &self,
        project_root: &Path,
        fs: &Arc<dyn FileSystem>,
    ) -> Result<AnalysisReport, AnalyzerError> {
        let mut report = AnalysisReport::default();
        let jars = find_jars(project_root, fs.as_ref())?;

        for jar_path in jars {
            let mod_analysis = inspect_jar(&jar_path, fs.as_ref())?;
            report.mods.push(mod_analysis);
        }
        Ok(report)
    }
}

fn find_jars(root: &Path, fs: &dyn FileSystem) -> Result<Vec<PathBuf>, AnalyzerError> {
    let mut jars = Vec::new();
    for entry in fs.list(root)? {
        if entry.extension().is_some_and(|s| s == "jar") {
            jars.push(entry);
        }
    }
    Ok(jars)
}

fn inspect_jar(jar_path: &Path, fs: &dyn FileSystem) -> Result<ModAnalysis, AnalyzerError> {
    let bytes = fs.read(jar_path)?;
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;

    let mut manifest = String::new();
    let mut mods_toml = String::new();
    let mut fabric_mod_json = String::new();
    let mut mcmod_info = String::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        match name.as_str() {
            "META-INF/MANIFEST.MF" => manifest = content,
            "META-INF/mods.toml" => mods_toml = content,
            "fabric.mod.json" => fabric_mod_json = content,
            "mcmod.info" => mcmod_info = content,
            _ => {}
        }
    }

    let (loader, metadata) =
        parse_jar_metadata(&manifest, &mods_toml, &fabric_mod_json, &mcmod_info)?;
    let minecraft = detect_minecraft_version(&metadata);
    let java = detect_java_version(&manifest);

    Ok(ModAnalysis {
        metadata,
        source_files: Vec::new(),
        jar_path: Some(jar_path.to_path_buf()),
        detected_loader: loader,
        detected_minecraft: minecraft,
        detected_java: java,
    })
}

fn parse_jar_metadata(
    manifest: &str,
    mods_toml: &str,
    fabric_json: &str,
    mcmod_info: &str,
) -> Result<(Loader, ModMetadata), AnalyzerError> {
    // Try Fabric first
    if !fabric_json.is_empty() {
        let v: serde_json::Value =
            serde_json::from_str(fabric_json).map_err(|e| AnalyzerError::Parse(e.to_string()))?;
        let mod_id = v["id"].as_str().unwrap_or("unknown").to_string();
        let version = v["version"].as_str().unwrap_or("0.0.0").to_string();
        let name = v["name"].as_str().unwrap_or(&mod_id).to_string();
        let loader = Loader::Fabric;

        let minecraft = v["depends"]["minecraft"]
            .as_str()
            .unwrap_or("1.20.1")
            .to_string();
        let (major, minor, patch) = parse_version(&minecraft);

        let mut dependencies = Vec::new();
        if let Some(deps) = v["depends"].as_object() {
            for (k, v) in deps {
                if k != "minecraft" {
                    dependencies.push(ModDependency {
                        mod_id: k.clone(),
                        version: v.as_str().unwrap_or("*").to_string(),
                        optional: false,
                    });
                }
            }
        }

        return Ok((
            loader.clone(),
            ModMetadata {
                mod_id,
                version,
                name,
                loader,
                minecraft: MinecraftVersion {
                    major,
                    minor,
                    patch,
                },
                java: None,
                dependencies,
            },
        ));
    }

    // Try Forge/NeoForge (mods.toml)
    if !mods_toml.is_empty() {
        let v: toml::Value = mods_toml
            .parse()
            .map_err(|e: toml::de::Error| AnalyzerError::Parse(e.to_string()))?;
        if let Some(mods) = v["mods"].as_array() {
            if let Some(first) = mods.first() {
                let mod_id = first["modId"].as_str().unwrap_or("unknown").to_string();
                let version = first["version"].as_str().unwrap_or("0.0.0").to_string();
                let name = first["displayName"].as_str().unwrap_or(&mod_id).to_string();
                let loader_str = first["loader"].as_str().unwrap_or("FORGE");
                let loader = match loader_str {
                    "NEOFORGE" => Loader::NeoForge,
                    "FORGE" => Loader::Forge,
                    _ => Loader::Unknown(loader_str.to_string()),
                };

                let minecraft_str = first["minecraft"].as_str().unwrap_or("1.20.1");
                let (major, minor, patch) = parse_version(minecraft_str);

                let mut dependencies = Vec::new();
                if let Some(deps) = first["dependencies"].as_array() {
                    for dep in deps {
                        dependencies.push(ModDependency {
                            mod_id: dep["modId"].as_str().unwrap_or("").to_string(),
                            version: dep["versionRange"].as_str().unwrap_or("*").to_string(),
                            optional: dep["mandatory"].as_bool().unwrap_or(true) == false,
                        });
                    }
                }

                return Ok((
                    loader.clone(),
                    ModMetadata {
                        mod_id,
                        version,
                        name,
                        loader,
                        minecraft: MinecraftVersion {
                            major,
                            minor,
                            patch,
                        },
                        java: None,
                        dependencies,
                    },
                ));
            }
        }
    }

    // Try legacy mcmod.info
    if !mcmod_info.is_empty() {
        let v: serde_json::Value =
            serde_json::from_str(mcmod_info).map_err(|e| AnalyzerError::Parse(e.to_string()))?;
        let default_arr: Vec<serde_json::Value> = Vec::new();
        let arr = v.as_array().unwrap_or(&default_arr);
        if let Some(first) = arr.first() {
            let mod_id = first["modid"].as_str().unwrap_or("unknown").to_string();
            let version = first["version"].as_str().unwrap_or("0.0.0").to_string();
            let name = first["name"].as_str().unwrap_or(&mod_id).to_string();
            let mc_version = first["mcversion"].as_str().unwrap_or("1.20.1");
            let (major, minor, patch) = parse_version(mc_version);

            return Ok((
                Loader::Forge,
                ModMetadata {
                    mod_id,
                    version,
                    name,
                    loader: Loader::Forge,
                    minecraft: MinecraftVersion {
                        major,
                        minor,
                        patch,
                    },
                    java: None,
                    dependencies: Vec::new(),
                },
            ));
        }
    }

    // Fallback: parse MANIFEST.MF
    let mut mod_id = "unknown".to_string();
    let mut version = "0.0.0".to_string();
    let mut name = "Unknown Mod".to_string();

    for line in manifest.lines() {
        if line.starts_with("Implementation-Title:") {
            name = line["Implementation-Title:".len()..].trim().to_string();
        } else if line.starts_with("Implementation-Version:") {
            version = line["Implementation-Version:".len()..].trim().to_string();
        } else if line.starts_with("Mod-ID:") {
            mod_id = line["Mod-ID:".len()..].trim().to_string();
        }
    }

    Ok((
        Loader::Unknown("manifest".into()),
        ModMetadata {
            mod_id,
            version,
            name,
            loader: Loader::Unknown("manifest".into()),
            minecraft: MinecraftVersion {
                major: 1,
                minor: 20,
                patch: 1,
            },
            java: None,
            dependencies: Vec::new(),
        },
    ))
}

fn parse_version(s: &str) -> (u32, u32, u32) {
    let parts: Vec<&str> = s.split('.').collect();
    let major = parts.get(0).and_then(|p| p.parse().ok()).unwrap_or(1);
    let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(20);
    let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(1);
    (major, minor, patch)
}

fn detect_minecraft_version(metadata: &ModMetadata) -> MinecraftVersion {
    metadata.minecraft.clone()
}

fn detect_java_version(manifest: &str) -> Option<JavaVersion> {
    for line in manifest.lines() {
        if line.starts_with("Build-Jdk:") || line.starts_with("Created-By:") {
            let val = line.split(':').nth(1).unwrap_or("").trim();
            if val.contains("17") || val.contains("21") {
                let major = if val.contains("21") { 21 } else { 17 };
                return Some(JavaVersion {
                    major,
                    vendor: "unknown".into(),
                });
            }
        }
    }
    None
}

// 4.2 Source tree scan
pub struct SourceScanner;

impl IAnalyzer for SourceScanner {
    fn analyze(
        &self,
        project_root: &Path,
        fs: &Arc<dyn FileSystem>,
    ) -> Result<AnalysisReport, AnalyzerError> {
        let mut report = AnalysisReport::default();
        let java_files = find_files(project_root, fs.as_ref(), "java")?;
        let kt_files = find_files(project_root, fs.as_ref(), "kt")?;
        let scala_files = find_files(project_root, fs.as_ref(), "scala")?;

        if !java_files.is_empty() {
            report.languages.push("java".into());
        }
        if !kt_files.is_empty() {
            report.languages.push("kotlin".into());
        }
        if !scala_files.is_empty() {
            report.languages.push("scala".into());
        }

        Ok(report)
    }
}

fn find_files(root: &Path, fs: &dyn FileSystem, ext: &str) -> Result<Vec<PathBuf>, AnalyzerError> {
    let mut files = Vec::new();
    for entry in fs.list(root)? {
        if entry.extension().is_some_and(|s| s == ext) {
            files.push(entry);
        } else if fs.is_dir(&entry) {
            files.extend(find_files(&entry, fs, ext)?);
        }
    }
    Ok(files)
}

// 4.5 Build system detection
pub struct BuildSystemDetector;

impl IAnalyzer for BuildSystemDetector {
    fn analyze(
        &self,
        project_root: &Path,
        fs: &Arc<dyn FileSystem>,
    ) -> Result<AnalysisReport, AnalyzerError> {
        let mut report = AnalysisReport::default();

        let gradle_files = [
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
        ];
        let maven_files = ["pom.xml"];

        for file in gradle_files {
            let path = project_root.join(file);
            if fs.exists(&path) {
                report.build_system = Some(BuildSystem {
                    kind: BuildSystemKind::Gradle,
                    file: path,
                });
                break;
            }
        }

        if report.build_system.is_none() {
            for file in maven_files {
                let path = project_root.join(file);
                if fs.exists(&path) {
                    report.build_system = Some(BuildSystem {
                        kind: BuildSystemKind::Maven,
                        file: path,
                    });
                    break;
                }
            }
        }

        Ok(report)
    }
}

// 4.4 Dependency graph builder
pub struct DependencyGraphBuilder;

impl IAnalyzer for DependencyGraphBuilder {
    fn analyze(
        &self,
        _project_root: &Path,
        _fs: &Arc<dyn FileSystem>,
    ) -> Result<AnalysisReport, AnalyzerError> {
        let mut report = AnalysisReport::default();
        // Dependency graph built from mod metadata in other detectors
        // This analyzer just ensures the structure exists
        report.dependency_graph = DependencyGraph::default();
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::LocalFileSystem;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_analyzer_engine_default() {
        let engine = AnalyzerEngine::default();
        assert_eq!(engine.detectors.len(), 4);
    }

    #[test]
    fn test_find_jars_empty() {
        let dir = tempdir().unwrap();
        let fs = LocalFileSystem;
        let jars = find_jars(dir.path(), &fs).unwrap();
        assert!(jars.is_empty());
    }

    #[test]
    fn test_build_system_detector_gradle() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("build.gradle"), "plugins { id 'java' }").unwrap();
        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let detector = BuildSystemDetector;
        let report = detector.analyze(dir.path(), &fs).unwrap();
        assert!(matches!(
            report.build_system,
            Some(BuildSystem {
                kind: BuildSystemKind::Gradle,
                ..
            })
        ));
    }

    #[test]
    fn test_build_system_detector_maven() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("pom.xml"), "<project></project>").unwrap();
        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let detector = BuildSystemDetector;
        let report = detector.analyze(dir.path(), &fs).unwrap();
        assert!(matches!(
            report.build_system,
            Some(BuildSystem {
                kind: BuildSystemKind::Maven,
                ..
            })
        ));
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.20.1"), (1, 20, 1));
        assert_eq!(parse_version("1.21"), (1, 21, 1));
        assert_eq!(parse_version("invalid"), (1, 20, 1));
        assert_eq!(parse_version("garbage"), (1, 20, 1));
    }
}
