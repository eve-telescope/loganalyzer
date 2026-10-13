//! The `Content-Disposition` of a download: `attachment;
//! filename=<ASCII>` plus `filename*=utf-8''<name>` (RFC 6266) when the
//! name is not plain ASCII.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

/// Percent-encodes everything but letters, digits and `-_.~`.
const RAWURLENCODE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// The ASCII stand-in for a name, for clients without `filename*`:
/// Unicode spaces become spaces, invisible and control characters go,
/// letters of the Latin, Greek and Cyrillic scripts are transliterated
/// (deunicode), anything else is dropped.
pub fn ascii(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        match c {
            ' '..='~' => out.push(c),
            '\t'
            | '\n'
            | '\r'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200B}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}' => out.push(' '),
            '\u{00C0}'..='\u{052F}' | '\u{1E00}'..='\u{1EFF}' => {
                if let Some(transliterated) = deunicode::deunicode_char(c)
                    && transliterated
                        .bytes()
                        .all(|byte| (b' '..=b'~').contains(&byte))
                {
                    out.push_str(transliterated);
                }
            }
            _ => {}
        }
    }
    out
}

/// A header parameter value: a token stays bare, anything else is
/// quoted with `"` and `\` escaped.
fn quote(value: &str) -> String {
    let token = !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*.^_`|~-".contains(&byte));
    if token {
        value.to_owned()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

pub fn attachment(name: &str) -> String {
    // Path separators were never part of a stored name (the upload kept
    // only the part after the last one); a name with one is cut there.
    let name = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let fallback = ascii(name).replace('%', "");
    let mut header = format!("attachment; filename={}", quote(&fallback));
    if fallback != name {
        header.push_str("; filename*=");
        header.push_str(&quote(&format!(
            "utf-8''{}",
            utf8_percent_encode(name, RAWURLENCODE)
        )));
    }
    header
}

#[cfg(test)]
mod tests {
    use super::attachment;

    #[test]
    fn builds_dispositions_with_an_ascii_fallback() {
        assert_eq!(
            attachment("my-fight.txt"),
            "attachment; filename=my-fight.txt"
        );
        assert_eq!(
            attachment("with space.txt"),
            "attachment; filename=\"with space.txt\""
        );
        assert_eq!(
            attachment("Kämpfe.txt"),
            "attachment; filename=Kampfe.txt; filename*=utf-8''K%C3%A4mpfe.txt"
        );
        assert_eq!(
            attachment("per%cent.txt"),
            "attachment; filename=percent.txt; filename*=utf-8''per%25cent.txt"
        );
    }
}
