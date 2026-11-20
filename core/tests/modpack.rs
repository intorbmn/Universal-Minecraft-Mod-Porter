//! Phase 10 — modpack import, analysis, plan and export. Fixtures are built at test time so
//! hashes are real and descriptors use real schemas.

use serde_json::json;
use sha1::{Digest, Sha1};
use std::io::Write;
use std::path::{Path, PathBuf};
use ummp_core::metadata::Loader;
use ummp_core::modpack::{
    import, import_with, EntryKind, ImportOptions, Limits, PackError, PackFormat, Support,
};

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

fn zip_bytes(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        for (name, bytes) in entries {
            z.start_file(*name, zip::write::FileOptions::default())
                .unwrap();
            z.write_all(bytes).unwrap();
        }
        z.finish().unwrap();
    }
    buf.into_inner()
}

/// A Fabric mod JAR with a real `fabric.mod.json` and a binary-looking class entry.
fn fabric_jar(id: &str, mc: &str, deps: &[(&str, &str)]) -> Vec<u8> {
    let mut depends = serde_json::Map::new();
    depends.insert("fabricloader".into(), json!(">=0.15.0"));
    depends.insert("minecraft".into(), json!(mc));
    for (k, v) in deps {
        depends.insert((*k).into(), json!(v));
    }
    let desc = json!({
        "schemaVersion": 1, "id": id, "version": "1.0.0", "name": id,
        "environment": "*", "depends": depends
    });
    let class_name = format!("com/example/{id}/Mod.class");
    let mut entries: Vec<(&str, Vec<u8>)> =
        vec![("fabric.mod.json", desc.to_string().into_bytes())];
    if let Some(c) = real_class() {
        entries.push((class_name.as_str(), c));
    }
    zip_bytes(&entries)
}

/// A real compiled class (binary `.class`), built once with `javac`; `None` without a JDK.
fn real_class() -> Option<Vec<u8>> {
    static CLASS: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    CLASS
        .get_or_init(|| {
            let dir = tempfile::tempdir().ok()?;
            let src = dir.path().join("Mod.java");
            std::fs::write(&src, "package p;\npublic class Mod { public int size() { return 4; } }\n")
                .ok()?;
            let ok = std::process::Command::new("javac")
                .arg("-d")
                .arg(dir.path())
                .arg(&src)
                .output()
                .ok()?
                .status
                .success();
            ok.then(|| std::fs::read(dir.path().join("p/Mod.class")).ok())?
        })
        .clone()
}

fn forge_jar(id: &str, mc: &str) -> Vec<u8> {
    let toml = format!(
        "modLoader=\"javafml\"\nloaderVersion=\"[47,)\"\nlicense=\"MIT\"\n\
         [[mods]]\nmodId=\"{id}\"\nversion=\"1.0.0\"\ndisplayName=\"{id}\"\n\
         [[dependencies.{id}]]\nmodId=\"minecraft\"\nmandatory=true\nversionRange=\"[{mc}]\"\n\
         ordering=\"NONE\"\nside=\"BOTH\"\n"
    );
    zip_bytes(&[("META-INF/mods.toml", toml.into_bytes())])
}

fn sha1_hex(bytes: &[u8]) -> String {
    Sha1::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn hex(len: usize) -> String {
    "ab".repeat(len / 2)
}

fn index(files: serde_json::Value) -> Vec<u8> {
    json!({
        "formatVersion": 1, "game": "minecraft", "versionId": "1.0.0", "name": "Demo Pack",
        "summary": "synthetic",
        "files": files,
        "dependencies": { "minecraft": "1.20.1", "fabric-loader": "0.15.11" }
    })
    .to_string()
    .into_bytes()
}

fn remote_file(path: &str) -> serde_json::Value {
    json!({
        "path": path,
        "hashes": { "sha1": hex(40), "sha512": hex(128) },
        "env": { "client": "required", "server": "optional" },
        "downloads": ["https://cdn.modrinth.com/data/demo/versions/v1/demowidget-1.0.0.jar"],
        "fileSize": 12345
    })
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, bytes).unwrap();
    p
}

