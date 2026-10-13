//! CombatLogUploadTest.php, one to one, with the assertions made exact:
//! the upload page's props, the upload, the analysis page's props, ESI
//! name resolution against the mock, the raw log download and the
//! regeneration.

use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::common::{TestApp, assert_json_eq, at, fixture, gunzip, gzip, testlog};

/// The answer `fakeEsiIds()` gave.
fn fake_esi_ids(app: &TestApp) {
    app.esi.set_table(json!({
        "characters": { "Aria Vex": 93000001, "Korr Malan": 93000002 },
        "inventory_types": { "Drake": 24698, "Hurricane": 24702 },
    }));
}

#[tokio::test]
async fn the_upload_page_renders_with_the_configured_max_upload_size() {
    let mut app = TestApp::new().await;
    app.reconfigure(|config| config.upload_max_size_mb = 7);
    assert_json_eq(
        &json!({ "maxUploadSizeMb": 7, "name": "EVE Log Analyzer", "auth": { "user": null } }),
        &app.props("/home").await,
    );
}

#[tokio::test]
async fn a_combat_log_can_be_uploaded_and_redirects_to_show_page() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let response = app.upload("combat.txt", &testlog()).await;
    let (id, uuid, listener, _, _) = app.first_log().await.expect("a log");
    assert_eq!(listener, "Nicolas Kion");
    assert!(app.event_count(id).await > 0);
    assert_eq!(response.redirect(), format!("/logs/{uuid}"));
}

#[tokio::test]
async fn the_show_page_passes_url_filters_as_a_prop() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let uuid = app.upload_log("combat.txt", &testlog()).await;
    let props = app
        .props(&format!(
            "/logs/{uuid}?from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut&pilot=Aria%20Vex"
        ))
        .await;
    assert_json_eq(
        &json!({
            "from": "2026-02-12T10:20:00",
            "to": "2026-02-12T10:25:00",
            "pilot": "Aria Vex",
            "hide": ["logiDealt", "neutOut"],
        }),
        at(&props, "filters"),
    );
}

#[tokio::test]
async fn the_show_page_returns_empty_filters_when_no_query_params_are_present() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let uuid = app.upload_log("combat.txt", &testlog()).await;
    let props = app.props(&format!("/logs/{uuid}")).await;
    assert_json_eq(
        &json!({ "from": null, "to": null, "pilot": null, "hide": [] }),
        at(&props, "filters"),
    );
}

#[tokio::test]
async fn the_show_page_renders_with_analysis_data() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let uuid = app.upload_log("combat.txt", &testlog()).await;
    let props = app.props(&format!("/logs/{uuid}")).await;
    assert_eq!(at(&props, "analysis.listener"), "Nicolas Kion");
    assert_eq!(at(&props, "analysis.sessionStarted"), "2026.02.12 09:41:44");
    assert_eq!(
        at(&props, "analysis.events")
            .as_array()
            .expect("events")
            .len(),
        120
    );
    assert_eq!(at(&props, "uuid"), &json!(uuid.to_string()));
    let mut keys: Vec<&String> = props.as_object().expect("props").keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "analysis",
            "auth",
            "filters",
            "name",
            "pilotIds",
            "rawLogAvailable",
            "shipTypeIds",
            "uuid"
        ]
    );
    assert_json_eq(
        &json!({
            "timestamp": "2026.02.12 09:42:44",
            "damage": 80,
            "direction": "outgoing",
            "playerName": "Aria Vex",
            "corporation": "RAZR",
            "shipName": "Drake",
            "weapon": "Heavy Missile",
            "quality": "Hits",
            "type": "damage",
        }),
        at(&props, "analysis.events.0"),
    );
}

#[tokio::test]
async fn uploading_without_a_file_returns_validation_error() {
    let app = TestApp::new().await;
    let response = app.post_multipart("/analyze", &[], &[]).await;
    assert_json_eq(
        &json!({ "log_file": "The log file field is required." }),
        &response.errors(),
    );
    assert_eq!(app.count("combat_logs").await, 0);
}

