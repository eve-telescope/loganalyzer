//! The text helpers over several hundred hand-picked and random inputs
//! (tests/fixtures/string-functions/answers.json, the strings base64
//! encoded): stripping markup, trimming, reading a leading number, the
//! uploaded file's name and extension, and the download's
//! `Content-Disposition`.

use loganalyzer::parser::text::{leading_int, strip_tags, trim_whitespace};
use serde_json::Value;

use crate::common::repo_path;
use crate::parser::base64_decode;

fn answers() -> Vec<Value> {
    let path = repo_path("tests/fixtures/string-functions/answers.json");
    let answers: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("answers")).expect("JSON");
    answers.as_array().expect("a list").clone()
}

fn bytes(value: &Value) -> Option<Vec<u8>> {
    value.as_str().map(base64_decode)
}

fn utf8(value: &Value) -> Option<String> {
    bytes(value).map(|bytes| String::from_utf8(bytes).expect("UTF-8"))
}

#[test]
fn strip_tags_removes_markup() {
    let answers = answers();
    assert!(answers.len() > 700);
    for answer in &answers {
        let input = bytes(&answer["input"]).expect("input");
        assert_eq!(
            String::from_utf8_lossy(&strip_tags(&input)),
            String::from_utf8_lossy(&bytes(&answer["strip_tags"]).expect("strip_tags")),
            "strip_tags({:?})",
            String::from_utf8_lossy(&input),
        );
    }
}

#[test]
fn trim_whitespace_trims_unicode_whitespace() {
    for answer in answers() {
        let (Some(input), Some(expected)) =
            (utf8(&answer["input"]), utf8(&answer["trim_whitespace"]))
        else {
            continue;
        };
        assert_eq!(trim_whitespace(&input), expected, "{input:?}");
    }
}

#[test]
fn leading_int_reads_the_number_at_the_start() {
    let mut checked = 0;
    for answer in answers() {
        let Some(expected) = answer["leading_int"].as_i64() else {
            continue;
        };
        let input = utf8(&answer["input"]).expect("input");
        assert_eq!(leading_int(&input), expected, "{input:?}");
        checked += 1;
    }
    assert!(checked >= 9);
}

#[test]
fn the_client_name_and_extension_drop_the_path() {
    use loganalyzer::upload::{client_extension, client_name};
    let mut checked = 0;
    for answer in answers() {
        let Some(expected) = utf8(&answer["client_name"]) else {
            continue;
        };
        let input = utf8(&answer["input"]).expect("input");
        assert_eq!(client_name(&input), expected, "name of {input:?}");
        assert_eq!(
            client_extension(&client_name(&input)),
            utf8(&answer["client_extension"]).expect("an extension"),
            "extension of {input:?}"
        );
        checked += 1;
    }
    assert!(checked > 700);
}

/// Names with a path separator have no disposition: a stored name never
/// has one (the upload keeps the part after the last).
#[test]
fn the_download_disposition_has_an_ascii_fallback() {
    use loganalyzer::disposition::attachment;
    let mut checked = 0;
    for answer in answers() {
        let Some(expected) = utf8(&answer["disposition"]) else {
            continue;
        };
        let input = utf8(&answer["input"]).expect("input");
        assert_eq!(attachment(&input), expected, "{input:?}");
        checked += 1;
    }
    assert!(checked > 400, "{checked}");
}
