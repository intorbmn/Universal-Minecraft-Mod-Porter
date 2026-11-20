//! Command bodies. Each one is a single call into `AppServices` (ADR 0004): no migration,
//! scan, or analysis logic lives here. The `#[tauri::command]` wrappers are in `lib.rs`.

use std::path::Path;

use serde::Serialize;
use ummp_core::app::AppError;
use ummp_core::app::AppServices;
use ummp_core::app::views::{AnalysisView, RiskView, SessionView, TargetsView};

/// The only error shape the web view sees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UiError {
    /// `"input"`, `"refused"`, `"not_found"`, `"cancelled"`, or `"internal"`.
    pub kind: &'static str,
    pub message: String,
}

impl From<AppError> for UiError {
    fn from(e: AppError) -> Self {
        let kind = match &e {
            AppError::Input(_) => "input",
            AppError::Refused(_) => "refused",
            AppError::NotFound(_) => "not_found",
            AppError::Cancelled => "cancelled",
            _ => "internal",
        };
        Self {
            kind,
            message: e.to_string(),
        }
    }
}

pub fn analyze(svc: &AppServices, input: &Path) -> Result<AnalysisView, UiError> {
    Ok(svc.analyze(input)?)
}

pub fn scan(svc: &AppServices, input: &Path) -> Result<RiskView, UiError> {
    Ok(svc.scan(input)?)
}

pub fn sessions(svc: &AppServices, limit: usize) -> Result<Vec<SessionView>, UiError> {
    Ok(svc.sessions(limit)?)
}

pub fn targets(svc: &AppServices) -> TargetsView {
    svc.targets()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use ummp_core::app::Settings;

    fn services(dir: &Path) -> AppServices {
        AppServices::open(Settings::new(dir.to_path_buf())).unwrap()
    }

    fn fixture(rel: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(rel)
    }

    #[test]
    fn analyze_returns_the_core_view_for_a_real_mod_folder() {
        let d = tempdir().unwrap();
        let view = analyze(&services(d.path()), &fixture("loader/fabric-src")).unwrap();
        assert_eq!(view.kind, "folder");
        assert_eq!(view.mods[0].loader, "fabric");
    }

    #[test]
    fn missing_input_becomes_an_input_error_with_the_core_message() {
        let d = tempdir().unwrap();
        let err = scan(&services(d.path()), &d.path().join("nope")).unwrap_err();
        assert_eq!(err.kind, "input");
        assert!(err.message.contains("does not exist"), "{err:?}");
    }

    #[test]
    fn sessions_is_empty_on_a_fresh_data_folder_and_targets_lists_loaders() {
        let d = tempdir().unwrap();
        let svc = services(d.path());
        assert!(sessions(&svc, 10).unwrap().is_empty());
        assert!(!targets(&svc).loaders.is_empty());
    }

    #[test]
    fn error_kinds_map_one_to_one() {
        let k = |e: AppError| UiError::from(e).kind;
        assert_eq!(k(AppError::Refused("x".into())), "refused");
        assert_eq!(k(AppError::NotFound("x".into())), "not_found");
        assert_eq!(k(AppError::Cancelled), "cancelled");
        assert_eq!(k(AppError::Io(std::io::Error::other("x"))), "internal");
    }
}
