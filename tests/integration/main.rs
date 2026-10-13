//! The single integration-test binary: every suite lives here as a module
//! so a full `cargo test` links one executable.
//!
//! The suites assume they never run concurrently with each other: they
//! truncate the shared tables. `.cargo/config.toml` pins
//! `RUST_TEST_THREADS=1` to keep that assumption true.
//!
//! The ported PHP tests keep their file names (`combat_log_parser` is
//! `parser`, `combat_log_upload`, `example`); the `golden_*` suites,
//! `parser`'s fixture comparisons and `php_functions` compare with what
//! the Laravel reference answered.

mod common;

mod behavior;
mod combat_log_upload;
mod esi_client;
mod example;
mod filters;
mod golden_http;
mod golden_mutations;
mod golden_pages;
mod harness;
mod parser;
mod php_functions;
mod routes;
