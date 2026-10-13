//! The upload form and the regeneration end to end: what is accepted and
//! with which message the rest is refused, the stored log, its events,
//! its raw file and the names cached from ESI.

use std::time::Instant;

use axum::body::Body;
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::common::{APP_URL, HOST, TestApp, assert_json_eq, fixture, gzip, header_map, multipart};

/// The cached names, in the order they were stored.
async fn entities(app: &TestApp) -> Vec<(String, String, Option<i64>)> {
    sqlx::query_as("select kind, name, eve_id from eve_entities order by id")
        .fetch_all(&app.pool)
        .await
        .expect("entities read")
}

fn character(name: &str, eve_id: Option<i64>) -> (String, String, Option<i64>) {
    ("character".to_owned(), name.to_owned(), eve_id)
}

fn item_type(name: &str, eve_id: Option<i64>) -> (String, String, Option<i64>) {
    ("inventory_type".to_owned(), name.to_owned(), eve_id)
}

/// The names ESI was asked for, one list per request.
fn asked(app: &TestApp) -> Vec<Vec<String>> {
    app.esi
        .requests()
        .into_iter()
        .map(|request| request.names)
        .collect()
}

/// The raw files in storage.
fn raw_files(app: &TestApp) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(app.storage.path())
        .expect("storage")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// The stored log's listener, session start and file name.
async fn stored_log(app: &TestApp) -> (String, String, Option<String>) {
    let (_, _, listener, session_started, original_filename) =
        app.first_log().await.expect("a log");
    (listener, session_started, original_filename)
}

/// A refused upload: what it is, its text fields, its files (field,
/// name, bytes) and the message.
type RefusedUpload<'a> = (
    &'a str,
    Vec<(&'a str, &'a str)>,
    Vec<(&'a str, &'a str, Vec<u8>)>,
    &'a str,
);

#[tokio::test]
async fn refuses_what_is_not_a_log_file() {
    let filled = |size: usize| vec![b'a'; size];
    let testlog = fixture("testlog");
    let required = "The log file field is required.";
    let extensions = "The log file field must have one of the following extensions: txt, log.";
    let cases: Vec<RefusedUpload> = vec![
        ("no file", vec![], vec![], required),
        (
            "an empty file input",
            vec![],
            vec![("log_file", "", vec![])],
            required,
        ),
        (
            "a text field",
            vec![("log_file", "not a file")],
            vec![],
            "The log file field must be a file.",
        ),
        (
            "a blank text field",
            vec![("log_file", "   ")],
            vec![],
            required,
        ),
        (
            "a pdf",
            vec![],
            vec![("log_file", "fight.pdf", testlog.clone())],
            extensions,
        ),
        (
            "no extension",
            vec![],
            vec![("log_file", "fight", testlog.clone())],
            extensions,
        ),
        (
            "a space after the extension",
            vec![],
            vec![("log_file", "fight.txt ", testlog.clone())],
            extensions,
        ),
        (
            "one byte over 25 MB",
            vec![],
            vec![("log_file", "huge.txt", filled(25 * 1024 * 1024 + 1))],
            "The log file field must not be greater than 25600 kilobytes.",
        ),
        (
            "too big and a pdf",
            vec![],
            vec![("log_file", "huge.pdf", filled(25 * 1024 * 1024 + 1))],
            extensions,
        ),
    ];
    for (case, fields, files, message) in cases {
        let app = TestApp::new().await;
        let files: Vec<(&str, &str, &[u8])> = files
            .iter()
            .map(|(field, name, bytes)| (*field, *name, bytes.as_slice()))
            .collect();
        let response = app.post_multipart("/analyze", &fields, &files).await;
        assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY, "{case}");
        assert_json_eq(&json!({ "log_file": message }), &response.errors());
        assert_eq!(app.count("combat_logs").await, 0, "{case}");
        assert!(raw_files(&app).is_empty(), "{case}");
        assert!(app.esi.requests().is_empty(), "{case}");
    }
}

