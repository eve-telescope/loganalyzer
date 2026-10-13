//! The combat log parser: an EVE Online gamelog in, the listener, the
//! session start and the combat events out. The patterns run on PCRE2
//! in UTF and UCP mode, which decides how lazy quantifiers, the
//! lookahead of the event splitter and `$` before a final newline behave.
//! The input is the uploaded bytes as they are; a log that is not UTF-8
//! matches nothing (listener `Unknown`, no events).
//!
//! tests/integration/parser.rs pins the output for every fixture and a
//! generated corpus.

pub mod text;

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use pcre2::bytes::{Captures, Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use text::{clean, leading_int, strip_tags};

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

fn regex(pattern: &str, multi_line: bool, dotall: bool) -> Regex {
    RegexBuilder::new()
        .utf(true)
        .ucp(true)
        .multi_line(multi_line)
        .dotall(dotall)
        .jit_if_available(true)
        .build(pattern)
        .unwrap_or_else(|error| panic!("{pattern}: {error}"))
}

/// `/…/u`
fn u(pattern: &str) -> Regex {
    regex(pattern, false, false)
}

static LISTENER: LazyLock<Regex> = LazyLock::new(|| regex(r"^\s*Listener:\s*(.+)$", true, false));
static SESSION_STARTED: LazyLock<Regex> =
    LazyLock::new(|| regex(r"^\s*Session Started:\s*(.+)$", true, false));
static ENTRY: LazyLock<Regex> = LazyLock::new(|| {
    regex(
        r"^\[ (\d{4}\.\d{2}\.\d{2} \d{2}:\d{2}:\d{2}) \] \(combat\) (.*?)(?=^\[ \d{4}\.\d{2}\.\d{2} \d{2}:\d{2}:\d{2} \] |\z)",
        true,
        true,
    )
});
static LOGI_RECEIVED_RICH: LazyLock<Regex> = LazyLock::new(|| {
    u(
        r"<b>(\d+)</b>.*?remote (shield|armor|hull) (boosted|repaired) by.*?<b>([^<]+)</b>.*?<font size=10>([^<]+)</font></b>.*? - (.+?)(?:</font>)?$",
    )
});
static LOGI_RECEIVED_PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    u(
        r"<b>(\d+)</b>.*?remote (shield|armor|hull) (boosted|repaired) by.*?<b><color=0xffffffff>(.+?)</b>.*? - (.+?)(?:</font>)?$",
    )
});
static LOGI_DEALT_RICH: LazyLock<Regex> = LazyLock::new(|| {
    regex(
        r"<b>(\d+)</b>.*?remote (shield|armor|hull) (boosted|repaired) to.*?<fontsize=12>([^<\r\n]+).*?<fontsize=10>([^<]+)</color>.*?<b>(?:\[([^\]]+)\]|&lt;([^&]+)&gt;)</b>.*? - (.+?)(?:</font>)?\s*$",
        false,
        true,
    )
});
static LOGI_DEALT_PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    u(
        r"<b>(\d+)</b>.*?remote (shield|armor|hull) (boosted|repaired) to.*?<b><color=0xffffffff>(.+?)</b>.*? - (.+)$",
    )
});
static ENERGY_RICH: LazyLock<Regex> = LazyLock::new(|| {
    u(
        r"<b>([+-]?\d+) GJ</b>.*?energy (neutralized|drained)(?: (to|from))?\s*</font><b>(.*?\[[^\]]+\] ?-\s*(?:</font>)?)</b>.*? - (.+?)</font>",
    )
});
static ENERGY_PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    u(
        r"<b>([+-]?\d+) GJ</b>.*?energy (neutralized|drained)(?: (to|from))? .*?(?:<b>([^<]+)</b>|<fontsize=\d+>([^<]+)</fontsize>).*? - (.+?)</font>",
    )
});
static DAMAGE: LazyLock<Regex> = LazyLock::new(|| {
    u(
        r"<b>(\d+)</b>.*?<font size=10>(to|from)</font>.*?<b><color=0xffffffff>(.+?)</b>.*? - (.+?) - (Hits|Penetrates|Glances Off|Grazes|Smashes|Wrecks)",
    )
});
static MISS_INCOMING: LazyLock<Regex> =
    LazyLock::new(|| u(r"^(.+?) belonging to (.+?) misses you completely - (.+)$"));
