//! The route contract: every route of the Laravel app
//! (tests/fixtures/routes.json, `artisan route:list --json` from the
//! Forge box) is answered by the api, the pages through their props
//! endpoint. Nothing a route served is a 404 or 405 now, except Laravel's
//! local disk routes under `/storage`, which never served anything the
//! app made (FIXES.md, deliberate changes).

use axum::body::Body;
use axum::http::{Method, StatusCode};

use crate::common::golden::read_json;
use crate::common::{HOST, TestApp, header_map, testlog};

#[tokio::test]
async fn every_laravel_route_is_answered() {
    let app = TestApp::new().await;
    let uuid = app.upload_log("combat.txt", &testlog()).await.to_string();
    let routes = read_json("routes.json");
    let mut checked = 0;
    for route in routes.as_array().expect("routes") {
        let uri = route["uri"].as_str().expect("a uri");
        let path = format!("/{}", uri.trim_start_matches('/'))
            .replace("{combatLog}", &uuid)
            .replace("{path}", "combat-logs/x.txt.gz");
        let path = match path.as_str() {
            "/" => "/api/pages/home".to_owned(),
            page if page.starts_with("/logs/") && page.matches('/').count() == 2 => {
                format!("/api/pages{page}")
            }
            other => other.to_owned(),
        };
        for method in route["method"].as_str().expect("methods").split('|') {
            let response = if method == "POST" {
                app.post(&path).await
            } else {
                app.request(
                    method.parse::<Method>().expect("a method"),
                    &path,
                    header_map(&[("host", HOST)]),
                    Body::empty(),
                )
                .await
            };
            if uri.starts_with("storage/") {
                assert_eq!(response.status, StatusCode::NOT_FOUND, "{method} {path}");
            } else {
                assert!(
                    response.status != StatusCode::NOT_FOUND
                        && response.status != StatusCode::METHOD_NOT_ALLOWED
                        && response.status != StatusCode::NOT_IMPLEMENTED,
                    "{method} {path}: {}",
                    response.status
                );
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 13);
}