#[tokio::test]
async fn keeps_the_file_name_the_browser_sent() {
    for (sent, stored) in [
        ("combat.txt", "combat.txt"),
        ("FIGHT.TXT", "FIGHT.TXT"),
        ("fight.Log", "fight.Log"),
        ("testlog2.log", "testlog2.log"),
        ("C:\\fakepath\\fight.txt", "fight.txt"),
        ("Kämpfe 日本.txt", "Kämpfe 日本.txt"),
    ] {
        let app = TestApp::new().await;
        let uuid = app.upload_log(sent, &fixture("testlog")).await;
        assert_eq!(
            stored_log(&app).await,
            (
                "Nicolas Kion".to_owned(),
                "2026.02.12 09:41:44".to_owned(),
                Some(stored.to_owned())
            ),
            "{sent}"
        );
        assert_eq!(raw_files(&app), [format!("{uuid}.txt.gz")], "{sent}");
    }
}

/// Anything with the right name is stored, even when nothing in it is a
/// combat line: the analysis is then empty.
#[tokio::test]
async fn stores_files_without_combat_lines_as_empty_logs() {
    for (name, bytes) in [
        ("empty.txt", Vec::new()),
        ("broken.txt", fixture("edge-invalid-utf8")),
        ("Local_20260101.txt", fixture("edge-utf16le-chat-bom")),
        ("big.txt", vec![b'a'; 25 * 1024 * 1024]),
    ] {
        let app = TestApp::new().await;
        let uuid = app.upload_log(name, &bytes).await;
        assert_eq!(
            stored_log(&app).await,
            ("Unknown".to_owned(), String::new(), Some(name.to_owned())),
            "{name}"
        );
        assert_eq!(app.count("combat_events").await, 0, "{name}");
        assert_eq!(raw_files(&app), [format!("{uuid}.txt.gz")], "{name}");
        assert!(app.esi.requests().is_empty(), "{name}");
    }
}

#[tokio::test]
async fn caches_every_name_of_the_log_under_the_kind_esi_knows_it_as() {
    let app = TestApp::new().await;
    app.upload_log("combat.txt", &fixture("testlog")).await;
    assert_eq!(app.count("combat_events").await, 120);
    assert_eq!(
        asked(&app),
        [[
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
        ]]
    );
    assert_eq!(
        entities(&app).await,
        [
            character("Aria Vex", Some(93000001)),
            character("Korr Malan", Some(93000002)),
            character("Selene Dur", Some(93000003)),
            character("Tovak Rin", Some(93000004)),
            character("Logi Pilot", Some(93000005)),
            item_type("Drake", Some(24698)),
            item_type("Hurricane", Some(24702)),
            item_type("Caracal", Some(621)),
            item_type("Rupture", Some(629)),
            item_type("Basilisk", Some(11985)),
        ]
    );
}

/// A name ESI does not know is cached without an id, so it is not asked
/// for again; a ship name in the pilot column is cached for both kinds.
#[tokio::test]
async fn caches_unknown_names_without_an_id() {
    let app = TestApp::new().await;
    app.upload_log("testlog2.log", &fixture("testlog2")).await;
    assert_eq!(app.count("combat_events").await, 950);
    assert_eq!(
        asked(&app),
        [["Sabre", "Curse", "Bhaalgorn", "Stiletto", "Ashimmu"]]
    );
    assert_eq!(
        entities(&app).await,
        [
            character("Sabre", None),
            item_type("Sabre", Some(22456)),
            character("Curse", None),
            character("Bhaalgorn", None),
            character("Stiletto", None),
            character("Ashimmu", None),
            item_type("Curse", None),
            item_type("Bhaalgorn", None),
            item_type("Stiletto", None),
            item_type("Ashimmu", None),
        ]
    );
}