#[tokio::test]
async fn uploading_an_oversized_file_returns_validation_error() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let max_kb = 25 * 1024;
    let response = app
        .upload("huge.txt", &vec![0u8; (max_kb + 1) * 1024])
        .await;
    assert_json_eq(
        &json!({ "log_file": "The log file field must not be greater than 25600 kilobytes." }),
        &response.errors(),
    );
    assert_eq!(app.count("combat_logs").await, 0);
}

#[tokio::test]
async fn combat_events_are_stored_as_individual_rows() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    app.upload_log("combat.txt", &testlog()).await;
    let (id, ..) = app.first_log().await.expect("a log");
    assert_eq!(app.event_count(id).await, 120);
}

#[tokio::test]
async fn the_log_is_shareable_via_uuid_url() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let uuid = app.upload_log("combat.txt", &testlog()).await;
    assert_eq!(
        app.get(&format!("/api/pages/logs/{uuid}")).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn uploading_resolves_pilot_and_ship_names_to_eve_ids_via_esi() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    app.upload_log("combat.txt", &testlog()).await;
    let requests = app.esi.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].names,
        [
            "Aria Vex",
            "Korr Malan",
            "Selene Dur",
            "Tovak Rin",
            "Logi Pilot",
            "Drake",
            "Hurricane",
            "Caracal",
            "Rupture",
            "Basilisk"
        ]
    );
    assert_eq!(app.eve_id(None, "Aria Vex").await, Some(93000001));
    assert_eq!(app.eve_id(None, "Drake").await, Some(24698));
}

#[tokio::test]
async fn the_show_page_passes_resolved_eve_ids_as_props() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let uuid = app.upload_log("combat.txt", &testlog()).await;
    let props = app.props(&format!("/logs/{uuid}")).await;
    assert_json_eq(
        &json!({ "Aria Vex": 93000001, "Korr Malan": 93000002 }),
        at(&props, "pilotIds"),
    );
    assert_json_eq(
        &json!({ "Drake": 24698, "Hurricane": 24702 }),
        at(&props, "shipTypeIds"),
    );
}

#[tokio::test]
async fn names_already_resolved_are_not_requested_from_esi_again() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    app.create_entity("character", "Aria Vex", Some(93000001))
        .await;
    app.upload_log("combat.txt", &testlog()).await;
    let requests = app.esi.requests();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].names.iter().any(|name| name == "Aria Vex"));
    assert_eq!(requests[0].names.len(), 9);
}

#[tokio::test]
async fn uploading_still_succeeds_when_esi_is_unavailable() {
    let mut app = TestApp::new().await;
    app.reconfigure(|config| {
        config.esi.tries = 1;
        config.esi.retry_delay = std::time::Duration::ZERO;
    });
    app.esi.fail_with(500);
    let response = app.upload("combat.txt", &testlog()).await;
    let (_, uuid, ..) = app.first_log().await.expect("a log");
    assert_eq!(response.redirect(), format!("/logs/{uuid}"));
    assert_eq!(app.count("eve_entities").await, 0);
}

#[tokio::test]
async fn names_matching_an_item_type_are_never_treated_as_characters() {
    let app = TestApp::new().await;
    app.esi.set_table(json!({
        "characters": { "Hammerhead II": 95194769, "Aria Vex": 93000001 },
        "inventory_types": { "Hammerhead II": 2185 },
    }));
    let uuid = app
        .upload_log(
            "combat.txt",
            &fixture("upload-names-matching-an-item-type-are-never-treated-as-characters"),
        )
        .await;
    assert_eq!(app.eve_id(Some("character"), "Hammerhead II").await, None);
    assert_eq!(
        app.eve_id(Some("inventory_type"), "Hammerhead II").await,
        Some(2185)
    );
    let props = app.props(&format!("/logs/{uuid}")).await;
    assert_json_eq(&json!({ "Aria Vex": 93000001 }), at(&props, "pilotIds"));
    assert_json_eq(&json!({ "Hammerhead II": 2185 }), at(&props, "shipTypeIds"));
}