static MISS_OUTGOING: LazyLock<Regex> =
    LazyLock::new(|| u(r"^Your (.+?) misses (.+?) completely - (.+)$"));
static BRACKETS: LazyLock<Regex> = LazyLock::new(|| u(r"\[([^\]]+)\]"));
static ENERGY_SHIP: LazyLock<Regex> =
    LazyLock::new(|| u(r"<b>([^<]*)</b>|<fontsize=\d+>([^<]*)</fontsize>"));
static TARGET: LazyLock<Regex> = LazyLock::new(|| u(r"^(.+?)\[([^\]]*)\]\(([^)]+)\)$"));

/// The first match, or nothing (no match, or a PCRE2 error such as
/// a subject that is not UTF-8).
fn first<'s>(regex: &Regex, subject: &'s [u8]) -> Option<Captures<'s>> {
    regex.captures(subject).ok().flatten()
}

/// A group's bytes; a group that did not take part is empty.
fn group<'s>(captures: &Captures<'s>, index: usize) -> &'s [u8] {
    captures
        .get(index)
        .map_or(&[][..], |found| found.as_bytes())
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Parses a whole gamelog.
pub fn parse(contents: &[u8]) -> ParsedLog {
    // A log that is not UTF-8 matches nothing. Every later subject is a
    // part of this one cut at character boundaries, so it is checked once.
    if std::str::from_utf8(contents).is_err() {
        return ParsedLog {
            listener: "Unknown".to_owned(),
            session_started: String::new(),
            events: Vec::new(),
        };
    }
    let listener =
        first(&LISTENER, contents).map_or_else(|| "Unknown".to_owned(), |c| clean(group(&c, 1)));
    let session_started =
        first(&SESSION_STARTED, contents).map_or_else(String::new, |c| clean(group(&c, 1)));
    ParsedLog {
        listener,
        session_started,
        events: attribute_drone_damage(parse_events(contents)),
    }
}

fn parse_events(contents: &[u8]) -> Vec<Event> {
    let mut events = Vec::new();
    for entry in ENTRY.captures_iter(contents) {
        // An error mid-way ends the list with the matches so far.
        let Ok(entry) = entry else { break };
        if let Some(event) = parse_combat_line(&text(group(&entry, 1)), group(&entry, 2)) {
            events.push(event);
        }
    }
    events
}

/// A quality like `shield boosted`.
fn logi_quality(captures: &Captures<'_>) -> String {
    format!(
        "{} {}",
        clean(group(captures, 2)),
        clean(group(captures, 3))
    )
}

fn energy_quality(verb: &[u8]) -> String {
    if verb == b"neutralized" {
        "Neutralized"
    } else {
        "Drained"
    }
    .to_owned()
}

/// The amount of an energy line, without its sign; `i64::MIN` saturates
/// to `i64::MAX`.
fn energy_amount(amount: &[u8]) -> i64 {
    leading_int(&text(amount)).saturating_abs()
}