/// A cached name counts only in its stored spelling: another spelling is
/// asked for again and updates the row, which keeps its spelling.
#[tokio::test]
async fn asks_again_for_names_cached_in_another_spelling() {
    let app = TestApp::new().await;
    app.create_entity("character", "Aria Vex", Some(93000001))
        .await;
    app.create_entity("character", "korr malan", None).await;
    app.create_entity("inventory_type", "DRAKE", Some(24698))
        .await;
    app.upload_log("combat.txt", &fixture("testlog")).await;
    assert_eq!(
        asked(&app),
        [[
            "Korr Malan",
            "Selene Dur",
            "Tovak Rin",
            "Logi Pilot",
            "Drake",
            "Hurricane",
            "Caracal",
            "Rupture",
            "Basilisk"
        ]]
    );
    assert_eq!(
        entities(&app).await,
        [
            character("Aria Vex", Some(93000001)),
            character("korr malan", Some(93000002)),
            item_type("DRAKE", Some(24698)),
            character("Selene Dur", Some(93000003)),
            character("Tovak Rin", Some(93000004)),
            character("Logi Pilot", Some(93000005)),
            item_type("Hurricane", Some(24702)),
            item_type("Caracal", Some(621)),
            item_type("Rupture", Some(629)),
            item_type("Basilisk", Some(11985)),
        ]
    );
}

#[tokio::test]
async fn sends_names_with_markup_entities_and_unicode_as_the_log_has_them() {
    let app = TestApp::new().await;
    app.upload_log("names.txt", &fixture("edge-tricky-names"))
        .await;
    assert_eq!(app.count("combat_events").await, 138);
    let names = [
        "Pilot [with] brackets",
        "Pilot (paren)",
        "Pilot - dash",
        "Pilot < less",
        "Bantam",
        "Pilot bold",
        "Zoë Ångström",
        "No\u{a0}Break Space",
        "Leading NBSP",
        "Trailing NBSP",
        "Tab\tInside",
        "Mongolian",
        "Ideographic",
        "Em Space",
        "日本語のパイロット",
        "Emoji 🚀 Pilot",
        "Quote \"Pilot\"",
        "Apostrophe's Pilot",
        "ad",
        "Pilot",
        "Amp &amp; Pilot",
        "00",
    ];
    let mut expected_asked: Vec<&str> = names.to_vec();
    expected_asked.push("Rifter");
    assert_eq!(asked(&app), [expected_asked]);
    let mut expected: Vec<_> = names.iter().map(|name| character(name, None)).collect();
    expected.push(item_type("Rifter", None));
    expected.push(item_type("Bantam", None));
    assert_eq!(entities(&app).await, expected);
}

#[tokio::test]
async fn caches_drone_names_as_item_types() {
    let app = TestApp::new().await;
    app.upload_log("drones.txt", &fixture("edge-drone-attribution"))
        .await;
    assert_eq!(app.count("combat_events").await, 20);
    let pilots = [
        "Tom DaNanRen",
        "Pilot A",
        "Pilot B",
        "Pilot C",
        "Pilot D",
        "Valkyrie II",
        "Lonely Pilot",
        "Pilot E",
        "Pilot F",
        "Hornet II",
        "Pilot G",
        "Infiltrator II",
        "Late Pilot",
    ];
    let types = [
        "Cenotaph",
        "Hammerhead II",
        "Rupture",
        "Warrior II",
        "Rifter",
        "Acolyte II",
        "Hobgoblin II",
        "Ogre II",
        "Myrmidon",
    ];
    let expected_asked: Vec<&str> = pilots.iter().chain(&types).copied().collect();
    assert_eq!(asked(&app), [expected_asked]);
    let mut expected: Vec<_> = pilots.iter().map(|name| character(name, None)).collect();
    expected.extend([
        item_type("Cenotaph", None),
        item_type("Hammerhead II", Some(2185)),
        item_type("Rupture", Some(629)),
        item_type("Warrior II", None),
        item_type("Valkyrie II", None),
        item_type("Rifter", None),
        item_type("Acolyte II", None),
        item_type("Hornet II", None),
        item_type("Hobgoblin II", None),
        item_type("Infiltrator II", None),
        item_type("Ogre II", None),
        item_type("Myrmidon", None),
    ]);
    assert_eq!(entities(&app).await, expected);
}

