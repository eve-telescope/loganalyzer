//! The analysis page's URL state (`from`, `to`, `pilot`, `hide`).

use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Filters {
    pub from: Option<String>,
    pub to: Option<String>,
    pub pilot: Option<String>,
    pub hide: Vec<String>,
}

/// Laravel's `Str::trim`.
pub fn str_trim(_value: &str) -> String {
    unimplemented!("not ported yet")
}

impl Filters {
    pub fn from_query(_query: &str) -> Self {
        unimplemented!("not ported yet")
    }
}
