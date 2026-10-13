//! The analysis page's URL state (`from`, `to`, `pilot`, `hide`), read
//! from the query string the way the page always read it: each value
//! trimmed of Unicode whitespace, an empty value as null, `hide` split on
//! commas with empty parts and `0` dropped. A parameter sent as an array
//! (`pilot[]=x`) is ignored.

use std::sync::LazyLock;

use pcre2::bytes::{Regex, RegexBuilder};
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Filters {
    pub from: Option<String>,
    pub to: Option<String>,
    pub pilot: Option<String>,
    pub hide: Vec<String>,
}

/// One query parameter after PHP's parsing: the last plain value, or an
/// array when the last occurrence used `name[...]`.
#[derive(Debug, Clone)]
enum Param {
    Value(String),
    Array,
}

/// The parameter `name` of a raw query string; the last occurrence wins.
fn param(query: &str, name: &str) -> Option<Param> {
    let mut found = None;
    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
        match key.split_once('[') {
            Some((base, rest)) if base == name && rest.contains(']') => found = Some(Param::Array),
            _ if key == name => found = Some(Param::Value(value.into_owned())),
            _ => {}
        }
    }
    found
}

static STR_TRIM: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new()
        .utf(true)
        .ucp(true)
        .build("^[\\s\\x{FEFF}\\x{200B}\\x{200E} \\n\\r\\t\\v\\0]+|[\\s\\x{FEFF}\\x{200B}\\x{200E} \\n\\r\\t\\v\\0]+$")
        .expect("pattern")
});

/// Laravel's `Str::trim`, which its TrimStrings middleware applies to
/// every query value: Unicode whitespace, BOM, zero width space and the
/// left-to-right mark at both ends.
pub fn str_trim(value: &str) -> String {
    let mut out = String::new();
    let mut last = 0;
    for found in STR_TRIM.find_iter(value.as_bytes()).filter_map(Result::ok) {
        out.push_str(&value[last..found.start()]);
        last = found.end();
    }
    out.push_str(&value[last..]);
    out
}

/// PHP's `trim`: ASCII whitespace and NUL.
fn php_trim(value: &str) -> &str {
    value.trim_matches([' ', '\n', '\r', '\t', '\x0b', '\0'])
}

/// A plain value after TrimStrings and ConvertEmptyStringsToNull.
fn value(query: &str, name: &str) -> Option<String> {
    match param(query, name)? {
        Param::Value(value) => Some(str_trim(&value)).filter(|value| !value.is_empty()),
        Param::Array => None,
    }
}

impl Filters {
    pub fn from_query(query: &str) -> Self {
        let hide = value(query, "hide")
            .map(|hide| {
                hide.split(',')
                    .map(php_trim)
                    .filter(|part| !part.is_empty() && *part != "0")
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            from: value(query, "from"),
            to: value(query, "to"),
            pilot: value(query, "pilot"),
            hide,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_like_laravel() {
        assert_eq!(str_trim("\u{200B} a b \u{3000}"), "a b");
        assert_eq!(str_trim("\u{180E}x"), "x");
    }
}