#[tokio::test]
async fn uploading_stores_the_compressed_raw_log_and_it_can_be_downloaded() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let contents = testlog();
    let uuid = app.upload_log("my-fight.txt", &contents).await;
    let stored = std::fs::read(app.raw_path(uuid)).expect("the raw log");
    assert_eq!(gunzip(&stored), contents);
    let props = app.props(&format!("/logs/{uuid}")).await;
    assert_eq!(at(&props, "rawLogAvailable"), &Value::Bool(true));

    let response = app.get(&format!("/logs/{uuid}/download")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.header("content-disposition"),
        Some("attachment; filename=my-fight.txt")
    );
    assert_eq!(
        response.header("content-type"),
        Some("text/plain; charset=utf-8")
    );
    assert_eq!(response.body, contents);
}

#[tokio::test]
async fn the_download_route_returns_404_when_no_raw_log_is_stored() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let uuid = app.upload_log("combat.txt", &testlog()).await;
    std::fs::remove_file(app.raw_path(uuid)).expect("the raw log goes");
    let props = app.props(&format!("/logs/{uuid}")).await;
    assert_eq!(at(&props, "rawLogAvailable"), &Value::Bool(false));
    assert_eq!(
        app.get(&format!("/logs/{uuid}/download")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_stored_combat_log_can_be_regenerated_from_its_raw_contents() {
    let app = TestApp::new().await;
    fake_esi_ids(&app);
    let contents = fixture("upload-a-stored-combat-log-can-be-regenerated-from-its-raw-contents");
    let (id, uuid) = app
        .create_log("Stale Listener", "2025.01.01 00:00:00", Some("fight.txt"))
        .await;
    app.create_event(
        id,
        "2025.01.01 00:00:01",
        1,
        "incoming",
        "Stale Pilot",
        "Stale Weapon",
    )
    .await;
    let compressed = gzip(&contents);
    std::fs::write(app.raw_path(uuid), &compressed).expect("the raw log");

    let response = app.post(&format!("/logs/{uuid}/regenerate")).await;
    assert_eq!(response.redirect(), format!("/logs/{uuid}"));

    let (_, _, listener, session_started, original_filename) =
        app.first_log().await.expect("the log");
    assert_eq!(listener, "Hunter Hughes");
    assert_eq!(session_started, "2026.07.20 22:25:40");
    assert_eq!(original_filename.as_deref(), Some("fight.txt"));
    assert_eq!(
        std::fs::read(app.raw_path(uuid)).expect("the raw log"),
        compressed
    );
    assert_eq!(app.event_count(id).await, 1);
    let event: (i64, String, Option<String>, Option<String>, String) = sqlx::query_as(
        "select damage, player_name, corporation, ship_name, weapon from combat_events
         where combat_log_id = $1",
    )
    .bind(id)
    .fetch_one(&app.pool)
    .await
    .expect("the event");
    assert_eq!(
        event,
        (
            382,
            "TinkerbeI".to_owned(),
            Some("LPPLS".to_owned()),
            Some("Crow".to_owned()),
            "'Peace' Large Remote Armor Repairer".to_owned()
        )
    );
}

#[tokio::test]
async fn regeneration_returns_404_without_changing_events_when_the_raw_log_is_missing() {
    let app = TestApp::new().await;
    let (id, uuid) = app
        .create_log(
            "Some Listener",
            "2026.01.01 00:00:00",
            Some("combat_log.txt"),
        )
        .await;
    app.create_event(
        id,
        "2026.01.01 00:00:01",
        100,
        "outgoing",
        "Existing Pilot",
        "Existing Weapon",
    )
    .await;
    let response = app.post(&format!("/logs/{uuid}/regenerate")).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(app.event_count(id).await, 1);
}