fn all_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(all_files(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 10.A Import
// ---------------------------------------------------------------------------

#[test]
fn imports_mrpack_files_and_overrides() {
    let tmp = tempfile::tempdir().unwrap();
    let extra = fabric_jar("extra", "1.20.1", &[]);
    let pack = write(
        tmp.path(),
        "demo.mrpack",
        &zip_bytes(&[
            (
                "modrinth.index.json",
                index(json!([remote_file("mods/demowidget-1.0.0.jar")])),
            ),
            (
                "overrides/config/demowidget.toml",
                b"size = 4\n".to_vec(),
            ),
            ("overrides/mods/extra.jar", extra.clone()),
        ]),
    );
    let work = tmp.path().join("work");
    let p = import(&pack, &work).unwrap();

    assert_eq!(p.format, PackFormat::Mrpack);
    assert_eq!(p.name, "Demo Pack");
    assert_eq!(p.mc.as_deref(), Some("1.20.1"));
    assert_eq!(p.loader, Some(Loader::Fabric));
    assert_eq!(p.loader_version.as_deref(), Some("0.15.11"));

    assert_eq!(p.entries.len(), 2);
    let remote = &p.entries[0];
    assert_eq!(remote.path, "mods/demowidget-1.0.0.jar");
    assert_eq!(remote.kind, EntryKind::Mod);
    assert!(remote.local.is_none());
    assert_eq!(remote.env.client, Support::Required);
    assert_eq!(remote.env.server, Support::Optional);
    assert_eq!(remote.downloads.len(), 1);
    assert_eq!(remote.sha1.as_deref(), Some(hex(40).as_str()));
    assert_eq!(remote.size, Some(12345));

    let local = &p.entries[1];
    assert_eq!(local.path, "mods/extra.jar");
    assert_eq!(local.sha1.as_deref(), Some(sha1_hex(&extra).as_str()));
    assert!(local.local.as_ref().unwrap().exists());

    assert!(p.overrides.contains(&PathBuf::from("config/demowidget.toml")));
    assert_eq!(
        std::fs::read_to_string(p.root.join("config/demowidget.toml")).unwrap(),
        "size = 4\n"
    );
}

#[test]
fn rejects_mrpack_with_path_traversal() {
    for bad in ["../evil.jar", "/abs.jar", "C:\\x.jar", "mods/../../evil.jar"] {
        let tmp = tempfile::tempdir().unwrap();
        let pack = write(
            tmp.path(),
            "bad.mrpack",
            &zip_bytes(&[("modrinth.index.json", index(json!([remote_file(bad)])))]),
        );
        let work = tmp.path().join("work");
        let err = import(&pack, &work).unwrap_err();
        assert!(matches!(err, PackError::UnsafePath(_)), "{bad}: {err:?}");
        // Nothing outside the work dir was created.
        let outside: Vec<_> = all_files(tmp.path())
            .into_iter()
            .filter(|f| !f.starts_with(&work) && f != &pack)
            .collect();
        assert!(outside.is_empty(), "{bad}: {outside:?}");
    }
}

#[test]
fn rejects_zip_slip_in_overrides() {
    let tmp = tempfile::tempdir().unwrap();
    let pack = write(
        tmp.path(),
        "slip.mrpack",
        &zip_bytes(&[
            ("modrinth.index.json", index(json!([]))),
            ("overrides/../../evil.txt", b"x".to_vec()),
        ]),
    );
    let work = tmp.path().join("a").join("work");
    let err = import(&pack, &work).unwrap_err();
    assert!(matches!(err, PackError::UnsafePath(_)), "{err:?}");
    assert!(!tmp.path().join("evil.txt").exists());
    assert!(!tmp.path().join("a").join("evil.txt").exists());
}

#[test]
fn rejects_mrpack_hash_mismatch() {
    // The zip bundles the file at its listed path, but the bytes do not match the hash.
    let tmp = tempfile::tempdir().unwrap();
    let pack = write(
        tmp.path(),
        "h.mrpack",
        &zip_bytes(&[
            (
                "modrinth.index.json",
                index(json!([remote_file("mods/x.jar")])),
            ),
            ("mods/x.jar", b"not the declared bytes".to_vec()),
        ]),
    );
    let err = import(&pack, &tmp.path().join("work")).unwrap_err();
    assert!(matches!(err, PackError::HashMismatch { .. }), "{err:?}");

    // A malformed hash is refused even when nothing is bundled.
    let mut f = remote_file("mods/y.jar");
    f["hashes"]["sha1"] = json!("abc");
    let pack = write(
        tmp.path(),
        "m.mrpack",
        &zip_bytes(&[("modrinth.index.json", index(json!([f])))]),
    );
    let err = import(&pack, &tmp.path().join("work2")).unwrap_err();
    assert!(matches!(err, PackError::Invalid(_)), "{err:?}");
}

#[test]
fn caps_archive_entry_size() {
    let tmp = tempfile::tempdir().unwrap();
    let pack = write(
        tmp.path(),
        "big.mrpack",
        &zip_bytes(&[
            ("modrinth.index.json", index(json!([]))),
            ("overrides/big.bin", vec![0u8; 2 * 1024 * 1024]),
        ]),
    );
    let opts = ImportOptions {
        limits: Limits {
            max_entry_bytes: 1024 * 1024,
            ..Limits::default()
        },
        ..ImportOptions::default()
    };
    let err = import_with(&pack, &tmp.path().join("work"), &opts).unwrap_err();
    assert!(matches!(err, PackError::LimitExceeded(_)), "{err:?}");
    // The default limits accept the same pack.
    assert!(import(&pack, &tmp.path().join("work2")).is_ok());
}

#[test]
fn imports_curseforge_manifest_as_unresolved() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = json!({
        "minecraft": { "version": "1.20.1", "modLoaders": [{ "id": "forge-47.2.0", "primary": true }] },
        "manifestType": "minecraftModpack", "manifestVersion": 1, "name": "Demo Pack",
        "version": "1.0.0", "author": "demo",
        "files": [{ "projectID": 123, "fileID": 456, "required": true }],
        "overrides": "overrides"
    });
    let pack = write(
        tmp.path(),
        "demo-curseforge.zip",
        &zip_bytes(&[
            ("manifest.json", manifest.to_string().into_bytes()),
            ("overrides/config/demo.cfg", b"S:size=4\n".to_vec()),
        ]),
    );
    let p = import(&pack, &tmp.path().join("work")).unwrap();
    assert_eq!(p.format, PackFormat::CurseForge);
    assert_eq!(p.name, "Demo Pack");
    assert_eq!(p.mc.as_deref(), Some("1.20.1"));
    assert_eq!(p.loader, Some(Loader::Forge));
    assert_eq!(p.loader_version.as_deref(), Some("47.2.0"));
    assert_eq!(p.entries.len(), 1);
    let e = &p.entries[0];
    assert_eq!(e.curseforge, Some((123, 456)));
    assert!(e.local.is_none() && e.sha1.is_none() && e.downloads.is_empty());
    assert!(p.overrides.contains(&PathBuf::from("config/demo.cfg")));
}

#[test]
fn imports_prism_instance_components() {
    let tmp = tempfile::tempdir().unwrap();
    let inst = tmp.path().join("demo-prism");
    write(
        &inst,
        "mmc-pack.json",
        json!({ "formatVersion": 1, "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.15.11" } ] })
        .to_string()
        .as_bytes(),
    );
    write(&inst, "instance.cfg", b"[General]\nname=Demo Instance\n");
    write(
        &inst,
        "minecraft/mods/a.jar",
        &fabric_jar("a", "1.20.1", &[]),
    );
    write(&inst, "minecraft/config/a.toml", b"x = 1\n");
    let p = import(&inst, &tmp.path().join("work")).unwrap();
    assert_eq!(p.format, PackFormat::Prism);
    assert_eq!(p.name, "Demo Instance");
    assert_eq!(p.mc.as_deref(), Some("1.20.1"));
    assert_eq!(p.loader, Some(Loader::Fabric));
    assert_eq!(p.loader_version.as_deref(), Some("0.15.11"));
    assert_eq!(p.entries.len(), 1);
    assert_eq!(p.entries[0].path, "mods/a.jar");
    assert!(p.entries[0].local.is_some());
    assert!(p.overrides.contains(&PathBuf::from("config/a.toml")));
}

