//! tests/Feature/ExampleTest.php ("returns a successful response"): the
//! home page answers. Its props come from the api; the page itself is
//! pinned by frontend/e2e/routes.spec.ts. tests/Unit/ExampleTest.php
//! (`true is true`) has nothing to port.

use axum::http::StatusCode;

use crate::common::TestApp;

#[tokio::test]
async fn returns_a_successful_response() {
    let app = TestApp::new().await;
    assert_eq!(app.get("/api/pages/home").await.status, StatusCode::OK);
}
