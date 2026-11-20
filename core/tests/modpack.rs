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
    zip_bytes(&[
        ("fabric.mod.json", desc.to_string().into_bytes()),
        (
            &format!("com/example/{id}/Mod.class"),
            vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65, 0xFF, 0x00],
        ),
    ])
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
