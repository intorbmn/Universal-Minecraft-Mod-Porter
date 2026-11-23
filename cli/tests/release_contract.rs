//! cargo-dist contract. PR CI only runs `dist plan`, which does not compile anything, so a
//! missing `[profile.dist]` passed CI and would only fail on the first real tag build.

use std::path::Path;

/// Returns the lines of one `[section]` of the workspace manifest, up to the next header.
fn manifest_section(header: &str) -> Option<Vec<String>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    let text = std::fs::read_to_string(&path).expect("read workspace Cargo.toml");
    let mut lines = text.lines().map(str::trim);
    lines.find(|line| *line == header)?;
    Some(
        lines
            .take_while(|line| !line.starts_with('['))
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_owned)
            .collect(),
    )
}

#[test]
fn workspace_defines_the_dist_profile_cargo_dist_builds_with() {
    let dist = manifest_section("[profile.dist]")
        .expect("[profile.dist] is required by `dist build` (cargo-dist)");
    assert!(
        dist.iter()
            .any(|line| line.replace(' ', "") == "inherits=\"release\""),
        "[profile.dist] must inherit the release profile, got {dist:?}"
    );
}
