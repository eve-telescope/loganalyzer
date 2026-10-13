//! The text helpers the parser's output depends on: stripping markup (a
//! state machine, not a regex), trimming Unicode whitespace and reading a
//! number out of a digit string. tests/integration/string_functions.rs
//! pins them over several hundred inputs.

/// Removes HTML tags, `<?...?>` blocks and comments, and NUL bytes. A `<` followed by whitespace is
/// text, quotes inside a tag hide `>`, nested `<` inside a tag count
/// depth. Works on bytes, so UTF-8 passes through unchanged.
pub fn strip_tags(input: &[u8]) -> Vec<u8> {
    let len = input.len();
    // One byte past the end reads as NUL.
    let at = |index: usize| -> u8 { input.get(index).copied().unwrap_or(0) };
    let is_space = |byte: u8| matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);

    let mut out = Vec::with_capacity(len);
    let mut state = 0u8;
    let mut depth = 0usize;
    let mut in_quote = 0u8;
    let mut last = 0u8;
    let mut brackets = 0i64;
    let mut is_xml = false;
    let mut p = 0usize;

    while p < len {
        let c = input[p];
        match state {
            0 => match c {
                0 => {}
                b'<' => {
                    if in_quote != 0 {
                    } else if is_space(at(p + 1)) {
                        out.push(c);
                    } else {
                        last = b'<';
                        state = 1;
                    }
                }
                b'>' => {
                    if depth > 0 {
                        depth -= 1;
                    } else if in_quote == 0 {
                        out.push(c);
                    }
                }
                _ => out.push(c),
            },
            1 => match c {
                0 => {}
                b'<' => {
                    if in_quote == 0 && !is_space(at(p + 1)) {
                        depth += 1;
                    }
                }
                b'>' => {
                    if depth > 0 {
                        depth -= 1;
                    } else if in_quote == 0 {
                        last = b'>';
                        if !(is_xml && p >= 1 && input[p - 1] == b'-') {
                            in_quote = 0;
                            state = 0;
                            is_xml = false;
                        }
                    }
                }
                b'"' | b'\'' => {
                    if p != 0 && (in_quote == 0 || c == in_quote) {
                        in_quote = if in_quote != 0 { 0 } else { c };
                    }
                }
                b'!' => {
                    if p >= 1 && input[p - 1] == b'<' {
                        state = 3;
                        last = c;
                    }
                }
                b'?' if p >= 1 && input[p - 1] == b'<' => {
                    brackets = 0;
                    state = 2;
                }
                _ => {}
            },
            2 => match c {
                b'(' => {
                    if last != b'"' && last != b'\'' {
                        last = b'(';
                        brackets += 1;
                    }
                }
                b')' => {
                    if last != b'"' && last != b'\'' {
                        last = b')';
                        brackets -= 1;
                    }
                }
                b'>' => {
                    if depth > 0 {
                        depth -= 1;
                    } else if in_quote == 0
                        && brackets == 0
                        && p >= 1
                        && last != b'"'
                        && input[p - 1] == b'?'
                    {
                        in_quote = 0;
                        state = 0;
                    }
                }
                b'"' | b'\'' => {
                    if p >= 1 && input[p - 1] != b'\\' {
                        if last == c {
                            last = 0;
                        } else if last != b'\\' {
                            last = c;
                        }
                        if p != 0 && (in_quote == 0 || c == in_quote) {
                            in_quote = if in_quote != 0 { 0 } else { c };
                        }
                    }
                }
                b'l' | b'L'
                    if p > 4
                        && matches!(input[p - 1], b'm' | b'M')
                        && matches!(input[p - 2], b'x' | b'X')
                        && input[p - 3] == b'?'
                        && input[p - 4] == b'<' =>
                {
                    state = 1;
                    is_xml = true;
                }
                _ => {}
            },
            3 => match c {
                b'>' => {
                    if depth > 0 {
                        depth -= 1;
                    } else if in_quote == 0 {
                        in_quote = 0;
                        state = 0;
                    }
                }
                b'"' | b'\'' => {
                    if p != 0 && input[p - 1] != b'\\' && (in_quote == 0 || c == in_quote) {
                        in_quote = if in_quote != 0 { 0 } else { c };
                    }
                }
                b'-' => {
                    if p >= 2 && input[p - 1] == b'-' && input[p - 2] == b'!' {
                        state = 4;
                    }
                }
                b'E' | b'e'
                    // `<!DOCTYPE` is a tag, not a comment.
                    if p > 6 && input[p - 6..p].eq_ignore_ascii_case(b"doctyp") => {
                        state = 1;
                    }
                _ => {}
            },
            _ => {
                if c == b'>'
                    && in_quote == 0
                    && p >= 2
                    && input[p - 1] == b'-'
                    && input[p - 2] == b'-'
                {
                    in_quote = 0;
                    state = 0;
                }
            }
        }
        p += 1;
    }
    out
}

