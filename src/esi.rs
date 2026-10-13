//! ESI's `POST /universe/ids/`: names in, the characters and item types
//! they belong to out. Public, no login.

use std::collections::HashMap;
use std::sync::LazyLock;

use pcre2::bytes::{Regex, RegexBuilder};
use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

use crate::config::EsiConfig;

/// The API version this client is written against (ESI's
/// `X-Compatibility-Date`).
pub const COMPATIBILITY_DATE: &str = "2026-05-19";

/// ESI resolves at most this many names per request.
pub const MAX_NAMES_PER_REQUEST: usize = 500;

#[derive(Debug, Clone)]
pub struct Esi {
    client: reqwest::Client,
    config: EsiConfig,
}

/// The ids of the names ESI knew, by their spelling in ESI.
#[derive(Debug, Default, Clone)]
pub struct UniverseIds {
    pub characters: HashMap<String, i64>,
    pub inventory_types: HashMap<String, i64>,
}

#[derive(Debug)]
pub enum EsiError {
    /// No answer (connection refused, timeout).
    Unreachable(String),
    /// An answer that was not a success.
    Status(u16),
    /// A success whose body was not the documented shape.
    Malformed(String),
}

impl std::fmt::Display for EsiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EsiError::Unreachable(error) => write!(formatter, "ESI unreachable: {error}"),
            EsiError::Status(status) => write!(formatter, "ESI answered {status}"),
            EsiError::Malformed(error) => write!(formatter, "ESI answer malformed: {error}"),
        }
    }
}

#[derive(Deserialize)]
struct IdName {
    id: i64,
    name: String,
}

#[derive(Deserialize)]
struct IdsBody {
    #[serde(default)]
    characters: Vec<IdName>,
    #[serde(default)]
    inventory_types: Vec<IdName>,
}

impl Esi {
    pub fn new(config: EsiConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(config.user_agent.clone())
            .timeout(config.timeout)
            .build()
            .expect("an HTTP client");
        Self { client, config }
    }

    /// Resolves up to 500 names. Retries a server error or a failed
    /// connection until the configured tries are used up.
    pub async fn ids(&self, names: &[String]) -> Result<UniverseIds, EsiError> {
        let url = format!("{}/universe/ids/", self.config.base_url);
        let mut attempt = 1;
        loop {
            let outcome = self
                .client
                .post(&url)
                .header("X-Compatibility-Date", COMPATIBILITY_DATE)
                .json(names)
                .send()
                .await;
            let retryable = match &outcome {
                Ok(response) => response.status().is_server_error(),
                Err(_) => true,
            };
            if retryable && attempt < self.config.tries {
                attempt += 1;
                tokio::time::sleep(self.config.retry_delay).await;
                continue;
            }
            let response = outcome.map_err(|error| EsiError::Unreachable(error.to_string()))?;
            if !response.status().is_success() {
                return Err(EsiError::Status(response.status().as_u16()));
            }
            let body: IdsBody = response
                .json()
                .await
                .map_err(|error| EsiError::Malformed(error.to_string()))?;
            let map = |ids: Vec<IdName>| {
                ids.into_iter()
                    .map(|entry| (normalize(&entry.name), entry.id))
                    .collect()
            };
            return Ok(UniverseIds {
                characters: map(body.characters),
                inventory_types: map(body.inventory_types),
            });
        }
    }
}

static PYTHON_WRAPPER: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new()
        .build(r"u'((?:\\u[0-9a-fA-F]{4})+)'")
        .expect("pattern")
});
static ESCAPES: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new()
        .build(r"(?:\\u[0-9a-fA-F]{4})+")
        .expect("pattern")
});

/// A name as ESI meant it: literal `\uXXXX` escapes that ESI sometimes
/// leaves in strings decoded (also inside a Python-style `u'…'`), then
/// NFC.
pub fn normalize(name: &str) -> String {
    let decoded = if name.contains("\\u") {
        let unwrapped = replace_all(&PYTHON_WRAPPER, name, |captures| {
            String::from_utf8_lossy(&captures[1]).into_owned()
        });
        replace_all(&ESCAPES, &unwrapped, |captures| {
            let sequence = String::from_utf8_lossy(&captures[0]).into_owned();
            serde_json::from_str::<String>(&format!("\"{sequence}\"")).unwrap_or(sequence)
        })
    } else {
        name.to_owned()
    };
    decoded.nfc().collect()
}

fn replace_all(
    regex: &Regex,
    subject: &str,
    replacement: impl Fn(&pcre2::bytes::Captures<'_>) -> String,
) -> String {
    let mut out = String::new();
    let mut last = 0;
    for captures in regex
        .captures_iter(subject.as_bytes())
        .filter_map(Result::ok)
    {
        let whole = captures.get(0).expect("a match");
        out.push_str(&subject[last..whole.start()]);
        out.push_str(&replacement(&captures));
        last = whole.end();
    }
    out.push_str(&subject[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn decodes_literal_escapes_and_normalizes() {
        assert_eq!(normalize("u'\\u038f'"), "\u{038f}");
        assert_eq!(normalize("A \\u0410\\u043b B"), "A \u{410}\u{43b} B");
        assert_eq!(normalize("\\uD83D\\uDE80"), "\u{1F680}");
        assert_eq!(normalize("\\uD83D lone"), "\\uD83D lone");
        assert_eq!(normalize("e\u{0301}"), "\u{e9}");
        assert_eq!(normalize("plain"), "plain");
    }
}
