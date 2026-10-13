//! Combat logs: storing an upload, regenerating a log from its raw file,
//! and reading one back for the analysis page.

use serde::Serialize;
use serde_json::{Map, Value, json};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::filters::Filters;
use crate::parser::{Event, ParsedLog, parse};
use crate::server::AppState;
use crate::storage::{RawLogs, Undecodable};

/// Rows per insert statement.
const INSERT_CHUNK: usize = 500;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CombatLog {
    pub id: i64,
    pub uuid: Uuid,
    pub listener: String,
    pub session_started: String,
    pub original_filename: Option<String>,
}

/// A log's uuid as it appears in a link: the canonical 8-4-4-4-12 form,
/// any case. Postgres would also take braces or no hyphens, which never
/// named a log.
pub fn parse_uuid(text: &str) -> Option<Uuid> {
    let bytes = text.as_bytes();
    let canonical = bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        });
    canonical.then(|| Uuid::parse_str(text).ok()).flatten()
}

pub async fn find(pool: &PgPool, uuid: &str) -> sqlx::Result<Option<CombatLog>> {
    let Some(uuid) = parse_uuid(uuid) else {
        return Ok(None);
    };
    sqlx::query_as(
        "select id, uuid, listener, session_started, original_filename
         from combat_logs where uuid = $1",
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await
}

/// Parses the log off the async runtime: a 25 MB log takes a moment.
pub async fn parse_blocking(contents: Vec<u8>) -> (ParsedLog, Vec<u8>) {
    tokio::task::spawn_blocking(move || (parse(&contents), contents))
        .await
        .expect("the parser does not panic")
}

/// Looks the names of the events up in ESI (cached).
async fn resolve_names(state: &AppState, events: &[Event]) -> sqlx::Result<()> {
    crate::entities::resolve(
        &state.pool,
        &state.esi,
        events.iter().map(|event| event.player_name.clone()),
        events.iter().filter_map(|event| event.ship_name.clone()),
    )
    .await
}

async fn insert_events(
    transaction: &mut Transaction<'_, Postgres>,
    log_id: i64,
    events: &[Event],
) -> sqlx::Result<()> {
    for chunk in events.chunks(INSERT_CHUNK) {
        let column = |pick: fn(&Event) -> Option<String>| -> Vec<Option<String>> {
            chunk.iter().map(pick).collect()
        };
        sqlx::query(
            "insert into combat_events
                (combat_log_id, timestamp, damage, direction, player_name, corporation,
                 ship_name, weapon, quality, type)
             select $1, r.timestamp, r.damage, r.direction, r.player_name,
                 r.corporation, r.ship_name, r.weapon, r.quality, r.type
             from unnest($2::text[], $3::bigint[], $4::text[], $5::text[], $6::text[],
                 $7::text[], $8::text[], $9::text[], $10::text[])
             with ordinality as r(timestamp, damage, direction, player_name, corporation,
                 ship_name, weapon, quality, type, position)
             order by r.position",
        )
        .bind(log_id)
        .bind(column(|event| Some(event.timestamp.clone())))
        .bind(chunk.iter().map(|event| event.damage).collect::<Vec<i64>>())
        .bind(column(|event| Some(event.direction.as_str().to_owned())))
        .bind(column(|event| Some(event.player_name.clone())))
        .bind(column(|event| event.corporation.clone()))
        .bind(column(|event| event.ship_name.clone()))
        .bind(column(|event| Some(event.weapon.clone())))
        .bind(column(|event| Some(event.quality.clone())))
        .bind(column(|event| Some(event.kind.as_str().to_owned())))
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum StoreError {
    Database(sqlx::Error),
    Storage(std::io::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Database(error) => write!(formatter, "database: {error}"),
            StoreError::Storage(error) => write!(formatter, "storage: {error}"),
        }
    }
}

impl From<sqlx::Error> for StoreError {
    fn from(error: sqlx::Error) -> Self {
        StoreError::Database(error)
    }
}

/// Stores an upload: parses it, resolves its names, keeps the raw bytes
/// gzipped and saves the log with its events. The log and its events are
/// saved together or not at all; the raw file is written first and
/// removed again when saving fails.
pub async fn store(
    state: &AppState,
    contents: Vec<u8>,
    original_filename: Option<String>,
    uuid: Uuid,
) -> Result<Uuid, StoreError> {
    let (parsed, contents) = parse_blocking(contents).await;
    resolve_names(state, &parsed.events).await?;
    state
        .raw_logs
        .put(uuid, &contents)
        .await
        .map_err(StoreError::Storage)?;
    let saved = async {
        let mut transaction = state.pool.begin().await?;
        let log_id: i64 = sqlx::query_scalar(
            "insert into combat_logs
                (uuid, listener, session_started, original_filename, created_at, updated_at)
             values ($1, $2, $3, $4, now(), now()) returning id",
        )
        .bind(uuid)
        .bind(&parsed.listener)
        .bind(&parsed.session_started)
        .bind(&original_filename)
        .fetch_one(&mut *transaction)
        .await?;
        insert_events(&mut transaction, log_id, &parsed.events).await?;
        transaction.commit().await
    }
    .await;
    if let Err(error) = saved {
        state.raw_logs.delete(uuid).await;
        return Err(error.into());
    }
    Ok(uuid)
}

#[derive(Debug)]
pub enum RegenerateError {
    /// No raw file for this log.
    Missing,
    Undecodable,
    Failed(String),
}

/// Parses a log again from its raw file: the listener, the session start
/// and every event are replaced (the events get new ids); the file and
/// the original file name stay.
pub async fn regenerate(state: &AppState, log: &CombatLog) -> Result<(), RegenerateError> {
    let contents = match state.raw_logs.get(log.uuid).await {
        Ok(Some(Ok(contents))) => contents,
        Ok(Some(Err(Undecodable))) => return Err(RegenerateError::Undecodable),
        Ok(None) => return Err(RegenerateError::Missing),
        Err(error) => return Err(RegenerateError::Failed(error.to_string())),
    };
    let (parsed, _) = parse_blocking(contents).await;
    let failed = |error: sqlx::Error| RegenerateError::Failed(error.to_string());
    resolve_names(state, &parsed.events).await.map_err(failed)?;
    let mut transaction = state.pool.begin().await.map_err(failed)?;
    sqlx::query(
        "update combat_logs set listener = $2, session_started = $3, updated_at = now()
         where id = $1",
    )
    .bind(log.id)
    .bind(&parsed.listener)
    .bind(&parsed.session_started)
    .execute(&mut *transaction)
    .await
    .map_err(failed)?;
    sqlx::query("delete from combat_events where combat_log_id = $1")
        .bind(log.id)
        .execute(&mut *transaction)
        .await
        .map_err(failed)?;
    insert_events(&mut transaction, log.id, &parsed.events)
        .await
        .map_err(failed)?;
    transaction.commit().await.map_err(failed)
}

/// An event as the analysis page reads it.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct EventRow {
    timestamp: String,
    damage: i64,
    direction: String,
    player_name: String,
    corporation: Option<String>,
    ship_name: Option<String>,
    weapon: String,
    quality: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    kind: String,
}

/// The props of the analysis page.
pub async fn page_props(
    pool: &PgPool,
    raw_logs: &RawLogs,
    log: &CombatLog,
    filters: Filters,
) -> sqlx::Result<Map<String, Value>> {
    let events: Vec<EventRow> = sqlx::query_as(
        "select timestamp, damage, direction, player_name, corporation, ship_name, weapon,
                quality, type
         from combat_events where combat_log_id = $1 order by id",
    )
    .bind(log.id)
    .fetch_all(pool)
    .await?;

    let mut names: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let ship_names = events
        .iter()
        .filter_map(|event| event.ship_name.as_ref())
        .filter(|name| !name.is_empty() && name.as_str() != "0");
    for name in events
        .iter()
        .map(|event| &event.player_name)
        .chain(ship_names)
    {
        if seen.insert(name.as_str()) {
            names.push(name.clone());
        }
    }
    let (characters, types) = crate::entities::ids_for(pool, &names).await?;
    let ids = |pairs: Vec<(String, i64)>| -> Value {
        Value::Object(
            pairs
                .into_iter()
                .map(|(name, id)| (name, json!(id)))
                .collect(),
        )
    };

    let mut props = Map::new();
    props.insert(
        "analysis".to_owned(),
        json!({
            "listener": log.listener,
            "sessionStarted": log.session_started,
            "events": events,
        }),
    );
    props.insert("uuid".to_owned(), json!(log.uuid.to_string()));
    props.insert(
        "rawLogAvailable".to_owned(),
        json!(raw_logs.exists(log.uuid).await),
    );
    props.insert("pilotIds".to_owned(), ids(characters));
    props.insert("shipTypeIds".to_owned(), ids(types));
    props.insert("filters".to_owned(), json!(filters));
    Ok(props)
}
