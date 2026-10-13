//! The ESI client and the name cache against the mock ESI: the request it
//! sends, the retry policy (two tries, then the chunk is skipped; FIXES.md
//! "ESI being down held an upload for 20 seconds"), chunks of 500 names,
//! and the cache's spelling rules.

use std::time::{Duration, Instant};

use serde_json::json;

use crate::common::{TestApp, USER_AGENT, testlog};

/// A log naming `count` distinct pilots.
fn log_with_pilots(count: usize) -> Vec<u8> {
    let mut log = String::from("  Listener: Test Pilot\n");
    for index in 0..count {
        log.push_str(&format!(
            "[ 2026.01.01 12:00:00 ] (combat) <color=0xff00ffff><b>1</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Pilot {index:04}</b><font size=10><color=0x77ffffff> - Gun - Hits\n"
        ));
    }
    log.into_bytes()
}

#[tokio::test]
async fn sends_the_names_with_the_user_agent_and_compatibility_date() {
    let app = TestApp::new().await;
    app.upload_log("combat.txt", &testlog()).await;
    let requests = app.esi.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].user_agent.as_deref(), Some(USER_AGENT));
    assert_eq!(
        requests[0].compatibility_date.as_deref(),
        Some("2026-05-19")
    );
    assert_eq!(
        requests[0].content_type.as_deref(),
        Some("application/json")
    );
}

#[tokio::test]
async fn the_default_user_agent_names_the_site_and_its_source() {
    assert_eq!(
        loganalyzer::config::default_user_agent("https://loganalyzer.eve-telescope.com"),
        USER_AGENT
    );
}

#[tokio::test]
async fn retries_a_server_error_once() {
    let app = TestApp::new().await;
    app.esi.fail_next(&[502]);
    app.upload_log("combat.txt", &testlog()).await;
    assert_eq!(app.esi.requests().len(), 2);
    assert_eq!(app.eve_id(None, "Aria Vex").await, Some(93000001));
}

#[tokio::test]
async fn gives_up_after_two_tries_and_leaves_the_names_uncached() {
    let app = TestApp::new().await;
    app.esi.fail_with(503);
    app.upload_log("combat.txt", &testlog()).await;
    assert_eq!(app.esi.requests().len(), 2);
    assert_eq!(app.count("eve_entities").await, 0);
}

#[tokio::test]
async fn does_not_retry_a_client_error() {
    let app = TestApp::new().await;
    app.esi.fail_with(400);
    app.upload_log("combat.txt", &testlog()).await;
    assert_eq!(app.esi.requests().len(), 1);
    assert_eq!(app.count("eve_entities").await, 0);
}

#[tokio::test]
async fn an_unreachable_esi_costs_two_quick_tries() {
    let mut app = TestApp::new().await;
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        listener.local_addr().expect("an address")
    };
    app.reconfigure(|config| config.esi.base_url = format!("http://{closed}"));
    let started = Instant::now();
    app.upload_log("combat.txt", &testlog()).await;
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(app.count("eve_entities").await, 0);
    assert_eq!(app.count("combat_logs").await, 1);
}

#[tokio::test]
async fn a_slow_answer_times_out_and_is_retried() {
    let mut app = TestApp::new().await;
    app.reconfigure(|config| config.esi.timeout = Duration::from_millis(100));
    app.esi.delay(Duration::from_millis(400));
    let started = Instant::now();
    app.upload_log("combat.txt", &testlog()).await;
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(app.esi.requests().len(), 2);
    assert_eq!(app.count("eve_entities").await, 0);
}

#[tokio::test]
async fn an_answer_that_is_not_the_documented_shape_skips_the_chunk() {
    let app = TestApp::new().await;
    app.esi.answer_with("null");
    app.upload_log("combat.txt", &testlog()).await;
    assert_eq!(app.count("eve_entities").await, 0);
    assert_eq!(app.count("combat_logs").await, 1);
}

