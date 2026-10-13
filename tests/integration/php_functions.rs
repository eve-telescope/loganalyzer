//! The ports of PHP's string functions against what PHP answered
//! (tests/fixtures/php-functions/answers.json, recorded with
//! scripts/laravel-reference/php-functions.php over hand-picked and
//! random inputs).

use loganalyzer::parser::php::{int, mb_trim, strip_tags};
use serde_json::Value;

use crate::common::repo_path;
use crate::parser::base64_decode;

fn answers() -> Vec<Value> {
    let path = repo_path("tests/fixtures/php-functions/answers.json");
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
fn strip_tags_answers_like_php() {
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
fn mb_trim_answers_like_php() {
    for answer in answers() {
        let (Some(input), Some(expected)) = (utf8(&answer["input"]), utf8(&answer["mb_trim"]))
        else {
            continue;
        };
        assert_eq!(mb_trim(&input), expected, "mb_trim({input:?})");
    }
}

#[test]
fn the_int_cast_answers_like_php() {
    let mut checked = 0;
    for answer in answers() {
        let Some(expected) = answer["int"].as_i64() else {
            continue;
        };
        let input = utf8(&answer["input"]).expect("input");
        assert_eq!(int(&input), expected, "(int) {input:?}");
        checked += 1;
    }
    assert!(checked >= 9);
}

#[test]
fn the_client_name_and_extension_answer_like_symfony() {
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

/// FIXES.md, "Non-ASCII file names got a different ASCII fallback": the
/// inputs whose fallback name differs from Laravel's transliteration.
const FALLBACK_DIFFERENCES: &[&str] = &["ǅ.txt"];

#[test]
fn the_download_disposition_answers_like_symfony() {
    use loganalyzer::disposition::attachment;
    let mut checked = 0;
    for answer in answers() {
        let Some(expected) = utf8(&answer["disposition"]) else {
            continue;
        };
        let input = utf8(&answer["input"]).expect("input");
        // Symfony refused names with path separators; stored names never
        // have one (the upload keeps the part after the last).
        if expected.starts_with("error:") || FALLBACK_DIFFERENCES.contains(&input.as_str()) {
            continue;
        }
        assert_eq!(attachment(&input), expected, "{input:?}");
        checked += 1;
    }
    assert!(checked > 400, "{checked}");
    for input in FALLBACK_DIFFERENCES {
        let answer = answers()
            .into_iter()
            .find(|answer| utf8(&answer["input"]).as_deref() == Some(*input))
            .expect("an answer");
        assert_ne!(
            attachment(input),
            utf8(&answer["disposition"]).expect("a disposition")
        );
    }
}
