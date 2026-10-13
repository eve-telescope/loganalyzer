//! The parser: single lines and the full output for every fixture in
//! tests/fixtures/parser (`<name>.txt` against `<name>.expected.json`)
//! and for the generated corpus of small logs.

use loganalyzer::parser::{Direction, Event, EventType, ParsedLog, parse};
use serde_json::Value;

use crate::common::{assert_json_eq, repo_path};

fn fixture(name: &str) -> Vec<u8> {
    let path = repo_path(&format!("tests/fixtures/parser/{name}.txt"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn expected(name: &str) -> Value {
    let path = repo_path(&format!("tests/fixtures/parser/{name}.expected.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).expect("expected JSON")
}

fn parsed(name: &str) -> ParsedLog {
    parse(&fixture(name))
}

fn testlog() -> ParsedLog {
    parsed("testlog")
}

fn only_event(name: &str) -> Event {
    let events = parsed(name).events;
    assert_eq!(events.len(), 1, "{events:#?}");
    events.into_iter().next().expect("one event")
}

#[test]
fn it_extracts_the_listener_name_from_the_header() {
    assert_eq!(testlog().listener, "Nicolas Kion");
}

#[test]
fn it_extracts_the_session_start_time() {
    assert_eq!(testlog().session_started, "2026.02.12 09:41:44");
}

#[test]
fn it_parses_combat_events() {
    assert!(!testlog().events.is_empty());
}

#[test]
fn it_parses_outgoing_damage_hits_correctly() {
    let event = only_event("parser-it-parses-outgoing-damage-hits-correctly");
    assert_eq!(event.damage, 500);
    assert_eq!(event.direction, Direction::Outgoing);
    assert_eq!(event.player_name, "Enemy Pilot");
    assert_eq!(event.ship_name.as_deref(), Some("Drake"));
    assert_eq!(event.weapon, "Heavy Missile");
    assert_eq!(event.kind, EventType::Damage);
}

#[test]
fn it_parses_incoming_miss_lines() {
    let event = only_event("parser-it-parses-incoming-miss-lines");
    assert_eq!(event.damage, 0);
    assert_eq!(event.direction, Direction::Incoming);
    assert_eq!(event.quality, "Misses");
}

#[test]
fn it_parses_outgoing_miss_lines() {
    let event = &parsed("parser-it-parses-outgoing-miss-lines").events[0];
    assert_eq!(event.direction, Direction::Outgoing);
    assert_eq!(event.quality, "Misses");
}

#[test]
fn it_handles_empty_combat_logs_gracefully() {
    assert!(
        parsed("parser-it-handles-empty-combat-logs-gracefully")
            .events
            .is_empty()
    );
}

#[test]
fn it_parses_logistics_received_events() {
    let event = only_event("parser-it-parses-logistics-received-events");
    assert_eq!(event.damage, 575);
    assert_eq!(event.direction, Direction::Incoming);
    assert_eq!(event.kind, EventType::Logistics);
    assert_eq!(event.player_name, "Logi Pilot");
    assert_eq!(event.ship_name.as_deref(), Some("Basilisk"));
    assert_eq!(event.weapon, "Large Remote Shield Booster");
}

#[test]
fn it_parses_logi_events_from_the_test_log_file() {
    let logi = testlog()
        .events
        .iter()
        .filter(|event| event.kind == EventType::Logistics)
        .count();
    assert!(logi > 0);
}

#[test]
fn it_parses_incoming_energy_neutralization_events() {
    let event = only_event("parser-it-parses-incoming-energy-neutralization-events");
    assert_eq!(event.damage, 522);
    assert_eq!(event.direction, Direction::Incoming);
    assert_eq!(event.kind, EventType::Neutralization);
    assert_eq!(event.ship_name.as_deref(), Some("Sabre"));
    assert_eq!(event.weapon, "True Sansha Heavy Energy Neutralizer");
    assert_eq!(event.quality, "Neutralized");
}

#[test]
fn it_parses_incoming_energy_nosferatu_events_with_negative_gj() {
    let event = only_event("parser-it-parses-incoming-energy-nosferatu-events-with-negative-gj");
    assert_eq!(event.damage, 2);
    assert_eq!(event.direction, Direction::Incoming);
    assert_eq!(event.kind, EventType::Neutralization);
    assert_eq!(event.ship_name.as_deref(), Some("Stiletto"));
    assert_eq!(event.weapon, "Small Ghoul Compact Energy Nosferatu");
    assert_eq!(event.quality, "Drained");
}

#[test]
fn it_parses_neutralization_events_from_the_testlog2_fixture() {
    let neuts = parsed("testlog2")
        .events
        .iter()
        .filter(|event| event.kind == EventType::Neutralization)
        .count();
    assert!(neuts > 800, "{neuts}");
}

#[test]
fn events_use_enums_for_direction_and_type() {
    // The types make anything else impossible; the stored form is the
    // enum's value.
    for event in testlog().events {
        assert!(matches!(event.direction.as_str(), "outgoing" | "incoming"));
        assert!(matches!(
            event.kind.as_str(),
            "damage" | "logistics" | "neutralization"
        ));
    }
}

#[test]
fn it_attributes_outgoing_drone_damage_to_the_sole_owner() {
    let events = parsed("parser-it-attributes-outgoing-drone-damage-to-the-sole-owner").events;
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].player_name, "Tom DaNanRen");
    assert_eq!(events[1].ship_name.as_deref(), Some("Hammerhead II"));
    assert_eq!(events[1].direction, Direction::Outgoing);
}

#[test]
fn it_leaves_drone_damage_unattributed_when_several_pilots_own_that_drone_type() {
    let events = parsed(
        "parser-it-leaves-drone-damage-unattributed-when-several-pilots-own-that-drone-type",
    )
    .events;
    assert_eq!(events[2].player_name, "Warrior II");
}

#[test]
fn it_strips_markup_from_logistics_dealt_weapons() {
    let event = only_event("parser-it-strips-markup-from-logistics-dealt-weapons");
    assert_eq!(event.weapon, "Medium Remote Shield Booster");
    assert_eq!(event.player_name, "Fleet Mate");
    assert_eq!(event.kind, EventType::Logistics);
}

#[test]
fn it_parses_multiline_rich_format_logistics_dealt_events() {
    let event = only_event("parser-it-parses-multiline-rich-format-logistics-dealt-events");
    assert_eq!(event.kind, EventType::Logistics);
    assert_eq!(event.direction, Direction::Outgoing);
    assert_eq!(event.damage, 382);
    assert_eq!(event.player_name, "TinkerbeI");
    assert_eq!(event.corporation.as_deref(), Some("LPPLS"));
    assert_eq!(event.ship_name.as_deref(), Some("Crow"));
    assert_eq!(event.weapon, "'Peace' Large Remote Armor Repairer");
    assert_eq!(event.quality, "armor repaired");
}

#[test]
fn no_parsed_field_ever_contains_markup() {
    for event in testlog().events {
        assert!(!event.player_name.contains('<'));
        assert!(!event.weapon.contains('<'));
        assert!(!event.ship_name.unwrap_or_default().contains('<'));
    }
}

#[test]
fn it_parses_logistics_received_with_a_plain_pilot_target_issue_1() {
    let event = only_event("parser-it-parses-logistics-received-with-a-plain-pilot-target-issue-1");
    assert_eq!(event.kind, EventType::Logistics);
    assert_eq!(event.direction, Direction::Incoming);
    assert_eq!(event.damage, 512);
    assert_eq!(event.player_name, "Logi Pilot");
    assert_eq!(event.corporation.as_deref(), Some("CORP"));
    assert_eq!(event.ship_name.as_deref(), Some("Basilisk"));
    assert_eq!(event.weapon, "Large Remote Shield Booster");
}

#[test]
fn it_parses_rich_format_outgoing_energy_neutralization_with_pilot_details() {
    let event = only_event(
        "parser-it-parses-rich-format-outgoing-energy-neutralization-with-pilot-details",
    );
    assert_eq!(event.kind, EventType::Neutralization);
    assert_eq!(event.direction, Direction::Outgoing);
    assert_eq!(event.damage, 27);
    assert_eq!(event.player_name, "DarkOldMaker");
    assert_eq!(event.corporation.as_deref(), Some("TLOS"));
    assert_eq!(event.ship_name.as_deref(), Some("Huginn"));
    assert_eq!(event.weapon, "Heavy Abyssal Energy Neutralizer");
    assert_eq!(event.quality, "Neutralized");
}

#[test]
fn it_parses_rich_format_outgoing_nosferatu_drains_with_positive_gj() {
    let event =
        only_event("parser-it-parses-rich-format-outgoing-nosferatu-drains-with-positive-gj");
    assert_eq!(event.direction, Direction::Outgoing);
    assert_eq!(event.damage, 269);
    assert_eq!(event.player_name, "Daniel Grosny");
    assert_eq!(event.ship_name.as_deref(), Some("Basilisk"));
    assert_eq!(event.quality, "Drained");
}

#[test]
fn it_parses_incoming_neutralization_with_fontsize_wrapped_ship_names() {
    let event =
        only_event("parser-it-parses-incoming-neutralization-with-fontsize-wrapped-ship-names");
    assert_eq!(event.direction, Direction::Incoming);
    assert_eq!(event.damage, 3345);
    assert_eq!(event.player_name, "Keepstar");
    assert_eq!(
        event.weapon,
        "Standup Energy Neutralization Burst Projector"
    );
}

#[test]
fn it_parses_outgoing_neutralization_from_the_testlog3_fixture() {
    let outgoing = parsed("testlog3")
        .events
        .iter()
        .filter(|event| {
            event.kind == EventType::Neutralization && event.direction == Direction::Outgoing
        })
        .count();
    assert!(outgoing > 300, "{outgoing}");
}

// Lines with odd parts.

#[test]
fn a_rich_energy_line_without_a_ship_keeps_the_event() {
    let event = only_event("edge-rich-neut-without-ship");
    assert_eq!(event.player_name, "DarkOldMaker");
    assert_eq!(event.corporation.as_deref(), Some("TLOS"));
    assert_eq!(event.ship_name, None);
    assert_eq!(event.direction, Direction::Outgoing);
    assert_eq!(event.damage, 27);
}

#[test]
fn a_rich_energy_line_whose_pilot_sits_in_markup_reads_as_the_plain_format() {
    let event = only_event("edge-rich-neut-pilot-in-tag");
    assert_eq!(event.player_name, "Huginn");
    assert_eq!(event.ship_name.as_deref(), Some("Huginn"));
    assert_eq!(event.direction, Direction::Incoming);
}

#[test]
fn a_drone_owner_with_a_numeric_name_is_attributed() {
    let events = parsed("edge-numeric-pilot").events;
    assert_eq!(events[1].player_name, "12345");
    assert_eq!(events[1].ship_name.as_deref(), Some("Hammerhead II"));
}

#[test]
fn an_energy_amount_below_i64_min_saturates() {
    let events = parsed("edge-huge-numbers").events;
    let last = events.last().expect("events");
    assert_eq!(last.damage, i64::MAX);
}

#[test]
fn a_corporation_or_ship_named_0_is_kept() {
    let events = parsed("edge-corp-zero").events;
    assert_eq!(events[0].corporation.as_deref(), Some("0"));
    assert_eq!(events[3].corporation.as_deref(), Some("0"));
    let event = only_event("edge-rich-neut-zero-ship");
    assert_eq!(event.ship_name.as_deref(), Some("0"));
}

// Every fixture against its expected output.

#[test]
fn every_fixture_parses_to_its_expected_output() {
    let directory = repo_path("tests/fixtures/parser");
    let mut names: Vec<String> = std::fs::read_dir(&directory)
        .expect("fixtures")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".txt").map(str::to_owned)
        })
        .collect();
    names.sort();
    assert!(names.len() > 50, "{names:?}");
    for name in names {
        let wanted = expected(&name);
        let actual = serde_json::to_value(parsed(&name)).expect("JSON");
        if let Some(difference) = crate::common::json_difference(&name, &wanted, &actual) {
            panic!("{difference}");
        }
    }
}

