//! UMMP desktop shell. Thin by design: commands forward to `ummp_core::app::AppServices`
//! and job events are forwarded to the window as `ummp://event`.

mod commands;

use std::path::PathBuf;
use std::sync::Arc;

use commands::UiError;
use tauri::{AppHandle, Emitter, Manager, State};
use ummp_core::app::views::{
    AnalysisView, JobView, PlanView, RiskView, SessionView, TargetsView,
};
use ummp_core::app::{AppServices, PlanRequest, PortRequest, Settings};
use ummp_core::events::{Event, EventHandler};
use uuid::Uuid;

/// Name of the window event that carries every job event.
pub const WINDOW_EVENT: &str = "ummp://event";

/// Sends each core event to the web view.
struct WindowForwarder(AppHandle);

impl EventHandler for WindowForwarder {
    fn handle(&self, event: &Event) {
        // A closed window is not an error for a job: the job keeps running and is saved.
        let _ = self.0.emit(WINDOW_EVENT, event);
    }
}

type Svc<'a> = State<'a, Arc<AppServices>>;

async fn blocking<T, F>(svc: &Svc<'_>, f: F) -> Result<T, UiError>
where
    T: Send + 'static,
    F: FnOnce(&AppServices) -> Result<T, UiError> + Send + 'static,
{
    let svc = Arc::clone(svc.inner());
    tauri::async_runtime::spawn_blocking(move || f(&svc))
        .await
        .map_err(|e| UiError {
            kind: "internal",
            message: format!("worker stopped: {e}"),
        })?
}

#[tauri::command]
async fn analyze(svc: Svc<'_>, input: PathBuf) -> Result<AnalysisView, UiError> {
    blocking(&svc, move |s| commands::analyze(s, &input)).await
}

#[tauri::command]
async fn scan(svc: Svc<'_>, input: PathBuf) -> Result<RiskView, UiError> {
    blocking(&svc, move |s| commands::scan(s, &input)).await
}

#[tauri::command]
async fn sessions(svc: Svc<'_>, limit: usize) -> Result<Vec<SessionView>, UiError> {
    blocking(&svc, move |s| commands::sessions(s, limit)).await
}

// Tauri hands State over by value; a reference parameter is not accepted.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
fn targets(svc: Svc<'_>) -> TargetsView {
    commands::targets(&svc)
}

#[tauri::command]
async fn plan(svc: Svc<'_>, request: PlanRequest) -> Result<PlanView, UiError> {
    blocking(&svc, move |s| commands::plan(s, &request)).await
}

#[tauri::command]
fn start_port(svc: Svc<'_>, request: PortRequest) -> Result<Uuid, UiError> {
    commands::start_port(&svc, request)
}

#[tauri::command]
fn start_migrate_in_place(
    svc: Svc<'_>,
    request: PlanRequest,
    reviewed: bool,
) -> Result<Uuid, UiError> {
    commands::start_migrate_in_place(&svc, request, reviewed)
}

#[tauri::command]
async fn job(svc: Svc<'_>, id: Uuid) -> Result<JobView, UiError> {
    blocking(&svc, move |s| commands::job(s, id)).await
}

#[tauri::command]
fn cancel(svc: Svc<'_>, id: Uuid) -> Result<(), UiError> {
    commands::cancel(&svc, id)
}

#[tauri::command]
async fn rollback(
    svc: Svc<'_>,
    project: PathBuf,
    checkpoint: Option<Uuid>,
) -> Result<(), UiError> {
    blocking(&svc, move |s| commands::rollback(s, &project, checkpoint)).await
}

/// Starts the desktop app.
///
/// # Panics
/// Panics when Tauri cannot start (no window system, broken config): nothing can run then.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let svc = Arc::new(AppServices::open(Settings::new(dir))?);
            svc.subscribe(Arc::new(WindowForwarder(app.handle().clone())));
            app.manage(svc);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            analyze,
            scan,
            sessions,
            targets,
            plan,
            start_port,
            start_migrate_in_place,
            job,
            cancel,
            rollback
        ])
        .run(tauri::generate_context!())
        .expect("error while running the UMMP desktop app");
}
