//! ★ The template plugin in `templates/plugin/` is built here with cargo, loaded exactly as a
//! user would install it (`ummp plugins check`), and its edit is applied by `MigrationEngine`
//! after a checkpoint, then rolled back.
use std::path::{Path, PathBuf};
use std::process::Command;
use ummp_core::adapter::{AdapterContext, AdapterRegistry};
use ummp_core::checkpoint::CheckpointStore;
use ummp_core::descriptor::DescriptorTarget;
use ummp_core::migration::{MigrationEngine, MigrationPlan, MigrationRequest, RunOptions};
use ummp_core::plugins::PluginManager;
use ummp_core::project::Project;
use ummp_core::resources::PackFormats;
use uuid::Uuid;

/// Build the template and return the path of its executable (cargo reports it as JSON).
fn build_template() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .current_dir(&root)
        .args([
            "build",
            "--package",
            "ummp-plugin-template",
            "--message-format=json",
        ])
        .output()
        .expect("cargo must be available");
    assert!(
        output.status.success(),
        "template build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["reason"] == "compiler-artifact" && v["target"]["name"] == "demo-renamer")
        .find_map(|v| v["executable"].as_str().map(PathBuf::from))
        .expect("cargo reported no template executable")
}

fn install_template(plugins: &Path) -> PathBuf {
    let exe = build_template();
    let dir = plugins.join("demo-renamer");
    std::fs::create_dir_all(&dir).unwrap();
    let name = if cfg!(windows) {
        "demo-renamer.exe"
    } else {
        "demo-renamer"
    };
    std::fs::copy(exe, dir.join(name)).unwrap();
    let manifest =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates/plugin/ummp-plugin.toml");
    let text = std::fs::read_to_string(manifest).unwrap();
    // The shipped manifest uses the Unix executable name; adapt it only on Windows.
    std::fs::write(
        dir.join("ummp-plugin.toml"),
        text.replace("[\"demo-renamer\"]", &format!("[\"{name}\"]")),
    )
    .unwrap();
    dir
}

fn plan() -> MigrationPlan {
    MigrationPlan {
        request: MigrationRequest {
            source_minecraft: "1.20.1".into(),
            target_minecraft: "1.21.1".into(),
            target_loader: "fabric".into(),
            min_confidence: 0.8,
            source_loader: Some("fabric".into()),
        },
        stages: Vec::new(),
        skipped: Vec::new(),
        breaking_changes: Vec::new(),
        recipes: Vec::new(),
        descriptor_target: DescriptorTarget {
            minecraft: "1.21.1".into(),
            loader: "fabric".into(),
            loader_version: None,
        },
        java: None,
        pack_formats: PackFormats::default(),
        datapack_renames: false,
    }
}

#[test]
fn template_plugin_passes_check_and_its_edit_is_applied_by_the_engine() {
    let plugins = tempfile::tempdir().unwrap();
    let dir = install_template(plugins.path());

    let manager = PluginManager::new(plugins.path());
    let info = manager.check(&dir).expect("plugins check must pass");
    assert_eq!(info.name, "demo-renamer");

    let project_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(project_dir.path().join("src")).unwrap();
    std::fs::write(
        project_dir.path().join("src/Main.java"),
        "class DemoWidget {}",
    )
    .unwrap();
    let project = Project::new(project_dir.path().canonicalize().unwrap());
    let files = vec![PathBuf::from("src/Main.java")];
    let plan = plan();
    let cx = AdapterContext {
        root: &project.root,
        files: &files,
        plan: Some(&plan),
        source_loader: Some("fabric".into()),
    };

    let mut adapters = manager.adapters().unwrap();
    assert_eq!(adapters.len(), 1);
    let plugin = adapters.remove(0);
    let set = plugin.edits(&cx).unwrap();
    assert_eq!(set.edits.len(), 1);

    let engine = MigrationEngine::default().with_adapters(AdapterRegistry::new(vec![plugin]));
    let snapshots = tempfile::tempdir().unwrap();
    let mut store = CheckpointStore::new();
    let checkpoint = engine
        .apply_edit_set(
            &set,
            &project,
            &mut store,
            &RunOptions {
                dry_run: false,
                snapshot_root: snapshots.path(),
                session_id: Uuid::new_v4(),
            },
        )
        .unwrap()
        .unwrap();
    let file = project.root.join("src/Main.java");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "class DemoGadget {}"
    );
    store.restore(checkpoint, &project.root).unwrap();
    assert_eq!(
        std::fs::read_to_string(file).unwrap(),
        "class DemoWidget {}"
    );
}