/// A client error from ESI caches nothing; the upload goes through.
#[tokio::test]
async fn an_esi_client_error_caches_nothing() {
    let app = TestApp::new().await;
    app.esi.fail_with(404);
    app.upload_log("combat.txt", &fixture("testlog")).await;
    assert_eq!(app.esi.requests().len(), 1);
    assert_eq!(app.count("combat_events").await, 120);
    assert!(entities(&app).await.is_empty());
}

/// ESI being down costs two quick tries, not a held upload.
#[tokio::test]
async fn esi_being_down_does_not_hold_the_upload() {
    let mut app = TestApp::new().await;
    app.esi.fail_with(500);
    let started = Instant::now();
    app.upload_log("combat.txt", &fixture("testlog")).await;
    assert!(started.elapsed().as_secs_f64() < 2.0);
    assert_eq!(app.esi.requests().len(), 2);
    assert_eq!(app.count("combat_events").await, 120);
    assert!(entities(&app).await.is_empty());
    // With the production settings (two tries, 250 ms apart) too.
    app.reconfigure(|config| {
        config.esi.retry_delay = loganalyzer::config::DEFAULT_ESI_RETRY_DELAY;
    });
    let started = Instant::now();
    app.upload_log("combat.txt", &fixture("testlog")).await;
    assert!(started.elapsed().as_secs_f64() < 2.0);
}

/// The stored events of a log, in order.
async fn events(app: &TestApp) -> Vec<Value> {
    sqlx::query_scalar(
        "select json_build_object('timestamp', timestamp, 'damage', damage, 'direction',
            direction, 'player_name', player_name, 'corporation', corporation, 'ship_name',
            ship_name, 'weapon', weapon, 'quality', quality, 'type', type)
         from combat_events order by id",
    )
    .fetch_all(&app.pool)
    .await
    .expect("events read")
}

/// Long names and weapons are stored as they are.
#[tokio::test]
async fn stores_fields_of_any_length() {
    let app = TestApp::new().await;
    app.upload_log("long.txt", &fixture("edge-long-fields"))
        .await;
    let stored = events(&app).await;
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0]["player_name"], "L".repeat(300));
    assert_eq!(stored[1]["weapon"], "W".repeat(256));
}

/// Amounts beyond 32 bits are stored; ones beyond 64 bits saturate.
#[tokio::test]
async fn stores_huge_amounts() {
    let app = TestApp::new().await;
    app.upload_log("huge.txt", &fixture("edge-huge-numbers"))
        .await;
    let damages: Vec<Value> = events(&app)
        .await
        .into_iter()
        .map(|event| event["damage"].clone())
        .collect();
    assert_eq!(
        damages,
        [
            json!(4294967295_i64),
            json!(4294967296_i64),
            json!(i64::MAX),
            json!(i64::MAX),
            json!(123),
            json!(i64::MAX)
        ]
    );
}

#[tokio::test]
async fn stores_a_drone_owner_with_a_numeric_name() {
    let app = TestApp::new().await;
    app.upload_log("numeric.txt", &fixture("edge-numeric-pilot"))
        .await;
    assert_eq!(events(&app).await[1]["player_name"], "12345");
}

#[tokio::test]
async fn stores_a_rich_energy_line_without_a_ship() {
    let app = TestApp::new().await;
    app.upload_log("neut.txt", &fixture("edge-rich-neut-without-ship"))
        .await;
    let stored = events(&app).await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0]["player_name"], "DarkOldMaker");
    assert_eq!(stored[0]["ship_name"], Value::Null);
}

