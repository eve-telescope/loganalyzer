//! The combat log parser: an EVE Online gamelog in, the listener, the
//! session start and the combat events out. The patterns are PHP's
//! `preg_*` patterns as they were, run by PCRE2 with PHP's `/u` options
//! (UTF and UCP), so lazy quantifiers, the lookahead of the event
//! splitter and `$` before a final newline behave the same. The input is
//! the uploaded bytes as they are; bytes that are not UTF-8 match nothing
//! (listener `Unknown`, no events), as PHP's functions return false on
//! them.
//!
//! tests/integration/parser.rs compares the output with what the PHP
//! parser answered for every fixture and a generated corpus.

pub mod php;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Outgoing,
    Incoming,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Outgoing => "outgoing",
            Direction::Incoming => "incoming",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Damage,
    Logistics,
    Neutralization,
}

impl EventType {
    pub fn as_str(self) -> &'static str {
        match self {
            EventType::Damage => "damage",
            EventType::Logistics => "logistics",
            EventType::Neutralization => "neutralization",
        }
    }
}

/// One combat event, in the shape the analysis page reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// The log's own time, `YYYY.MM.DD HH:MM:SS`.
    pub timestamp: String,
    /// Hit points, or GJ for energy warfare; never negative.
    pub damage: i64,
    pub direction: Direction,
    /// The other party: the target of outgoing events, the source of
    /// incoming ones (the drone's owner when it can be told).
    pub player_name: String,
    pub corporation: Option<String>,
    pub ship_name: Option<String>,
    pub weapon: String,
    pub quality: String,
    #[serde(rename = "type")]
    pub kind: EventType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedLog {
    pub listener: String,
    pub session_started: String,
    pub events: Vec<Event>,
}

/// Parses a whole gamelog.
pub fn parse(_contents: &[u8]) -> ParsedLog {
    unimplemented!("the parser is not ported yet")
}
