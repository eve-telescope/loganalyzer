//! The props of the two pages (`/api/pages/*`): the upload page, and the
//! analysis page for the fixture logs with the names resolved through the
//! mock ESI, the URL state read from the query, the raw file's state and
//! the links that are not a log.

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::common::{TestApp, assert_json_eq, fixture};

const NO_FILTERS: Value = Value::Null;

/// The analysis page's props: the parsed log, its uuid and the given ids
/// and filters (`NO_FILTERS` for an empty query).
fn log_props(
    fixture_name: &str,
    uuid: Uuid,
    raw_log_available: bool,
    pilot_ids: Value,
    ship_type_ids: Value,
    filters: Value,
) -> Value {
    let filters = if filters.is_null() {
        json!({ "from": null, "to": null, "pilot": null, "hide": [] })
    } else {
        filters
    };
    json!({
        "name": "EVE Log Analyzer",
        "auth": { "user": null },
        "analysis": loganalyzer::parser::parse(&fixture(fixture_name)),
        "uuid": uuid.to_string(),
        "rawLogAvailable": raw_log_available,
        "pilotIds": pilot_ids,
        "shipTypeIds": ship_type_ids,
        "filters": filters,
    })
}

fn testlog_pilot_ids() -> Value {
    json!({
        "Aria Vex": 93000001,
        "Korr Malan": 93000002,
        "Selene Dur": 93000003,
        "Tovak Rin": 93000004,
        "Logi Pilot": 93000005,
    })
}

fn testlog_ship_type_ids() -> Value {
    json!({
        "Drake": 24698,
        "Hurricane": 24702,
        "Caracal": 621,
        "Rupture": 629,
        "Basilisk": 11985,
    })
}

#[tokio::test]
async fn the_upload_page_names_the_size_limit() {
    let app = TestApp::new().await;
    assert_json_eq(
        &json!({ "name": "EVE Log Analyzer", "auth": { "user": null }, "maxUploadSizeMb": 25 }),
        &app.props("/home").await,
    );
}

/// Each fixture log, uploaded and opened: the parsed log with the ids of
/// the names ESI knows. A name that is also a ship is never a pilot.
#[tokio::test]
async fn the_analysis_page_has_the_log_and_the_ids_of_its_names() {
    for (fixture_name, pilot_ids, ship_type_ids) in [
        ("testlog", testlog_pilot_ids(), testlog_ship_type_ids()),
        ("testlog2", json!({}), json!({ "Sabre": 22456 })),
        (
            "testlog3",
            json!({}),
            json!({ "Basilisk": 11985, "Huginn": 11961 }),
        ),
        (
            "upload-names-matching-an-item-type-are-never-treated-as-characters",
            json!({ "Aria Vex": 93000001 }),
            json!({ "Drake": 24698, "Hammerhead II": 2185 }),
        ),
        ("edge-tricky-names", json!({}), json!({})),
        (
            "edge-drone-attribution",
            json!({}),
            json!({ "Rupture": 629, "Hammerhead II": 2185 }),
        ),
        ("edge-empty", json!({}), json!({})),
        (
            "edge-multiline-then-notify",
            json!({ "TinkerbeI": 93000008 }),
            json!({ "Crow": 11176 }),
        ),
    ] {
        let app = TestApp::new().await;
        let uuid = app.upload_log("combat.txt", &fixture(fixture_name)).await;
        assert_json_eq(
            &log_props(
                fixture_name,
                uuid,
                true,
                pilot_ids,
                ship_type_ids,
                NO_FILTERS,
            ),
            &app.props(&format!("/logs/{uuid}")).await,
        );
    }
}