/// The whitespace `trim_whitespace` removes.
const WHITESPACE: &[char] = &[
    ' ', '\x0c', '\n', '\r', '\t', '\x0b', '\0', '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}',
    '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200A}', '\u{2028}', '\u{2029}', '\u{202F}', '\u{205F}', '\u{3000}', '\u{0085}', '\u{180E}',
];

/// Trims Unicode whitespace (the set above, not `char::is_whitespace`:
/// no BOM, but U+180E) from both ends.
pub fn trim_whitespace(value: &str) -> &str {
    value.trim_matches(WHITESPACE)
}

/// A field as the parser stores it: markup stripped, then trimmed.
pub fn clean(value: &[u8]) -> String {
    let stripped = String::from_utf8_lossy(&strip_tags(value)).into_owned();
    trim_whitespace(&stripped).to_owned()
}

/// The number at the start of a string: leading whitespace, an optional
/// sign, the ASCII digits that follow (anything else ends the number;
/// none at all is 0), saturating at `i64::MAX` and `i64::MIN`.
pub fn leading_int(value: &str) -> i64 {
    let rest = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, digits) = match rest.as_bytes().first() {
        Some(b'-') => (true, &rest[1..]),
        Some(b'+') => (false, &rest[1..]),
        _ => (false, rest),
    };
    let mut number: i64 = 0;
    for byte in digits.bytes().take_while(u8::is_ascii_digit) {
        let digit = i64::from(byte - b'0');
        number = if negative {
            match number.checked_mul(10).and_then(|n| n.checked_sub(digit)) {
                Some(n) => n,
                None => return i64::MIN,
            }
        } else {
            match number.checked_mul(10).and_then(|n| n.checked_add(digit)) {
                Some(n) => n,
                None => return i64::MAX,
            }
        };
    }
    number
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tags_and_keeps_a_lone_less_than() {
        assert_eq!(strip_tags(b"<b>bold</b>"), b"bold");
        assert_eq!(strip_tags(b"a < b"), b"a < b");
        assert_eq!(strip_tags(b"a\0b"), b"ab");
        assert_eq!(strip_tags(b"<!-- c > still -->after"), b"after");
        assert_eq!(strip_tags(b"<a <b> c> d"), b" d");
    }

    #[test]
    fn trims_unicode_whitespace() {
        assert_eq!(trim_whitespace("\u{180E}\0 x \u{3000}"), "x");
        assert_eq!(trim_whitespace("\u{FEFF}x"), "\u{FEFF}x");
    }

    #[test]
    fn reads_the_leading_number() {
        assert_eq!(leading_int("000123"), 123);
        assert_eq!(leading_int("99999999999999999999999"), i64::MAX);
        assert_eq!(leading_int("-99999999999999999999999"), i64::MIN);
        assert_eq!(leading_int("+12"), 12);
        assert_eq!(leading_int("\u{0663}"), 0);
    }
}
