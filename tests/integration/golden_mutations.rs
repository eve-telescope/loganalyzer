//! Every upload and regenerate scenario the Laravel reference answered
//! (tests/fixtures/golden/mutations.json), replayed against the api with
//! the same file names, bytes, cached names and ESI answers: the answer
//! (N4), the stored rows and the names ESI was asked for (N5). Where
//! Laravel failed with 500 or 419, the fixed answer is pinned
//! (FIXES.md).

use std::time::Instant;

use axum::body::Body;
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::common::golden::{scenario, scenario_names};
use crate::common::{
    APP_URL, HOST, TestApp, TestResponse, USER_AGENT, assert_json_eq, fixture, gzip, header_map,
    multipart,
};

/// The rows record.py read, in the same shape.
async fn rows(app: &TestApp) -> Value {
    let query = |sql: &'static str| {
        let pool = app.pool.clone();
        async move {
            sqlx::query_scalar::<_, Value>(sql)
                .fetch_all(&pool)
                .await
                .expect("rows read")
        }
    };
    let mut raw_logs: Vec<String> = std::fs::read_dir(app.storage.path())
        .expect("storage")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .collect();
    raw_logs.sort();
    json!({
        "combat_logs": query(
            "select json_build_object('uuid', uuid, 'listener', listener, 'session_started',
                session_started, 'original_filename', original_filename)
             from combat_logs order by id",
        ).await,
        "combat_events": query(
            "select json_build_object('log', l.uuid, 'timestamp', e.timestamp, 'damage', e.damage,
                'direction', e.direction, 'player_name', e.player_name, 'corporation', e.corporation,
                'ship_name', e.ship_name, 'weapon', e.weapon, 'quality', e.quality, 'type', e.type)
             from combat_events e join combat_logs l on l.id = e.combat_log_id order by e.id",
        ).await,
        "eve_entities": query(
            "select json_build_object('kind', kind, 'name', name, 'eve_id', eve_id)
             from eve_entities order by id",
        ).await,
        "raw_logs": raw_logs,
    })
}

/// N2: the recorded uuids in rows stand for the replay's.
fn replace_uuid(value: &Value, recorded: &str, fresh: &str) -> Value {
    serde_json::from_str(&value.to_string().replace(recorded, fresh)).expect("JSON")
}

/// The file a scenario uploaded.
fn file_bytes(file: &Value) -> Vec<u8> {
    match file["fixture"].as_str() {
        Some(name) => fixture(name),
        None => {
            let fill = file["fill"]
                .as_str()
                .map_or(b'a', |fill| fill.as_bytes()[0]);
            vec![fill; file["size"].as_u64().expect("a size") as usize]
        }
    }
}

struct Replay {
    recorded: Value,
    app: TestApp,
    response: TestResponse,
    seconds: f64,
}

/// Replays an upload scenario as a same-origin request.
async fn replay_upload(name: &str) -> Replay {
    let recorded = scenario(name);
    let app = TestApp::new().await;
    for entry in recorded["known"].as_array().expect("known names") {
        app.create_entity(
            entry[0].as_str().expect("a kind"),
            entry[1].as_str().expect("a name"),
            entry[2].as_i64(),
        )
        .await;
    }
    if let Some(status) = recorded["fail_esi"].as_u64().filter(|status| *status > 0) {
        app.esi.fail_with(status as u16);
    }
    let fields: Vec<(String, String)> = recorded["request"]["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .map(|field| {
            (
                field[0].as_str().expect("a name").to_owned(),
                field[1].as_str().expect("a value").to_owned(),
            )
        })
        .collect();
    let files: Vec<(String, String, Vec<u8>)> = recorded["request"]["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|file| {
            (
                file["field"].as_str().expect("a field").to_owned(),
                file["filename"].as_str().expect("a file name").to_owned(),
                file_bytes(file),
            )
        })
        .collect();
    let field_refs: Vec<(&str, &str)> = fields
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let file_refs: Vec<(&str, &str, &[u8])> = files
        .iter()
        .map(|(field, name, bytes)| (field.as_str(), name.as_str(), bytes.as_slice()))
        .collect();
    let started = Instant::now();
    let response = app
        .post_multipart("/analyze", &field_refs, &file_refs)
        .await;
    Replay {
        recorded,
        seconds: started.elapsed().as_secs_f64(),
        app,
        response,
    }
}