fn parse_combat_line(timestamp: &str, content: &[u8]) -> Option<Event> {
    let event = |damage: i64,
                 direction: Direction,
                 player_name: String,
                 corporation: Option<String>,
                 ship_name: Option<String>,
                 weapon: String,
                 quality: String,
                 kind: EventType| Event {
        timestamp: timestamp.to_owned(),
        damage,
        direction,
        player_name,
        corporation,
        ship_name,
        weapon,
        quality,
        kind,
    };

    // Logistics received, the target in separate ship and pilot markup.
    if let Some(m) = first(&LOGI_RECEIVED_RICH, content) {
        return Some(event(
            leading_int(&text(group(&m, 1))),
            Direction::Incoming,
            clean(group(&m, 5)),
            None,
            Some(clean(group(&m, 4))),
            clean(group(&m, 6)),
            logi_quality(&m),
            EventType::Logistics,
        ));
    }

    // Logistics received, a plain `Pilot[CORP](Ship)` target (issue #1).
    if let Some(m) = first(&LOGI_RECEIVED_PLAIN, content) {
        let target = parse_target(&clean(group(&m, 4)));
        return Some(event(
            leading_int(&text(group(&m, 1))),
            Direction::Incoming,
            target.name,
            target.corporation,
            target.ship,
            clean(group(&m, 5)),
            logi_quality(&m),
            EventType::Logistics,
        ));
    }

    // Logistics dealt, with separate ship, pilot and corporation markup
    // (the only pattern where `.` crosses lines).
    if let Some(m) = first(&LOGI_DEALT_RICH, content) {
        let ticker = group(&m, 6);
        let corporation = clean(if ticker.is_empty() {
            group(&m, 7)
        } else {
            ticker
        });
        return Some(event(
            leading_int(&text(group(&m, 1))),
            Direction::Outgoing,
            clean(group(&m, 5)),
            Some(corporation),
            Some(clean(group(&m, 4))),
            clean(group(&m, 8)),
            logi_quality(&m),
            EventType::Logistics,
        ));
    }

    // Logistics dealt, plain target (kept whole, not split).
    if let Some(m) = first(&LOGI_DEALT_PLAIN, content) {
        return Some(event(
            leading_int(&text(group(&m, 1))),
            Direction::Outgoing,
            clean(group(&m, 4)),
            None,
            None,
            clean(group(&m, 5)),
            logi_quality(&m),
            EventType::Logistics,
        ));
    }

    // Energy neutralization and nosferatu, the victim named in full
    // ("Ship [ALLIANCE] [CORP] [Pilot] -"), which only appears when the
    // listener is the aggressor. A target whose brackets all sit inside
    // markup names no pilot; it is read as the plain format below.
    if let Some(m) = first(&ENERGY_RICH, content) {
        let target_html = group(&m, 4);
        let stripped = strip_tags(target_html);
        let mut tickers: Vec<&[u8]> = BRACKETS
            .captures_iter(&stripped)
            .filter_map(Result::ok)
            .map(|c| group(&c, 1))
            .collect();
        if let Some(pilot) = tickers.pop() {
            let ship = first(&ENERGY_SHIP, target_html).map_or_else(String::new, |s| {
                let bold = group(&s, 1);
                clean(if bold.is_empty() { group(&s, 2) } else { bold })
            });
            return Some(event(
                energy_amount(group(&m, 1)),
                Direction::Outgoing,
                clean(pilot),
                tickers.last().map(|corporation| clean(corporation)),
                (!ship.is_empty()).then_some(ship),
                clean(group(&m, 5)),
                energy_quality(group(&m, 2)),
                EventType::Neutralization,
            ));
        }
    }

    // Energy warfare naming only the other ship. The direction depends on
    // the verb, the preposition and the sign:
    //   "neutralized {ship}"       incoming neut
    //   "drained to {ship}"        incoming nos (negative GJ)
    //   "neutralized to {ship}"    outgoing neut (best effort)
    //   "drained from {ship}"      outgoing nos (positive GJ)
    if let Some(m) = first(&ENERGY_PLAIN, content) {
        let verb = group(&m, 2);
        let preposition = group(&m, 3);
        let bold = group(&m, 4);
        let ship = clean(if bold.is_empty() { group(&m, 5) } else { bold });
        let direction = if preposition == b"from"
            || group(&m, 1).starts_with(b"+")
            || (verb == b"neutralized" && preposition == b"to")
        {
            Direction::Outgoing
        } else {
            Direction::Incoming
        };
        return Some(event(
            energy_amount(group(&m, 1)),
            direction,
            ship.clone(),
            None,
            Some(ship),
            clean(group(&m, 6)),
            energy_quality(verb),
            EventType::Neutralization,
        ));
    }

    if let Some(m) = first(&DAMAGE, content) {
        let target = parse_target(&text(group(&m, 3)));
        let direction = if group(&m, 2) == b"to" {
            Direction::Outgoing
        } else {
            Direction::Incoming
        };
        return Some(event(
            leading_int(&text(group(&m, 1))),
            direction,
            target.name,
            target.corporation,
            target.ship,
            clean(group(&m, 4)),
            text(group(&m, 5)),
            EventType::Damage,
        ));
    }

    if let Some(m) = first(&MISS_INCOMING, content) {
        return Some(event(
            0,
            Direction::Incoming,
            clean(group(&m, 2)),
            None,
            None,
            clean(group(&m, 1)),
            "Misses".to_owned(),
            EventType::Damage,
        ));
    }

    if let Some(m) = first(&MISS_OUTGOING, content) {
        return Some(event(
            0,
            Direction::Outgoing,
            clean(group(&m, 2)),
            None,
            None,
            clean(group(&m, 1)),
            "Misses".to_owned(),
            EventType::Damage,
        ));
    }

    None
}