#[tokio::test]
async fn asks_for_at_most_500_names_at_a_time() {
    let app = TestApp::new().await;
    app.upload_log("many.txt", &log_with_pilots(1201)).await;
    let requests = app.esi.requests();
    let sizes: Vec<usize> = requests.iter().map(|request| request.names.len()).collect();
    assert_eq!(sizes, [500, 500, 201]);
    assert_eq!(requests[0].names[0], "Pilot 0000");
    assert_eq!(requests[2].names[200], "Pilot 1200");
    // Every name is cached, misses included.
    assert_eq!(app.count("eve_entities").await, 1201);
}

#[tokio::test]
async fn a_failing_chunk_does_not_stop_the_others() {
    let app = TestApp::new().await;
    app.esi.fail_next(&[500, 500]);
    app.upload_log("many.txt", &log_with_pilots(600)).await;
    let sizes: Vec<usize> = app
        .esi
        .requests()
        .iter()
        .map(|request| request.names.len())
        .collect();
    assert_eq!(sizes, [500, 500, 100]);
    assert_eq!(app.count("eve_entities").await, 100);
}

#[tokio::test]
async fn names_in_esi_answers_are_unescaped_and_normalized() {
    let app = TestApp::new().await;
    // ESI sometimes answers literal `\u` escapes, and decomposed letters.
    app.esi.answer_with(
        r#"{"characters": [{"id": 1, "name": "Zo\\u00eb \\u00c5ngstr\\u00f6m"}, {"id": 2, "name": "Céline"}]}"#,
    );
    let log = "  Listener: X\n\
        [ 2026.01.01 12:00:00 ] (combat) Your Gun misses Zoë Ångström completely - Gun\n\
        [ 2026.01.01 12:00:01 ] (combat) Your Gun misses C\u{e9}line completely - Gun\n";
    app.upload_log("combat.txt", log.as_bytes()).await;
    assert_eq!(app.eve_id(Some("character"), "Zoë Ångström").await, Some(1));
    assert_eq!(app.eve_id(Some("character"), "C\u{e9}line").await, Some(2));
}

#[tokio::test]
async fn a_cached_name_counts_only_in_its_stored_spelling() {
    // Laravel compared the cache's spelling exactly, so another spelling
    // asks ESI again and updates the stored row (which keeps its
    // spelling).
    let app = TestApp::new().await;
    app.create_entity("character", "aria vex", None).await;
    app.upload_log("combat.txt", &testlog()).await;
    assert!(
        app.esi.requests()[0]
            .names
            .iter()
            .any(|name| name == "Aria Vex")
    );
    let rows: Vec<(String, Option<i64>)> = sqlx::query_as(
        "select name, eve_id from eve_entities where kind = 'character' and name = 'ARIA VEX'",
    )
    .fetch_all(&app.pool)
    .await
    .expect("rows");
    assert_eq!(rows, [("aria vex".to_owned(), Some(93000001))]);
}

#[tokio::test]
async fn two_spellings_in_one_log_share_a_row() {
    let app = TestApp::new().await;
    app.esi
        .set_table(json!({ "characters": { "Aria Vex": 1 } }));
    let log = "  Listener: X\n\
        [ 2026.01.01 12:00:00 ] (combat) Your Gun misses Aria Vex completely - Gun\n\
        [ 2026.01.01 12:00:01 ] (combat) Your Gun misses aria vex completely - Gun\n\
        [ 2026.01.01 12:00:02 ] (combat) Your Gun misses Ária Vex completely - Gun\n";
    app.upload_log("combat.txt", log.as_bytes()).await;
    let rows: Vec<(String, Option<i64>)> =
        sqlx::query_as("select name, eve_id from eve_entities order by id")
            .fetch_all(&app.pool)
            .await
            .expect("rows");
    // The mock matches case-insensitively but not across accents; the
    // last spelling's answer wins, as in MySQL's upsert.
    assert_eq!(rows, [("Aria Vex".to_owned(), None)]);
}
