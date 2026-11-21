use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;
use ummp_core::app::{AppServices, Settings};
use ummp_server::router;

fn app(root: &std::path::Path) -> axum::Router {
    let data = tempdir().unwrap();
    let services = Arc::new(AppServices::open(Settings::new(data.path().to_path_buf())).unwrap());
    router(services, "test-bearer-token-with-at-least-32-bytes".into(), vec![root.into()])
}

fn authorized(method: &str, uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", "Bearer test-bearer-token-with-at-least-32-bytes")
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}

#[tokio::test]
async fn requires_token() {
    let root = tempdir().unwrap();
    let response = app(root.path())
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
    let root = tempdir().unwrap();
    let response = app(root.path())
        .oneshot(authorized(
            "POST",
            "/v1/analyze",
            Body::from(r#"{"path":"/etc/passwd"}"#),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn body_too_large_is_413() {
    let root = tempdir().unwrap();
    let body = format!(r#"{{"path":"{}"}}"#, "x".repeat(1024 * 1024));
    let response = app(root.path())
        .oneshot(authorized("POST", "/v1/analyze", Body::from(body)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn health_is_public_and_openapi_lists_every_route() {
    let root = tempdir().unwrap();
    let response = app(root.path())
        .oneshot(
            Request::builder()
                .uri("/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app(root.path())
        .oneshot(authorized("GET", "/v1/openapi.json", Body::empty()))
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