struct Target {
    name: String,
    corporation: Option<String>,
    ship: Option<String>,
}

/// `Pilot[CORP](Ship)`; anything else is a name on its own. An empty
/// ticker is no corporation.
fn parse_target(raw: &str) -> Target {
    match first(&TARGET, raw.as_bytes()) {
        Some(m) => {
            let corporation = clean(group(&m, 2));
            Target {
                name: clean(group(&m, 1)),
                corporation: (!corporation.is_empty()).then_some(corporation),
                ship: Some(clean(group(&m, 3))),
            }
        }
        None => Target {
            name: clean(raw.as_bytes()),
            corporation: None,
            ship: None,
        },
    }
}

/// Insertion-ordered set of names.
#[derive(Default)]
struct Names {
    order: Vec<String>,
    seen: HashSet<String>,
}

impl Names {
    fn insert(&mut self, name: &str) {
        if self.seen.insert(name.to_owned()) {
            self.order.push(name.to_owned());
        }
    }
}

/// Outgoing damage to a drone logs the drone itself as the target
/// (`Hammerhead II[.BOP](Hammerhead II)`), so the pilot slot holds a type
/// name. The owner comes from the rest of the log: pilots who appear with
/// the drone's name as their weapon, narrowed by the drone's corporation,
/// or the corporation's only pilot when nobody used that drone. With
/// several candidates the event stays as it is.
fn attribute_drone_damage(events: Vec<Event>) -> Vec<Event> {
    let mut weapon_users: HashMap<&str, Names> = HashMap::new();
    let mut corp_pilots: HashMap<&str, Names> = HashMap::new();
    for event in &events {
        if event.ship_name.as_deref() == Some(event.player_name.as_str()) {
            continue;
        }
        weapon_users
            .entry(&event.weapon)
            .or_default()
            .insert(&event.player_name);
        if let Some(corporation) = &event.corporation {
            corp_pilots
                .entry(corporation)
                .or_default()
                .insert(&event.player_name);
        }
    }

    let owners: Vec<Option<String>> = events
        .iter()
        .map(|event| {
            let is_drone_target = event.kind == EventType::Damage
                && event.ship_name.as_deref() == Some(event.player_name.as_str());
            if !is_drone_target {
                return None;
            }
            let mut candidates: Vec<&String> = weapon_users
                .get(event.player_name.as_str())
                .map(|users| users.order.iter().collect())
                .unwrap_or_default();
            if let Some(pilots) = event
                .corporation
                .as_deref()
                .and_then(|corporation| corp_pilots.get(corporation))
            {
                let narrowed: Vec<&String> = candidates
                    .iter()
                    .copied()
                    .filter(|name| pilots.seen.contains(*name))
                    .collect();
                if !narrowed.is_empty() {
                    candidates = narrowed;
                } else if candidates.is_empty() && pilots.order.len() == 1 {
                    candidates = pilots.order.iter().collect();
                }
            }
            (candidates.len() == 1).then(|| candidates[0].clone())
        })
        .collect();

    events
        .into_iter()
        .zip(owners)
        .map(|(event, owner)| match owner {
            Some(player_name) => Event {
                player_name,
                ..event
            },
            None => event,
        })
        .collect()
}
