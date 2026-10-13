//! The golden fixtures recorded from the Laravel reference
//! (scripts/laravel-reference/record.py) and the normalizations every
//! comparison applies. The only accepted differences are listed here and
//! in FIXES.md.
//!
//! - N1: Inertia's shared `errors` bag is dropped from page props. The
//!   upload form reads its errors from the 422 answer instead.
//! - N2: uuids are random: a recorded log's uuid stands for the uuid the
//!   replay's upload got.
//! - N3: an empty `pilotIds` or `shipTypeIds` was PHP's empty array
//!   (`[]`); it is the empty object here. Both maps compare as maps.
//! - N4: Laravel answered a form with a redirect: to the log (302
//!   `Location: /logs/{uuid}`) is `200 {"redirect": "/logs/{uuid}"}`
//!   here, back to the form with the errors flashed is
//!   `422 {"errors": {...}}`.
//! - N5: the ESI client names itself with a User-Agent of its own and
//!   tries a failing chunk twice instead of five times (FIXES.md); the
//!   names asked for and the compatibility date compare exactly.

use serde_json::Value;

use super::repo_path;

pub fn read_json(relative: &str) -> Value {
    let path = repo_path(&format!("tests/fixtures/{relative}"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

pub fn read_text(relative: &str) -> String {
    let path = repo_path(&format!("tests/fixtures/{relative}"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The names of the recorded fixtures in a golden directory.
pub fn recorded(directory: &str) -> Vec<String> {
    let path = repo_path(&format!("tests/fixtures/golden/{directory}"));
    let mut names: Vec<String> = std::fs::read_dir(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".json").map(str::to_owned)
        })
        .collect();
    names.sort();
    names
}

pub fn page(name: &str) -> Value {
    read_json(&format!("golden/pages/{name}.json"))
}

pub fn http(name: &str) -> Value {
    read_json(&format!("golden/http/{name}.json"))
}

pub fn scenario(name: &str) -> Value {
    read_json("golden/mutations.json")
        .as_array()
        .expect("scenarios")
        .iter()
        .find(|scenario| scenario["name"] == name)
        .unwrap_or_else(|| panic!("no scenario {name}"))
        .clone()
}

pub fn scenario_names() -> Vec<String> {
    read_json("golden/mutations.json")
        .as_array()
        .expect("scenarios")
        .iter()
        .map(|scenario| scenario["name"].as_str().expect("a name").to_owned())
        .collect()
}

/// The uuids the recording gave its uploaded logs (`uuids.json`).
pub fn recorded_uuids() -> Value {
    read_json("golden/uuids.json")
}

/// The fixture log and file name each recorded log was uploaded from
/// (record.py, `pages()`).
pub const RECORDED_LOGS: [(&str, &str, &str); 8] = [
    ("testlog", "my-fight.txt", "testlog"),
    ("testlog2", "testlog2.txt", "testlog2"),
    ("testlog3", "testlog3.txt", "testlog3"),
    (
        "hammerhead",
        "combat.txt",
        "upload-names-matching-an-item-type-are-never-treated-as-characters",
    ),
    ("tricky", "Kämpfe.txt", "edge-tricky-names"),
    ("drones", "with space.txt", "edge-drone-attribution"),
    ("empty", "per%cent.txt", "edge-empty"),
    ("multiline", "日本語.txt", "edge-multiline-then-notify"),
];

/// N1 and N3 on a recorded page's props.
pub fn normalize_props(props: &Value) -> Value {
    let mut props = props.clone();
    let fields = props.as_object_mut().expect("props");
    fields.remove("errors");
    for key in ["pilotIds", "shipTypeIds"] {
        if fields
            .get(key)
            .is_some_and(|value| value == &Value::Array(Vec::new()))
        {
            fields.insert(key.to_owned(), Value::Object(Default::default()));
        }
    }
    props
}
