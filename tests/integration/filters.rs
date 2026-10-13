//! The analysis page's URL state read from the query string: trimmed,
//! empty as null, `hide` split and filtered, array parameters ignored
//! (FIXES.md, "Array query parameters broke the analysis page").

use loganalyzer::filters::{Filters, str_trim};

fn filters(query: &str) -> Filters {
    Filters::from_query(query)
}

fn some(value: &str) -> Option<String> {
    Some(value.to_owned())
}

#[test]
fn reads_the_four_parameters() {
    assert_eq!(
        filters(
            "from=2026-02-12T10:20:00&to=2026-02-12T10:25:00&hide=logiDealt,neutOut&pilot=Aria%20Vex"
        ),
        Filters {
            from: some("2026-02-12T10:20:00"),
            to: some("2026-02-12T10:25:00"),
            pilot: some("Aria Vex"),
            hide: vec!["logiDealt".to_owned(), "neutOut".to_owned()],
        }
    );
}

#[test]
fn an_empty_query_has_no_filters() {
    assert_eq!(filters(""), Filters::default());
}

#[test]
fn values_are_trimmed_and_empty_ones_are_null() {
    let read = filters("from=%20&to=&pilot=%20%20Aria%20Vex%20&hide=%20%20");
    assert_eq!(read.from, None);
    assert_eq!(read.to, None);
    assert_eq!(read.pilot, some("Aria Vex"));
    assert!(read.hide.is_empty());
}

#[test]
fn unicode_whitespace_is_trimmed_at_the_ends_only() {
    let read = filters("pilot=%E2%80%8BAria%E3%80%80Vex%C2%A0&from=%E2%80%8E2026");
    assert_eq!(read.pilot, some("Aria\u{3000}Vex"));
    assert_eq!(read.from, some("2026"));
}

#[test]
fn hide_drops_empty_parts_and_zero() {
    assert_eq!(
        filters("hide=%20dpsDealt%20,,0,%20,neutIn,%200%20").hide,
        ["dpsDealt", "neutIn"]
    );
    assert!(filters("hide=0").hide.is_empty());
    assert_eq!(filters("hide=00,0.0").hide, ["00", "0.0"]);
}

#[test]
fn the_last_occurrence_wins() {
    let read = filters("from=a&from=b&hide=dpsDealt&hide=neutOut");
    assert_eq!(read.from, some("b"));
    assert_eq!(read.hide, ["neutOut"]);
}

#[test]
fn a_plus_is_a_space() {
    assert_eq!(filters("pilot=Aria+Vex").pilot, some("Aria Vex"));
}

#[test]
fn array_parameters_are_ignored() {
    let read = filters("pilot%5B%5D=Aria%20Vex&from%5Bx%5D=1&hide%5B%5D=dpsDealt&to[=x");
    assert_eq!(read, Filters::default());
    // The last occurrence decides.
    assert_eq!(filters("pilot[]=a&pilot=b").pilot, some("b"));
    assert_eq!(filters("pilot=b&pilot[]=a").pilot, None);
}

#[test]
fn bytes_that_are_not_utf8_become_replacement_characters() {
    assert_eq!(filters("pilot=%FFx").pilot, some("\u{FFFD}x"));
}

#[test]
fn str_trim_matches_laravel() {
    assert_eq!(str_trim("\u{FEFF}\u{200B}\u{200E} x \t\0"), "x");
    assert_eq!(str_trim("\u{2003}x\u{2029}"), "x");
}
