use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use tempfile::TempDir;
use tower::ServiceExt;
use ummp_core::{
    app::{AppServices, PortRequest, Settings},
    checkpoint::collect_files,
};
use ummp_server::router;

const TOKEN: &str = "test-bearer-token-with-at-least-32-bytes";

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/loader/fabric-src")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    collect_files(root)
        .unwrap()
        .into_iter()
        .map(|rel| (rel.clone(), std::fs::read(root.join(rel)).unwrap()))
        .collect()
}

fn setup(root: &Path) -> (TempDir, Arc<AppServices>, Router) {
    let data = tempfile::tempdir().unwrap();
    let services = Arc::new(AppServices::open(Settings::new(data.path().to_path_buf())).unwrap());
    let app = router(Arc::clone(&services), TOKEN.into(), vec![root.to_path_buf()]);
    (data, services, app)
}

fn request(method: &str, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {TOKEN}"))
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}

fn port_body(input: &Path, output: &Path) -> String {
    serde_json::json!({
        "input": input,
        "output": output,
        "to": "1.21.1",
        "from": "1.21.1",
        "loader": "neoforge",
        "zip": false,
        "min_confidence": 0.8,
        "kb": null,
        "mappings": null
    })
    .to_string()
}

#[tokio::test]
async fn requires_token() {
    let root = tempfile::tempdir().unwrap();
    let (_data, _services, app) = setup(root.path());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rejects_path_outside_allowed_roots() {
    let root = tempfile::tempdir().unwrap();
    let (_data, _services, app) = setup(root.path());
    let response = app
        .oneshot(request(
            "POST",
            "/v1/analyze",
            Body::from(r#"{"path":"/etc/passwd"}"#),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn plan_does_not_write() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("fabric");
    copy_tree(&fixture(), &input);
    let before = snapshot(&input);
    let (_data, _services, app) = setup(root.path());
    let body = serde_json::json!({
        "path": input,
        "from": "1.21.1",
        "to": "1.21.1",
        "loader": "neoforge",
        "min_confidence": 0.8
    });
    let response = app
        .oneshot(request("POST", "/v1/plan", Body::from(body.to_string())))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(snapshot(&input), before, "planning must not write to source");
}

#[tokio::test]
async fn migrate_without_reviewed_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("fabric");
    copy_tree(&fixture(), &input);
    let (_data, _services, app) = setup(root.path());
    let body = serde_json::json!({
        "path": input,
        "from": "1.21.1",
        "to": "1.21.1",
        "loader": "neoforge",
        "min_confidence": 0.8,
        "reviewed": false
    });
    let response = app
        .oneshot(request("POST", "/v1/migrate", Body::from(body.to_string())))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn port_job_completes_and_sse_events_are_ordered() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("fabric");
    let output = root.path().join("ported");
    copy_tree(&fixture(), &input);
    let (_data, _services, app) = setup(root.path());

    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/port",
            Body::from(port_body(&input, &output)),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    let started: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = started["job"].as_str().unwrap();

    let response = app
        .oneshot(request(
            "GET",
            &format!("/v1/jobs/{id}/events"),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();
    let mut stream_text = String::new();
    tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(frame) = http_body_util::BodyExt::frame(&mut body).await {
            if let Ok(frame) = frame {
                if let Ok(data) = frame.into_data() {
                    stream_text.push_str(&String::from_utf8_lossy(&data));
                    if stream_text.contains("event: job.finished") {
                        break;
                    }
                }
            }
        }
    })
    .await
    .expect("job event stream did not finish");
    let started_at = stream_text.find("event: job.started").expect("started event missing");
    let finished_at = stream_text.find("event: job.finished").expect("finished event missing");
    assert!(started_at < finished_at, "{stream_text}");
    assert!(stream_text.contains("event: job.progress"), "{stream_text}");
    assert!(output.join("UMMP_REPORT.md").is_file());
}

#[tokio::test]
async fn cancel_works() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("fabric");
    let output = root.path().join("ported");
    copy_tree(&fixture(), &input);
    let (_data, services, app) = setup(root.path());

    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    services.set_step_hook(Arc::new(move |_job, step| {
        if step == "porting" {
            entered_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
        }
    }));

    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/v1/port",
            Body::from(port_body(&input, &output)),
        ))
        .await
        .unwrap();
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    let started: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = started["job"].as_str().unwrap();
    entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();

    let response = app
        .oneshot(request(
            "POST",
            &format!("/v1/jobs/{id}/cancel"),
            Body::empty(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    release_tx.send(()).unwrap();

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if services.job(id.parse().unwrap()).unwrap().state
                == ummp_core::jobs::JobState::Cancelled
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("job was not cancelled");
}

#[tokio::test]
async fn body_too_large_is_413() {
    let root = tempfile::tempdir().unwrap();
    let (_data, _services, app) = setup(root.path());
    let body = format!(r#"{{"path":"{}"}}"#, "x".repeat(1024 * 1024));
    let response = app
        .oneshot(request("POST", "/v1/analyze", Body::from(body)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn health_is_public_and_openapi_lists_every_route() {
    let root = tempfile::tempdir().unwrap();
    let (_data, _services, app) = setup(root.path());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(request("GET", "/v1/openapi.json", Body::empty()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let spec: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for route in [
        "/v1/health",
        "/v1/analyze",
        "/v1/scan",
        "/v1/plan",
        "/v1/port",
        "/v1/migrate",
        "/v1/pack",
        "/v1/build",
        "/v1/jobs/{id}",
        "/v1/jobs/{id}/events",
        "/v1/jobs/{id}/cancel",
        "/v1/sessions/{project}/rollback",
        "/v1/openapi.json",
    ] {
        assert!(spec["paths"].get(route).is_some(), "missing {route}");
    }
}
