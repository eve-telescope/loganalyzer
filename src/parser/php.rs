//! The PHP string functions the parser's output depends on, ported
//! byte for byte: `strip_tags` (PHP's state machine, not a regex),
//! `mb_trim` (PHP 8.4's default character list) and the `(int)` cast of a
//! digit string. tests/integration/php_functions.rs checks them against
//! what PHP answered for several hundred inputs.

/// PHP's `strip_tags($value)` without allowed tags: removes HTML and PHP
/// tags and comments, and NUL bytes. A `<` followed by whitespace is
/// text, quotes inside a tag hide `>`, nested `<` inside a tag count
/// depth. Works on bytes, so UTF-8 passes through unchanged.
pub fn strip_tags(input: &[u8]) -> Vec<u8> {
    let len = input.len();
    // PHP reads one byte past the end (the string's NUL terminator).
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

/// The characters PHP 8.4's `mb_trim` removes by default.
const MB_TRIM: &[char] = &[
    ' ', '\x0c', '\n', '\r', '\t', '\x0b', '\0', '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}',
    '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200A}', '\u{2028}', '\u{2029}', '\u{202F}', '\u{205F}', '\u{3000}', '\u{0085}', '\u{180E}',
];

/// PHP 8.4's `mb_trim($value)`.
pub fn mb_trim(value: &str) -> &str {
    value.trim_matches(MB_TRIM)
}

/// The parser's `clean`: `mb_trim(strip_tags($value))`.
pub fn clean(value: &[u8]) -> String {
    let stripped = String::from_utf8_lossy(&strip_tags(value)).into_owned();
    mb_trim(&stripped).to_owned()
}

/// PHP's `(int) $value` for a string: leading whitespace, an optional
/// sign, the ASCII digits that follow (anything else ends the number;
/// none at all is 0), saturating at `PHP_INT_MAX` and `PHP_INT_MIN`.
pub fn int(value: &str) -> i64 {
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
    fn strips_tags_like_php() {
        assert_eq!(strip_tags(b"<b>bold</b>"), b"bold");
        assert_eq!(strip_tags(b"a < b"), b"a < b");
        assert_eq!(strip_tags(b"a\0b"), b"ab");
        assert_eq!(strip_tags(b"<!-- c > still -->after"), b"after");
        assert_eq!(strip_tags(b"<a <b> c> d"), b" d");
    }

    #[test]
    fn trims_like_mb_trim() {
        assert_eq!(mb_trim("\u{180E}\0 x \u{3000}"), "x");
        assert_eq!(mb_trim("\u{FEFF}x"), "\u{FEFF}x");
    }

    #[test]
    fn casts_like_php() {
        assert_eq!(int("000123"), 123);
        assert_eq!(int("99999999999999999999999"), i64::MAX);
        assert_eq!(int("-99999999999999999999999"), i64::MIN);
        assert_eq!(int("+12"), 12);
        assert_eq!(int("\u{0663}"), 0);
    }
}
