//! Template plugin: renames the synthetic type `DemoWidget` to `DemoGadget`.
//!
//! Copy this folder, change `FROM`/`TO` and the rules below, and keep the manifest `name`
//! equal to `Renamer::id`. UMMP checks every path you return, requires `original` to match
//! the file on disk, and applies edits only through its checkpointed engine.
use std::process::ExitCode;
use ummp_plugin_sdk::{Edit, Finding, Handling, PluginAdapter, ProjectFile, Target};

const FROM: &str = "DemoWidget";
const TO: &str = "DemoGadget";

struct Renamer;

impl PluginAdapter for Renamer {
    fn id(&self) -> &'static str {
        "demo-renamer"
    }

    fn findings(&self, files: &[ProjectFile]) -> Vec<Finding> {
        files
            .iter()
            .filter_map(|file| {
                let line = file.content.lines().position(|l| l.contains(FROM))?;
                let message = format!("{FROM} should become {TO}");
                let finding = Finding::new(self.id(), Handling::Auto, &file.path, &message);
                Some(finding.at_line(u32::try_from(line + 1).ok()?))
            })
            .collect()
    }

    fn edits(&self, files: &[ProjectFile], _target: &Target) -> Vec<Edit> {
        files
            .iter()
            .filter_map(|file| Edit::replace_all(file, FROM, TO))
            .collect()
    }
}

fn main() -> ExitCode {
    ummp_plugin_sdk::run(&Renamer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(content: &str) -> ProjectFile {
        ProjectFile {
            path: "src/Demo.java".into(),
            content: content.into(),
        }
    }

    #[test]
    fn finding_reports_the_one_based_line() {
        let found = Renamer.findings(&[file("class A {}\nclass DemoWidget {}")]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, Some(2));
    }

    #[test]
    fn edit_renames_every_occurrence_and_keeps_original() {
        let target = Target {
            minecraft: "1.21.1".into(),
            loader: "fabric".into(),
        };
        let edits = Renamer.edits(&[file("DemoWidget a; DemoWidget b;")], &target);
        assert_eq!(edits[0].original, "DemoWidget a; DemoWidget b;");
        assert_eq!(edits[0].updated, "DemoGadget a; DemoGadget b;");
    }

    #[test]
    fn untouched_files_produce_no_edit() {
        let target = Target {
            minecraft: "1.21.1".into(),
            loader: "fabric".into(),
        };
        assert!(Renamer.edits(&[file("class Other {}")], &target).is_empty());
    }
}