/// A post another site sent is refused; one without browser headers
/// (a script) cannot be forged by a page and goes through.
#[tokio::test]
async fn refuses_an_upload_from_another_site() {
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
        assert_json_eq(&json!({ "message": "Forbidden" }), &response.json());
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

/// A log with one stale event and the given raw file.
async fn stale_log(app: &TestApp, raw: Option<&[u8]>) -> uuid::Uuid {
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
    if let Some(raw) = raw {
        std::fs::write(app.raw_path(uuid), raw).expect("the raw log");
    }
    uuid
}

#[tokio::test]
async fn regenerating_parses_the_raw_log_again_and_caches_its_names() {
    let app = TestApp::new().await;
    let raw = gzip(&fixture(
        "upload-a-stored-combat-log-can-be-regenerated-from-its-raw-contents",
    ));
    let uuid = stale_log(&app, Some(&raw)).await;
    let response = app.post(&format!("/logs/{uuid}/regenerate")).await;
    assert_eq!(response.redirect(), format!("/logs/{uuid}"));
    assert_eq!(
        stored_log(&app).await,
        (
            "Hunter Hughes".to_owned(),
            "2026.07.20 22:25:40".to_owned(),
            Some("fight.txt".to_owned())
        )
    );
    assert_json_eq(
        &json!([{
            "timestamp": "2026.07.20 23:31:34",
            "damage": 382,
            "direction": "outgoing",
            "player_name": "TinkerbeI",
            "corporation": "LPPLS",
            "ship_name": "Crow",
            "weapon": "'Peace' Large Remote Armor Repairer",
            "quality": "armor repaired",
            "type": "logistics",
        }]),
        &Value::Array(events(&app).await),
    );
    assert_eq!(asked(&app), [["TinkerbeI", "Crow"]]);
    assert_eq!(
        entities(&app).await,
        [
            character("TinkerbeI", Some(93000008)),
            item_type("Crow", Some(11176))
        ]
    );
    assert_eq!(std::fs::read(app.raw_path(uuid)).ok(), Some(raw));
}

/// A damaged raw file is a 422 with a message; the log stays as it was.
#[tokio::test]
async fn regenerating_a_damaged_raw_log_changes_nothing() {
    let app = TestApp::new().await;
    let uuid = stale_log(&app, Some(b"this is not gzip")).await;
    let response = app.post(&format!("/logs/{uuid}/regenerate")).await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_json_eq(
        &json!({ "message": "The stored combat log could not be decoded." }),
        &response.json(),
    );
    assert_eq!(
        stored_log(&app).await,
        (
            "Stale Listener".to_owned(),
            "2025.01.01 00:00:00".to_owned(),
            Some("fight.txt".to_owned())
        )
    );
    assert_eq!(events(&app).await.len(), 1);
    assert_eq!(
        std::fs::read(app.raw_path(uuid)).ok().as_deref(),
        Some(b"this is not gzip".as_slice())
    );
    assert!(app.esi.requests().is_empty());
}

#[tokio::test]
async fn regenerating_without_a_raw_log_is_not_found() {
    let app = TestApp::new().await;
    let uuid = stale_log(&app, None).await;
    let response = app.post(&format!("/logs/{uuid}/regenerate")).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_json_eq(&json!({ "message": "Not Found" }), &response.json());
    assert_eq!(stored_log(&app).await.0, "Stale Listener");
    assert_eq!(events(&app).await.len(), 1);
}

#[tokio::test]
async fn regenerating_an_unknown_log_is_not_found() {
    let app = TestApp::new().await;
    let path = "/logs/00000000-0000-4000-8000-000000000000/regenerate";
    let response = app.post(path).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_json_eq(&json!({ "message": "Not Found" }), &response.json());
    // A plain form post gets the error page.
    let page = app
        .request(
            Method::POST,
            path,
            header_map(&[("host", HOST), ("origin", APP_URL)]),
            Body::empty(),
        )
        .await;
    assert_eq!(page.status, StatusCode::NOT_FOUND);
    assert_eq!(
        page.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert!(page.text().contains("<title>Not Found</title>"));
}

#[tokio::test]
async fn refuses_a_regeneration_from_another_site() {
    let app = TestApp::new().await;
    let uuid = stale_log(&app, None).await;
    let response = app
        .request(
            Method::POST,
            &format!("/logs/{uuid}/regenerate"),
            header_map(&[("host", HOST), ("origin", "https://evil.example")]),
            Body::empty(),
        )
        .await;
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert_eq!(stored_log(&app).await.0, "Stale Listener");
}