#[test]
fn imports_plain_folder_majority_loader() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("pack");
    write(&dir, "mods/a.jar", &fabric_jar("a", "1.20.1", &[]));
    write(&dir, "mods/b.jar", &fabric_jar("b", "1.20.1", &[]));
    write(&dir, "mods/c.jar", &forge_jar("c", "1.20.1"));
    write(&dir, "config/a.toml", b"x = 1\n");
    write(&dir, "kubejs/server_scripts/x.js", b"// script\n");

    let p = import(&dir, &tmp.path().join("work")).unwrap();
    assert_eq!(p.format, PackFormat::Folder);
    assert_eq!(p.loader, Some(Loader::Fabric));
    assert_eq!(p.mc.as_deref(), Some("1.20.1"));
    assert_eq!(p.entries.len(), 3);
    assert!(p.overrides.contains(&PathBuf::from("kubejs/server_scripts/x.js")));

    // --from-loader / --from-mc win over detection.
    let opts = ImportOptions {
        from_loader: Some(Loader::Forge),
        from_mc: Some("1.19.2".into()),
        ..ImportOptions::default()
    };
    let p = import_with(&dir, &tmp.path().join("work2"), &opts).unwrap();
    assert_eq!(p.loader, Some(Loader::Forge));
    assert_eq!(p.mc.as_deref(), Some("1.19.2"));
}

