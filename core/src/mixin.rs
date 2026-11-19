//! Application: Mixin migration (Phase 6.E).
//!
//! - `*.mixins.json`: raises `compatibilityLevel` (`JAVA_N`) to the target's
//!   Java requirement.
//! - Mixin classes (source containing `@Mixin`): class renames are applied to
//!   the string forms Mixin uses and the source transformer skips: JVM
//!   descriptors (`Lnet/minecraft/Foo;`), internal names (`net/minecraft/Foo`)
//!   and dotted names in `targets = "..."`. Only qualified rules apply.
//! - Injector `method = "..."` targets need method mappings, which the KB does
//!   not hold yet: they are counted and reported for manual review.

use crate::descriptor::{apply_edits, find_key, string_value, EditResult, Edits};
use crate::migration::RenameRule;
use std::path::Path;

pub fn is_mixin_config(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".mixins.json") || n == "mixins.json")
}

pub fn migrate_mixin_config(content: &str, java: Option<u32>) -> EditResult {
    let mut result = EditResult {
        updated: content.to_string(),
        ..EditResult::default()
    };
    let (Some(java), Some(root)) = (java, content.find('{')) else {
        return result;
    };
    let Some(r) =
        find_key(content, root, "compatibilityLevel").and_then(|v| string_value(content, v))
    else {
        return result;
    };
    let old = &content[r.clone()];
    let current: Option<u32> = old.strip_prefix("JAVA_").and_then(|n| n.parse().ok());
    if current.is_some_and(|c| c < java) {
        let edits: Edits = vec![(r, old.to_string(), format!("JAVA_{java}"))];
        let valid = |t: &str| serde_json::from_str::<serde_json::Value>(t).is_ok();
        if let Some((updated, replacements)) = apply_edits(content, edits, valid) {
            result.updated = updated;
            result.replacements = replacements;
        }
    }
    result
}

pub fn is_mixin_class(content: &str) -> bool {
    content.contains("@Mixin")
}

/// Apply one stage of qualified rules to the Mixin string forms in `content`.
pub fn migrate_mixin_strings(content: &str, rules: &[RenameRule]) -> EditResult {
    let mut result = EditResult {
        updated: content.to_string(),
        ..EditResult::default()
    };
    let mut found: Edits = Vec::new();
    for rule in rules.iter().filter(|r| r.is_qualified()) {
        let (from_int, to_int) = (rule.from.replace('.', "/"), rule.to.replace('.', "/"));
        let patterns = [
            (format!("L{from_int};"), format!("L{to_int};")),
            (format!("\"{from_int}\""), format!("\"{to_int}\"")),
            (format!("\"{}\"", rule.from), format!("\"{}\"", rule.to)),
        ];
        for (pat, new) in patterns {
            for (pos, _) in content.match_indices(&pat) {
                found.push((pos..pos + pat.len(), pat.clone(), new.clone()));
            }
        }
    }
    // Drop overlapping matches (keep the earliest).
    found.sort_by_key(|(r, _, _)| r.start);
    let mut edits: Edits = Vec::new();
    for e in found {
        if edits
            .last()
            .is_none_or(|(last, _, _)| e.0.start >= last.end)
        {
            edits.push(e);
        }
    }
    if let Some((updated, replacements)) = apply_edits(content, edits, |_| true) {
        result.updated = updated;
        result.replacements = replacements;
    }
    result
}

/// Number of injector `method = "..."` targets, which need method mappings.
pub fn count_method_targets(content: &str) -> usize {
    content
        .match_indices("method")
        .filter(|(pos, _)| {
            let rest = content[pos + "method".len()..].trim_start();
            rest.strip_prefix('=')
                .is_some_and(|r| r.trim_start().starts_with(['"', '{']))
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(from: &str, to: &str) -> RenameRule {
        RenameRule {
            from: from.into(),
            to: to.into(),
            kind: "move".into(),
            confidence: 1.0,
            version: "1.21".into(),
        }
    }

    #[test]
    fn test_config_compatibility_level_raised_only() {
        let src = "{\n  \"required\": true,\n  \"compatibilityLevel\": \"JAVA_8\",\n  \"mixins\": []\n}\n";
        let r = migrate_mixin_config(src, Some(17));
        assert_eq!(r.updated, src.replace("JAVA_8", "JAVA_17"));
        assert_eq!(r.replacements[0].line, 3);
        let r = migrate_mixin_config(&src.replace("JAVA_8", "JAVA_21"), Some(17));
        assert!(r.replacements.is_empty());
    }

    #[test]
    fn test_mixin_strings_descriptor_internal_and_targets() {
        let src = r#"@Mixin(targets = "com.example.old.DemoWidget")
class DemoMixin {
    @Inject(method = "tick(Lcom/example/old/DemoWidget;I)V", at = @At("HEAD"))
    void a() {}
    @Redirect(method = "b", at = @At(value = "INVOKE", target = "com/example/old/DemoWidget"))
    void b() {}
    String unrelated = "Lcom/example/old/DemoWidgetExtra;";
}"#;
        let r = migrate_mixin_strings(
            src,
            &[
                rule("com.example.old.DemoWidget", "com.example.demo.DemoWidget"),
                rule("DemoWidget", "DemoGadget"), // simple rules never apply here
            ],
        );
        assert!(r
            .updated
            .contains(r#"targets = "com.example.demo.DemoWidget""#));
        assert!(r.updated.contains("tick(Lcom/example/demo/DemoWidget;I)V"));
        assert!(r
            .updated
            .contains(r#"target = "com/example/demo/DemoWidget""#));
        assert!(r.updated.contains("Lcom/example/old/DemoWidgetExtra;"));
        assert_eq!(r.replacements.len(), 3);
        assert_eq!(count_method_targets(src), 2);
    }

    #[test]
    fn test_detection() {
        assert!(is_mixin_config(Path::new(
            "src/main/resources/demo.mixins.json"
        )));
        assert!(!is_mixin_config(Path::new("demo.json")));
        assert!(is_mixin_class("@Mixin(Foo.class) class X {}"));
    }
}
