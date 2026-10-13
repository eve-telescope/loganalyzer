//! The PHP string functions the parser's output depends on, ported
//! byte for byte: `strip_tags`, `mb_trim` and the `(int)` cast of a
//! digit string.

/// PHP's `strip_tags($value)` without allowed tags.
pub fn strip_tags(_input: &[u8]) -> Vec<u8> {
    unimplemented!("not ported yet")
}

/// PHP 8.4's `mb_trim($value)`.
pub fn mb_trim(_value: &str) -> &str {
    unimplemented!("not ported yet")
}

/// The parser's `clean`: `mb_trim(strip_tags($value))`.
pub fn clean(_value: &[u8]) -> String {
    unimplemented!("not ported yet")
}

/// PHP's `(int) $value` for a string.
pub fn int(_value: &str) -> i64 {
    unimplemented!("not ported yet")
}