// ---------------------------------------------------------------------------
// 10.B Analysis and graph
// ---------------------------------------------------------------------------

use ummp_core::modpack::analyze::{analyze_pack, gate, AnalyzeOptions};
use ummp_core::modpack::graph::{PackGraph, PackIssue};
use ummp_core::modpack::Pack;
use ummp_core::scan::Severity;

/// A plain-folder pack holding the given JARs under `mods/`.
fn folder_pack(tmp: &Path, jars: &[(&str, Vec<u8>)]) -> Pack {
    let dir = tmp.join("pack");
    for (name, bytes) in jars {
        write(&dir, &format!("mods/{name}"), bytes);
    }
    import(&dir, &tmp.join("work")).unwrap()
}

fn analyzed(tmp: &Path, jars: &[(&str, Vec<u8>)]) -> Pack {
    let mut p = folder_pack(tmp, jars);
    analyze_pack(&mut p, &AnalyzeOptions { threads: 2 }).unwrap();
    p
}

#[test]
fn graph_reports_missing_required_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let p = analyzed(
        tmp.path(),
        &[("a.jar", fabric_jar("a", "1.20.1", &[("libb", ">=1.0")]))],
    );
    let g = PackGraph::build(&p.entries);
    assert!(
        g.issues.iter().any(|i| matches!(i,
            PackIssue::MissingDependency { mod_id, requires } if mod_id == "a" && requires == "libb")),
        "{:?}",
        g.issues
    );
    // The built-ins (minecraft, fabricloader) are never reported.
    assert!(!g.issues.iter().any(|i| matches!(i,
        PackIssue::MissingDependency { requires, .. } if requires == "minecraft" || requires == "fabricloader")));

    // With the dependency present, nothing is missing.
    let tmp2 = tempfile::tempdir().unwrap();
    let p = analyzed(
        tmp2.path(),
        &[
            ("a.jar", fabric_jar("a", "1.20.1", &[("libb", ">=1.0")])),
            ("libb.jar", fabric_jar("libb", "1.20.1", &[])),
        ],
    );
    assert!(PackGraph::build(&p.entries).issues.is_empty());
}

#[test]
fn graph_reports_duplicate_mod_id() {
    let tmp = tempfile::tempdir().unwrap();
    let p = analyzed(
        tmp.path(),
        &[
            ("dup-1.jar", fabric_jar("dup", "1.20.1", &[])),
            ("dup-2.jar", fabric_jar("dup", "1.20.1", &[])),
        ],
    );
    let g = PackGraph::build(&p.entries);
    assert!(g.issues.iter().any(|i| matches!(i,
        PackIssue::DuplicateId { mod_id, paths } if mod_id == "dup" && paths.len() == 2)));
}