/// The URL state as the page gets it: trimmed (Unicode spaces too),
/// empty as null, the last occurrence of a parameter, `hide` split with
/// its empty parts and `0` dropped, array parameters ignored.
#[tokio::test]
async fn the_analysis_page_reads_the_url_state() {
    let app = TestApp::new().await;
    let uuid = app.upload_log("combat.txt", &fixture("testlog")).await;
    for (query, filters) in [
        (
            "from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut&pilot=Aria%20Vex",
            json!({
                "from": "2026-02-12T10:20:00",
                "to": "2026-02-12T10:25:00",
                "pilot": "Aria Vex",
                "hide": ["logiDealt", "neutOut"],
            }),
        ),
        (
            "from=%20&to=&pilot=%20%20Aria%20Vex%20&hide=%20dpsDealt%20,,0,%20,neutIn,%200%20",
            json!({ "from": null, "to": null, "pilot": "Aria Vex", "hide": ["dpsDealt", "neutIn"] }),
        ),
        (
            "pilot=%E2%80%8BAria%E3%80%80Vex%C2%A0&from=%E2%80%8E2026-02-12T10:20:00",
            json!({
                "from": "2026-02-12T10:20:00",
                "to": null,
                "pilot": "Aria\u{3000}Vex",
                "hide": [],
            }),
        ),
        (
            "from=a&from=b&hide=dpsDealt&hide=neutOut",
            json!({ "from": "b", "to": null, "pilot": null, "hide": ["neutOut"] }),
        ),
        (
            "pilot=Aria+Vex&hide=",
            json!({ "from": null, "to": null, "pilot": "Aria Vex", "hide": [] }),
        ),
        (
            "hide=0",
            json!({ "from": null, "to": null, "pilot": null, "hide": [] }),
        ),
        (
            "pilot%5B%5D=Aria%20Vex&from%5Bx%5D=1",
            json!({ "from": null, "to": null, "pilot": null, "hide": [] }),
        ),
        (
            "hide%5B%5D=dpsDealt",
            json!({ "from": null, "to": null, "pilot": null, "hide": [] }),
        ),
    ] {
        assert_json_eq(
            &log_props(
                "testlog",
                uuid,
                true,
                testlog_pilot_ids(),
                testlog_ship_type_ids(),
                filters,
            ),
            &app.props(&format!("/logs/{uuid}?{query}")).await,
        );
    }
}

/// A log whose raw file is gone or damaged still opens; only a missing
/// file turns the download and the regeneration off.
#[tokio::test]
async fn the_analysis_page_tells_whether_the_raw_log_is_there() {
    for (raw, available) in [(None, false), (Some(b"not gzip at all".as_slice()), true)] {
        let app = TestApp::new().await;
        let uuid = app.upload_log("combat.txt", &fixture("testlog2")).await;
        match raw {
            Some(bytes) => std::fs::write(app.raw_path(uuid), bytes).expect("a broken file"),
            None => std::fs::remove_file(app.raw_path(uuid)).expect("the raw log goes"),
        }
        assert_json_eq(
            &log_props(
                "testlog2",
                uuid,
                available,
                json!({}),
                json!({ "Sabre": 22456 }),
                NO_FILTERS,
            ),
            &app.props(&format!("/logs/{uuid}")).await,
        );
    }
}

/// A log opens by its uuid in either case; anything else is not a log.
#[tokio::test]
async fn only_the_uuid_opens_a_log() {
    let app = TestApp::new().await;
    let uuid = app.upload_log("combat.txt", &fixture("testlog")).await;
    let props = app
        .props(&format!("/logs/{}", uuid.to_string().to_uppercase()))
        .await;
    assert_eq!(props["uuid"], uuid.to_string());
    for path in [
        "00000000-0000-4000-8000-000000000000".to_owned(),
        "not-a-uuid".to_owned(),
        format!("%7B{uuid}%7D"),
        uuid.simple().to_string(),
        format!("{uuid}%20"),
    ] {
        let response = app.get(&format!("/api/pages/logs/{path}")).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND, "{path}");
        assert_json_eq(&json!({ "message": "Not Found" }), &response.json());
    }
}

#[tokio::test]
async fn a_trailing_slash_redirects_to_the_page() {
    let app = TestApp::new().await;
    let uuid = app.upload_log("combat.txt", &fixture("testlog")).await;
    let response = app.get(&format!("/api/pages/logs/{uuid}/")).await;
    assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(response.location(), format!("/api/pages/logs/{uuid}"));
}
