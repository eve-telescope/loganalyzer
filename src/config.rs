//! Runtime configuration from the environment, resolved once at startup.
//! Tests build a `Config` by hand instead of touching process env vars.

use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    /// `APP_NAME`: the shared `name` prop and the title suffix.
    pub app_name: String,
    /// `APP_URL` without a trailing slash.
    pub app_url: String,
    /// `LOG_STORAGE_DIR`: where the raw logs live, `{uuid}.txt.gz`.
    pub log_storage_dir: PathBuf,
    /// `UPLOAD_MAX_SIZE_MB`: the largest log accepted, in MiB.
    pub upload_max_size_mb: u64,
    pub esi: EsiConfig,
}

/// How names are resolved through ESI (`POST /universe/ids/`).
#[derive(Debug, Clone)]
pub struct EsiConfig {
    /// `ESI_BASE_URL`, `https://esi.evetech.net` by default.
    pub base_url: String,
    /// `ESI_USER_AGENT`: ESI asks every client to name itself and a way
    /// to reach its owner.
    pub user_agent: String,
    /// Attempts per chunk of names. A chunk that still fails is skipped
    /// and its names stay unresolved, so a slow ESI never holds an upload
    /// for long.
    pub tries: u32,
    pub retry_delay: Duration,
    /// The limit for one attempt, connecting included.
    pub timeout: Duration,
}

pub const DEFAULT_APP_NAME: &str = "EVE Log Analyzer";
pub const DEFAULT_APP_URL: &str = "https://loganalyzer.eve-telescope.com";
pub const DEFAULT_UPLOAD_MAX_SIZE_MB: u64 = 25;
pub const DEFAULT_ESI_BASE_URL: &str = "https://esi.evetech.net";
const DEFAULT_LOG_STORAGE_DIR: &str = "storage/combat-logs";
pub const DEFAULT_ESI_TRIES: u32 = 2;
pub const DEFAULT_ESI_RETRY_DELAY: Duration = Duration::from_millis(250);
pub const DEFAULT_ESI_TIMEOUT: Duration = Duration::from_secs(5);

/// The User-Agent ESI sees: the app, its site and its source.
pub fn default_user_agent(app_url: &str) -> String {
    format!("EVE Log Analyzer/1.0 (+{app_url}; +https://github.com/eve-telescope/loganalyzer)")
}

/// A variable that is set and not empty.
fn env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.is_empty() && value != "null")
}

fn env_or(key: &str, default: &str) -> String {
    env(key).unwrap_or_else(|| default.to_owned())
}

/// A number from the environment; one that does not parse stops the
/// startup instead of running with a surprise.
fn env_number<T: std::str::FromStr>(key: &str, default: T) -> T {
    env(key).map_or(default, |value| {
        value
            .parse()
            .unwrap_or_else(|_| panic!("{key} is {value:?}, expected a number"))
    })
}

impl Config {
    pub fn from_env() -> Self {
        let app_url = env_or("APP_URL", DEFAULT_APP_URL)
            .trim_end_matches('/')
            .to_owned();
        Self {
            app_name: env_or("APP_NAME", DEFAULT_APP_NAME),
            log_storage_dir: PathBuf::from(env_or("LOG_STORAGE_DIR", DEFAULT_LOG_STORAGE_DIR)),
            upload_max_size_mb: env_number("UPLOAD_MAX_SIZE_MB", DEFAULT_UPLOAD_MAX_SIZE_MB),
            esi: EsiConfig {
                base_url: env_or("ESI_BASE_URL", DEFAULT_ESI_BASE_URL)
                    .trim_end_matches('/')
                    .to_owned(),
                user_agent: env("ESI_USER_AGENT").unwrap_or_else(|| default_user_agent(&app_url)),
                tries: env_number("ESI_TRIES", DEFAULT_ESI_TRIES).max(1),
                retry_delay: Duration::from_millis(env_number(
                    "ESI_RETRY_DELAY_MS",
                    DEFAULT_ESI_RETRY_DELAY.as_millis() as u64,
                )),
                timeout: Duration::from_millis(env_number(
                    "ESI_TIMEOUT_MS",
                    DEFAULT_ESI_TIMEOUT.as_millis() as u64,
                )),
            },
            app_url,
        }
    }

    /// The largest accepted log in bytes.
    pub fn upload_max_bytes(&self) -> u64 {
        self.upload_max_size_mb * 1024 * 1024
    }
}
