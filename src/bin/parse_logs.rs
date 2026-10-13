//! Parses logs with the app's parser and prints the results, for the
//! differential check against the PHP parser (scripts/parser-diff): a
//! JSON object `{name: base64 of the log}` on stdin, `{name: {listener,
//! sessionStarted, events}}` on stdout. Reads nothing else and writes
//! nothing, so it can run over stored raw logs without touching them.

use std::io::Read;

use serde_json::{Map, Value};

fn decode(text: &str) -> Vec<u8> {
    let value = |byte: u8| -> u32 {
        match byte {
            b'A'..=b'Z' => u32::from(byte - b'A'),
            b'a'..=b'z' => u32::from(byte - b'a') + 26,
            b'0'..=b'9' => u32::from(byte - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("not base64"),
        }
    };
    let bytes: Vec<u8> = text.bytes().filter(|byte| *byte != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut buffer = 0u32;
        for (index, byte) in chunk.iter().enumerate() {
            buffer |= value(*byte) << (18 - 6 * index);
        }
        for index in 0..chunk.len() * 6 / 8 {
            out.push((buffer >> (16 - 8 * index)) as u8);
        }
    }
    out
}

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("logs on stdin");
    let logs: Map<String, Value> = serde_json::from_str(&input).expect("a JSON object");
    let mut parsed = Map::new();
    for (name, encoded) in logs {
        let bytes = decode(encoded.as_str().expect("base64"));
        let log = loganalyzer::parser::parse(&bytes);
        parsed.insert(name, serde_json::to_value(log).expect("JSON"));
    }
    println!(
        "{}",
        serde_json::to_string(&Value::Object(parsed)).expect("JSON")
    );
}