#[test]
fn graph_reports_cycle_without_hanging() {
    let tmp = tempfile::tempdir().unwrap();
    let p = analyzed(
        tmp.path(),
        &[
            ("a.jar", fabric_jar("a", "1.20.1", &[("b", "*")])),
            ("b.jar", fabric_jar("b", "1.20.1", &[("a", "*")])),
            ("c.jar", fabric_jar("c", "1.20.1", &[("a", "*")])),
        ],
    );
    let g = PackGraph::build(&p.entries);
    let cycle = g
        .issues
        .iter()
        .find_map(|i| match i {
            PackIssue::Cycle { mods } => Some(mods.clone()),
            _ => None,
        })
        .expect("a cycle");
    assert_eq!(cycle, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn graph_reports_side_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let mut p = analyzed(
        tmp.path(),
        &[
            ("a.jar", fabric_jar("a", "1.20.1", &[("lib", "*")])),
            ("lib.jar", fabric_jar("lib", "1.20.1", &[])),
        ],
    );
    for e in &mut p.entries {
        if e.path == "mods/lib.jar" {
            e.env.server = Support::Unsupported; // client-only library
        }
    }
    let g = PackGraph::build(&p.entries);
    assert!(g.issues.iter().any(|i| matches!(i,
        PackIssue::SideMismatch { mod_id, requires, side }
            if mod_id == "a" && requires == "lib" && *side == "server")), "{:?}", g.issues);
}

#[test]
fn pack_with_high_risk_jar_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let evil = zip_bytes(&[
        (
            "fabric.mod.json",
            br#"{"schemaVersion":1,"id":"evil","version":"1.0.0","depends":{"minecraft":"1.20.1"}}"#
                .to_vec(),
        ),
        (
            "evil.properties",
            b"hook=https://discord.com/api/webhooks/1/abc\n".to_vec(),
        ),
    ]);
    let p = analyzed(
        tmp.path(),
        &[
            ("good.jar", fabric_jar("good", "1.20.1", &[])),
            ("evil.jar", evil),
        ],
    );
    let evil = p.entries.iter().find(|e| e.path == "mods/evil.jar").unwrap();
    assert_eq!(evil.risk, Some(Severity::High));
    let good = p.entries.iter().find(|e| e.path == "mods/good.jar").unwrap();
    assert_eq!(good.risk, None);

    let err = gate(&p, false).unwrap_err();
    assert!(matches!(err, PackError::Refused(ref m) if m.contains("evil.jar")), "{err:?}");
    assert!(gate(&p, true).is_ok());
}

fn many_jars(n: usize) -> Vec<(String, Vec<u8>)> {
    (0..n)
        .map(|i| {
            let deps: Vec<(String, String)> = if i > 0 {
                vec![(format!("m{}", i - 1), "*".into())]
            } else {
                vec![]
            };
            let deps_ref: Vec<(&str, &str)> =
                deps.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
            (
                format!("m{i:03}.jar"),
                fabric_jar(&format!("m{i}"), "1.20.1", &deps_ref),
            )
        })
        .collect()
}

#[test]
fn parallel_analysis_matches_sequential() {
    let jars = many_jars(24);
    let refs: Vec<(&str, Vec<u8>)> = jars.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let tmp = tempfile::tempdir().unwrap();
    let mut one = folder_pack(tmp.path(), &refs);
    let mut many = import(&tmp.path().join("pack"), &tmp.path().join("w2")).unwrap();
    analyze_pack(&mut one, &AnalyzeOptions { threads: 1 }).unwrap();
    analyze_pack(&mut many, &AnalyzeOptions { threads: 8 }).unwrap();
    assert_eq!(
        serde_json::to_string(&one.entries).unwrap(),
        serde_json::to_string(&many.entries).unwrap()
    );
    assert!(one.entries.iter().all(|e| e.mod_info.is_some()));
}

#[test]
fn analysis_is_deterministic() {
    let jars = many_jars(12);
    let refs: Vec<(&str, Vec<u8>)> = jars.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let tmp = tempfile::tempdir().unwrap();
    let mut a = folder_pack(tmp.path(), &refs);
    let mut b = import(&tmp.path().join("pack"), &tmp.path().join("w2")).unwrap();
    analyze_pack(&mut a, &AnalyzeOptions { threads: 4 }).unwrap();
    analyze_pack(&mut b, &AnalyzeOptions { threads: 4 }).unwrap();
    let ga = PackGraph::build(&a.entries);
    let gb = PackGraph::build(&b.entries);
    assert_eq!(
        serde_json::to_string(&a.entries).unwrap(),
        serde_json::to_string(&b.entries).unwrap()
    );
    assert_eq!(format!("{:?}", ga.issues), format!("{:?}", gb.issues));
}
