//! Command bodies. Each one is a thin call into `AppServices` (ADR 0004).
//! No migration, scan, build, or filesystem business logic lives in this crate.

use std::path::Path;

use serde::Serialize;
use ummp_core::app::views::{
    AnalysisView, JobView, PlanView, RiskView, SessionView, TargetsView,
};
use ummp_core::app::{
    AppError, AppServices, PlanRequest, PortRequest,
};
use uuid::Uuid;

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

pub fn plan(svc: &AppServices, request: &PlanRequest) -> Result<PlanView, UiError> {
    Ok(svc.plan(request)?)
}

pub fn start_port(svc: &AppServices, request: PortRequest) -> Result<Uuid, UiError> {
    Ok(svc.start_port(request)?)
}

pub fn start_migrate_in_place(
    svc: &AppServices,
    request: PlanRequest,
    reviewed: bool,
) -> Result<Uuid, UiError> {
    Ok(svc.start_migrate_in_place(request, reviewed)?)
}

pub fn job(svc: &AppServices, id: Uuid) -> Result<JobView, UiError> {
    Ok(svc.job(id)?)
}

pub fn cancel(svc: &AppServices, id: Uuid) -> Result<(), UiError> {
    Ok(svc.cancel(id)?)
}

pub fn rollback(
    svc: &AppServices,
    project: &Path,
    checkpoint: Option<Uuid>,
) -> Result<(), UiError> {
    Ok(svc.rollback(project, checkpoint)?)
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
    fn plan_returns_the_core_diff_without_writing_the_source() {
        let d = tempdir().unwrap();
        let svc = services(d.path());
        let source = fixture("loader/fabric-src");
        let request = PlanRequest {
            input: source.clone(),
            from: Some("1.20.1".into()),
            to: "1.21.1".into(),
            loader: Some("neoforge".into()),
            min_confidence: 0.8,
        };
        let view = plan(&svc, &request).unwrap();
        assert_eq!(view.target_minecraft, "1.21.1");
        assert!(!view.files.is_empty());
        assert!(source.join("src/main/resources/fabric.mod.json").exists());
    }

    #[test]
    fn unreviewed_in_place_apply_is_refused_at_the_command_boundary() {
        let d = tempdir().unwrap();
        let svc = services(d.path());
        let request = PlanRequest {
            input: fixture("loader/fabric-src"),
            from: Some("1.20.1".into()),
            to: "1.21.1".into(),
            loader: Some("neoforge".into()),
            min_confidence: 0.8,
        };
        let err = start_migrate_in_place(&svc, request, false).unwrap_err();
        assert_eq!(err.kind, "refused");
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
