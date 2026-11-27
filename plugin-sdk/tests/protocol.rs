//! The golden files in `tests/fixtures/golden/plugin-protocol/` are the contract shared with
//! `core/src/plugins.rs`. If a field is renamed on either side, one of the two suites fails.
use serde_json::Value;
use std::path::PathBuf;
use ummp_plugin_sdk::{
    handle, Edit, Finding, Handling, PluginAdapter, ProjectFile, Request, Target, API,
};

fn golden(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/golden/plugin-protocol")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn golden_value(name: &str) -> Value {
    serde_json::from_str(&golden(name)).unwrap()
}

struct Demo;
impl PluginAdapter for Demo {
    fn id(&self) -> &'static str {
        "demo-renamer"
    }
    fn findings(&self, files: &[ProjectFile]) -> Vec<Finding> {
        files
            .iter()
            .map(|f| {
                Finding::new(
                    "demo-renamer",
                    Handling::Assisted,
                    &f.path,
                    "Demo type found",
                )
                .at_line(1)
            })
            .collect()
    }
    fn edits(&self, files: &[ProjectFile], _target: &Target) -> Vec<Edit> {
        files
            .iter()
            .filter_map(|f| Edit::replace_all(f, "DemoWidget", "DemoGadget"))
            .collect()
    }
}

#[test]
fn core_requests_parse_into_sdk_types() {
    assert_eq!(
        serde_json::from_str::<Request>(&golden("hello.request.json")).unwrap(),
        Request::Hello
    );
    let Request::Findings { files } =
        serde_json::from_str(&golden("findings.request.json")).unwrap()
    else {
        panic!("expected findings request");
    };
    assert_eq!(files[0].path, "src/main/java/Demo.java");
    let Request::Edits { files, target } =
        serde_json::from_str(&golden("edits.request.json")).unwrap()
    else {
        panic!("expected edits request");
    };
    assert_eq!(files[0].content, "class DemoWidget {}");
    assert_eq!(target.minecraft, "1.21.1");
    assert_eq!(target.loader, "fabric");
}

#[test]
fn sdk_responses_equal_golden_core_responses() {
    for (request, response) in [
        ("hello.request.json", "hello.response.json"),
        ("edits.request.json", "edits.response.json"),
    ] {
        let out = handle(&Demo, &golden(request)).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&out).unwrap(),
            golden_value(response)
        );
    }
}

#[test]
fn finding_with_and_without_line_matches_golden() {
    let with_line = Finding::new(
        "demo-renamer",
        Handling::Assisted,
        "src/main/java/Demo.java",
        "Demo type found",
    )
    .at_line(1);
    let without = Finding::new(
        "demo-renamer",
        Handling::Manual,
        "src/main/java/Demo.java",
        "Whole-file note",
    );
    let value = serde_json::to_value(vec![with_line, without]).unwrap();
    assert_eq!(value, golden_value("findings.response.json"));
}

#[test]
fn response_is_exactly_one_line() {
    let out = handle(&Demo, &golden("findings.request.json")).unwrap();
    assert!(!out.contains('\n'));
}

#[test]
fn unknown_method_is_an_error_not_a_panic() {
    assert!(handle(&Demo, r#"{"method":"format-disk"}"#).is_err());
    assert!(handle(&Demo, "not json").is_err());
}

#[test]
fn api_constant_matches_protocol_document() {
    assert_eq!(API, 1);
}

#[test]
fn replace_all_returns_none_when_nothing_changes() {
    let file = ProjectFile {
        path: "a.txt".into(),
        content: "nothing here".into(),
    };
    assert!(Edit::replace_all(&file, "DemoWidget", "DemoGadget").is_none());
}
