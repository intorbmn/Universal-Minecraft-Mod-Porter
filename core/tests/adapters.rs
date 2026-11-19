//! Phase 9 adapters: rule-based technology detection and file create/delete in edit sets.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use ummp_core::adapter::{AdapterRegistry, Handling};
use ummp_core::checkpoint::CheckpointStore;
use ummp_core::migration::{EditSet, FileEdit, MigrationEngine, RunOptions};
use ummp_core::project::{DefaultProjectManager, ProjectManager};
use uuid::Uuid;

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

fn read_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    ummp_core::checkpoint::collect_files(root)
        .unwrap()
        .into_iter()
        .map(|r| (r.clone(), std::fs::read(root.join(&r)).unwrap()))
        .collect()
}

#[test]
fn rules_detect_each_technology() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    let j = "src/main/java/com/example/demo";
    write(r, "src/main/kotlin/Demo.kt", "class Demo\n");
    write(r, "src/main/scala/Demo.scala", "object Demo\n");
    write(r, "build-logic/Demo.groovy", "class Demo {}\n");
    write(r, "native/CMakeLists.txt", "project(demo)\n");
    write(r, "native-rs/Cargo.toml", "[package]\nname = \"demo\"\n");
    write(
        r,
        &format!("{j}/DemoNative.java"),
        "class DemoNative {\n  // x\n  private native int size();\n}\n",
    );
    write(
        r,
        "src/main/resources/assets/demo/shaders/core/demo.fsh",
        "#version 150\n",
    );
    write(
        r,
        &format!("{j}/DemoRender.java"),
        "import com.mojang.blaze3d.vertex.PoseStack;\nclass DemoRender {}\n",
    );
    write(
        r,
        &format!("{j}/DemoPacket.java"),
        "class DemoPacket implements CustomPacketPayload {}\n",
    );
    write(
        r,
        &format!("{j}/DemoLoader.java"),
        "class DemoLoader extends ClassLoader {}\n",
    );
    write(
        r,
        &format!("{j}/mixin/DemoMixin.java"),
        "class DemoMixin {\n  @Redirect(method = \"x\")\n  void y() {}\n}\n",
    );
    write(
        r,
        "src/main/resources/demo.accesswidener",
        "accessWidener v2 named\n",
    );
    write(
        r,
        "src/main/resources/META-INF/accesstransformer.cfg",
        "public com.example.demo.DemoWidget\n",
    );
    write(r, "src/main/resources/META-INF/coremods.json", "{}\n");
    write(
        r,
        "common/build.gradle",
        "dependencies { modImplementation \"dev.architectury:architectury:1\" }\n",
    );
    write(r, &format!("{j}/Plain.java"), "class Plain {}\n");

    let found = AdapterRegistry::default().scan_dir(r);
    let by_tech: BTreeMap<&str, _> = found.iter().map(|f| (f.tech.as_str(), f)).collect();
    for tech in [
        "kotlin",
        "scala",
        "groovy",
        "c-cpp",
        "rust-native",
        "jni-jna",
        "shader",
        "rendering",
        "networking",
        "classloader",
        "mixin-advanced",
        "access-widener",
        "access-transformer",
        "coremod",
        "multi-loader",
    ] {
        assert!(by_tech.contains_key(tech), "missing {tech}: {found:#?}");
    }
    assert_eq!(found.iter().filter(|f| f.tech == "jni-jna").count(), 1);
    let jni = by_tech["jni-jna"];
    assert_eq!(jni.file, format!("{j}/DemoNative.java"));
    assert_eq!(jni.line, Some(3));
    assert_eq!(jni.handling, Handling::Manual);
    assert_eq!(by_tech["mixin-advanced"].line, Some(2));
    assert_eq!(by_tech["kotlin"].handling, Handling::Auto);
}

#[test]
fn plain_java_project_has_no_findings() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "src/main/java/com/example/demo/DemoWidget.java",
        "class DemoWidget {}\n",
    );
    assert!(AdapterRegistry::default().scan_dir(d.path()).is_empty());
}

#[test]
fn delete_and_create_apply_and_rollback() {
    let d = tempfile::tempdir().unwrap();
    let snaps = tempfile::tempdir().unwrap();
    write(d.path(), "fabric.mod.json", "{\"id\":\"demowidget\"}\n");
    write(d.path(), "src/Keep.java", "class Keep {}\n");
    let before = read_tree(d.path());
    let project = DefaultProjectManager.open(d.path()).unwrap();
    let set = EditSet {
        edits: vec![FileEdit {
            path: PathBuf::from("META-INF/neoforge.mods.toml"),
            original: String::new(),
            updated: "modLoader = \"javafml\"\n".into(),
            replacements: vec![],
        }],
        deletes: vec![PathBuf::from("fabric.mod.json")],
        ..EditSet::default()
    };
    let mut store = CheckpointStore::new();
    let id = MigrationEngine::default()
        .apply_edit_set(
            &set,
            &project,
            &mut store,
            &RunOptions {
                dry_run: false,
                snapshot_root: snaps.path(),
                session_id: Uuid::new_v4(),
            },
        )
        .unwrap()
        .expect("checkpoint");
    assert!(d.path().join("META-INF/neoforge.mods.toml").exists());
    assert!(!d.path().join("fabric.mod.json").exists());
    store.restore(id, d.path()).unwrap();
    assert_eq!(read_tree(d.path()), before);
}