/// N4 and the rows: the scenario ended as Laravel's did.
async fn same_outcome(replay: &Replay) {
    let recorded = &replay.recorded;
    assert_eq!(recorded["status"], 302, "{}", recorded["name"]);
    let location = recorded["location"].as_str().expect("a location");
    let mut expected_rows = recorded["rows"].clone();
    if let Some(recorded_uuid) = location.strip_prefix("/logs/") {
        let redirect = replay.response.redirect();
        let fresh = redirect.strip_prefix("/logs/").expect("a log").to_owned();
        expected_rows = replace_uuid(&expected_rows, recorded_uuid, &fresh);
    } else {
        assert_eq!(location, "/");
        assert_json_eq(&recorded["errors"], &replay.response.errors());
    }
    assert_json_eq(&expected_rows, &rows(&replay.app).await);
}

/// N5: the same names in the same chunks, with the same compatibility
/// date; the User-Agent is the app's own.
fn same_esi_requests(replay: &Replay) {
    let recorded: Vec<&Value> = replay.recorded["esi_requests"]
        .as_array()
        .expect("requests")
        .iter()
        .collect();
    let actual = replay.app.esi.requests();
    if replay.recorded["fail_esi"].as_u64().unwrap_or(0) >= 500 {
        // Laravel tried five times, the api twice (FIXES.md).
        assert_eq!(recorded.len(), 5);
        assert_eq!(actual.len(), 2);
    } else {
        assert_eq!(actual.len(), recorded.len());
    }
    for (recorded, actual) in recorded.iter().zip(&actual) {
        assert_eq!(recorded["names"], json!(actual.names));
        assert_eq!(recorded["compatibility_date"], "2026-05-19");
        assert_eq!(actual.compatibility_date.as_deref(), Some("2026-05-19"));
        assert_eq!(recorded["user_agent"], "EVE Log Analyzer");
        assert_eq!(actual.user_agent.as_deref(), Some(USER_AGENT));
    }
}

async fn same_upload(name: &str) {
    let replay = replay_upload(name).await;
    same_outcome(&replay).await;
    same_esi_requests(&replay);
}