#[test]
fn the_generated_corpus_parses_to_its_expected_output() {
    let read = |file: &str| -> Value {
        let path = repo_path(&format!("tests/fixtures/parser/generated/{file}"));
        serde_json::from_str(&std::fs::read_to_string(path).expect("corpus")).expect("JSON")
    };
    let inputs = read("inputs.json");
    let outputs = read("expected.json");
    let inputs = inputs.as_object().expect("inputs");
    assert_eq!(inputs.len(), 400);
    for (name, encoded) in inputs {
        let bytes = base64_decode(encoded.as_str().expect("base64"));
        let actual = serde_json::to_value(parse(&bytes)).expect("JSON");
        let wanted = &outputs[name];
        assert_json_eq(wanted, &actual);
    }
}

/// Standard base64, as the corpus files store the logs.
pub fn base64_decode(text: &str) -> Vec<u8> {
    let value = |byte: u8| -> u32 {
        match byte {
            b'A'..=b'Z' => u32::from(byte - b'A'),
            b'a'..=b'z' => u32::from(byte - b'a') + 26,
            b'0'..=b'9' => u32::from(byte - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("not base64: {byte}"),
        }
    };
    let bytes: Vec<u8> = text.bytes().filter(|byte| *byte != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut buffer = 0u32;
        for (index, byte) in chunk.iter().enumerate() {
            buffer |= value(*byte) << (18 - 6 * index);
        }
        let produced = chunk.len() * 6 / 8;
        for index in 0..produced {
            out.push((buffer >> (16 - 8 * index)) as u8);
        }
    }
    out
}