macro_rules! same_upload {
    ($($test:ident => $name:literal),* $(,)?) => {
        $(#[tokio::test] async fn $test() { same_upload($name).await; })*
    };
}

same_upload! {
    upload_testlog => "upload-testlog",
    upload_named_file => "upload-named-file",
    upload_testlog2 => "upload-testlog2",
    upload_testlog3 => "upload-testlog3",
    upload_hammerhead => "upload-hammerhead",
    upload_known_names => "upload-known-names",
    upload_esi_404 => "upload-esi-404",
    upload_tricky_names => "upload-tricky-names",
    upload_drones => "upload-drones",
    upload_invalid_utf8 => "upload-invalid-utf8",
    upload_chat_log => "upload-chat-log",
    upload_empty_file => "upload-empty-file",
    upload_uppercase_extension => "upload-uppercase-extension",
    upload_log_extension => "upload-log-extension",
    upload_windows_path => "upload-windows-path",
    upload_unicode_name => "upload-unicode-name",
    upload_max_size => "upload-max-size",
    upload_without_file => "upload-without-file",
    upload_empty_file_input => "upload-empty-file-input",
    upload_text_field => "upload-text-field",
    upload_blank_text_field => "upload-blank-text-field",
    upload_wrong_extension => "upload-wrong-extension",
    upload_no_extension => "upload-no-extension",
    upload_trailing_space_extension => "upload-trailing-space-extension",
    upload_oversized => "upload-oversized",
    upload_oversized_wrong_extension => "upload-oversized-wrong-extension",
}

/// FIXES.md, "ESI being down held an upload for 20 seconds".
#[tokio::test]
async fn upload_esi_down() {
    let mut replay = replay_upload("upload-esi-down").await;
    assert!(replay.recorded["seconds"].as_f64().expect("seconds") > 20.0);
    assert!(replay.seconds < 2.0, "{}", replay.seconds);
    same_outcome(&replay).await;
    same_esi_requests(&replay);
    // With the production settings (two tries, 250 ms apart) too.
    replay.app.reconfigure(|config| {
        config.esi.retry_delay = loganalyzer::config::DEFAULT_ESI_RETRY_DELAY;
    });
    let started = Instant::now();
    replay
        .app
        .upload("combat.txt", &fixture("testlog"))
        .await
        .redirect();
    assert!(started.elapsed().as_secs_f64() < 2.0);
}

/// The fixed uploads of FIXES.md, "Some logs failed the upload with
/// 500": the log is stored with the events the parser makes of it.
async fn fixed_upload(name: &str, fixture_name: &str) -> Value {
    let replay = replay_upload(name).await;
    assert_eq!(replay.recorded["status"], 500, "{name}");
    let redirect = replay.response.redirect();
    let uuid = redirect.strip_prefix("/logs/").expect("a log");
    let parsed = loganalyzer::parser::parse(&fixture(fixture_name));
    let stored = rows(&replay.app).await;
    assert_eq!(
        stored["combat_events"].as_array().expect("events").len(),
        parsed.events.len()
    );
    for (row, event) in stored["combat_events"]
        .as_array()
        .expect("events")
        .iter()
        .zip(&parsed.events)
    {
        assert_eq!(row["log"], uuid);
        assert_eq!(row["damage"], event.damage);
        assert_eq!(row["player_name"], event.player_name.as_str());
        assert_eq!(row["weapon"], event.weapon.as_str());
    }
    stored
}

#[tokio::test]
async fn upload_long_fields() {
    let stored = fixed_upload("upload-long-fields", "edge-long-fields").await;
    assert_eq!(stored["combat_events"][0]["player_name"], "L".repeat(300));
    assert_eq!(stored["combat_events"][1]["weapon"], "W".repeat(256));
}

#[tokio::test]
async fn upload_huge_numbers() {
    let stored = fixed_upload("upload-huge-numbers", "edge-huge-numbers").await;
    let damages: Vec<&Value> = stored["combat_events"]
        .as_array()
        .expect("events")
        .iter()
        .map(|event| &event["damage"])
        .collect();
    assert_eq!(
        damages,
        [
            &json!(4294967295_i64),
            &json!(4294967296_i64),
            &json!(i64::MAX),
            &json!(i64::MAX),
            &json!(123),
            &json!(i64::MAX)
        ]
    );
}

#[tokio::test]
async fn upload_numeric_pilot() {
    let stored = fixed_upload("upload-numeric-pilot", "edge-numeric-pilot").await;
    assert_eq!(stored["combat_events"][1]["player_name"], "12345");
}

#[tokio::test]
async fn upload_rich_neut_without_ship() {
    let stored = fixed_upload(
        "upload-rich-neut-without-ship",
        "edge-rich-neut-without-ship",
    )
    .await;
    assert_eq!(stored["combat_events"][0]["ship_name"], Value::Null);
}

/// FIXES.md (deliberate changes), "A same-origin check instead of the
/// CSRF token": Laravel refused a post without its token (419). A post
/// from another site is refused now, one without browser headers is
/// not.
#[tokio::test]
async fn upload_without_csrf() {
    let replay = replay_upload("upload-without-csrf").await;
    assert_eq!(replay.recorded["status"], 419);
    replay.response.redirect();

    let app = TestApp::new().await;
    let (content_type, body) = multipart(&[], &[("log_file", "combat.txt", &fixture("testlog"))]);
    for headers in [
        [
            ("origin", "https://evil.example"),
            ("sec-fetch-site", "cross-site"),
        ],
        [("origin", "https://evil.example"), ("x-none", "1")],
        [("origin", "null"), ("x-none", "1")],
        [("sec-fetch-site", "same-site"), ("x-none", "1")],
    ] {
        let mut all = vec![("host", HOST), ("content-type", content_type.as_str())];
        all.extend_from_slice(&headers);
        let response = app
            .request(
                Method::POST,
                "/analyze",
                header_map(&all),
                Body::from(body.clone()),
            )
            .await;
        assert_eq!(response.status, StatusCode::FORBIDDEN, "{headers:?}");
    }
    assert_eq!(app.count("combat_logs").await, 0);
    let plain = app
        .request(
            Method::POST,
            "/analyze",
            header_map(&[("host", HOST), ("content-type", content_type.as_str())]),
            Body::from(body),
        )
        .await;
    plain.redirect();
}

/// A regenerate scenario: the stale log record.py created, its raw file
/// written the same way.
async fn replay_regenerate(name: &str) -> (Value, TestApp, TestResponse, Uuid, Option<Vec<u8>>) {
    let recorded = scenario(name);
    let app = TestApp::new().await;
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
    let raw = match recorded["raw_fixture"].as_str() {
        Some("missing") => None,
        Some(fixture_name) => Some(gzip(&fixture(fixture_name))),
        None => Some(b"this is not gzip".to_vec()),
    };
    if let Some(raw) = &raw {
        std::fs::write(app.raw_path(uuid), raw).expect("the raw log");
    }
    let response = app.post(&format!("/logs/{uuid}/regenerate")).await;
    (recorded, app, response, uuid, raw)
}

#[tokio::test]
async fn regenerate() {
    let (recorded, app, response, uuid, raw) = replay_regenerate("regenerate").await;
    assert_eq!(recorded["status"], 302);
    assert_eq!(response.redirect(), format!("/logs/{uuid}"));
    let recorded_uuid = recorded["uuid"].as_str().expect("a uuid");
    assert_json_eq(
        &replace_uuid(&recorded["rows"], recorded_uuid, &uuid.to_string()),
        &rows(&app).await,
    );
    assert_eq!(recorded["file_unchanged"], true);
    assert_eq!(std::fs::read(app.raw_path(uuid)).ok(), raw);
    let asked: Vec<Vec<String>> = app.esi.requests().into_iter().map(|r| r.names).collect();
    assert_eq!(json!(asked), json!([recorded["esi_requests"][0]["names"]]));
}

/// FIXES.md, "A damaged raw log answered 500": PHP's `gzdecode` warned
/// before the 422 the controller meant to send.
#[tokio::test]
async fn regenerate_corrupt() {
    let (recorded, app, response, uuid, raw) = replay_regenerate("regenerate-corrupt").await;
    assert_eq!(recorded["status"], 500);
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_json_eq(
        &json!({ "message": "The stored combat log could not be decoded." }),
        &response.json(),
    );
    let recorded_uuid = recorded["uuid"].as_str().expect("a uuid");
    assert_json_eq(
        &replace_uuid(&recorded["rows"], recorded_uuid, &uuid.to_string()),
        &rows(&app).await,
    );
    assert_eq!(std::fs::read(app.raw_path(uuid)).ok(), raw);
}

#[tokio::test]
async fn regenerate_missing() {
    let (recorded, app, response, uuid, _) = replay_regenerate("regenerate-missing").await;
    assert_eq!(recorded["status"], 404);
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_json_eq(&json!({ "message": "Not Found" }), &response.json());
    let recorded_uuid = recorded["uuid"].as_str().expect("a uuid");
    assert_json_eq(
        &replace_uuid(&recorded["rows"], recorded_uuid, &uuid.to_string()),
        &rows(&app).await,
    );
}

#[tokio::test]
async fn regenerate_unknown() {
    let recorded = scenario("regenerate-unknown");
    assert_eq!(recorded["status"], 404);
    let app = TestApp::new().await;
    let path = "/logs/00000000-0000-4000-8000-000000000000/regenerate";
    assert_eq!(app.post(path).await.status, StatusCode::NOT_FOUND);
    // A plain form post gets the error page Laravel showed.
    let page = app
        .request(
            Method::POST,
            path,
            header_map(&[("host", HOST), ("origin", APP_URL)]),
            Body::empty(),
        )
        .await;
    assert_eq!(page.status, StatusCode::NOT_FOUND);
    assert_eq!(page.text(), recorded["body"].as_str().expect("the page"));
}

/// FIXES.md (deliberate changes), "A same-origin check instead of the
/// CSRF token".
#[tokio::test]
async fn regenerate_without_csrf() {
    let recorded = scenario("regenerate-without-csrf");
    assert_eq!(recorded["status"], 419);
    let app = TestApp::new().await;
    let (_, uuid) = app.create_log("Listener", "", None).await;
    let response = app
        .request(
            Method::POST,
            &format!("/logs/{uuid}/regenerate"),
            header_map(&[("host", HOST), ("origin", "https://evil.example")]),
            Body::empty(),
        )
        .await;
    assert_eq!(response.status, StatusCode::FORBIDDEN);
}

#[test]
fn every_recorded_scenario_has_a_test() {
    let source = include_str!("golden_mutations.rs");
    for name in scenario_names() {
        let test = name.replace('-', "_");
        assert!(
            source.contains(&format!("\"{name}\""))
                && (source.contains(&format!("fn {test}"))
                    || source.contains(&format!("{test} =>"))),
            "scenario {name} has no test"
        );
    }
}
