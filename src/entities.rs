//! EVE names to ids, cached in `eve_entities`. Every name of an upload
//! is looked up once: hits and misses are both stored, so a name only
//! ever goes to ESI until ESI has answered for it. When ESI fails, the
//! names of that chunk stay uncached and the upload goes on without
//! them.

use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use crate::esi::{Esi, MAX_NAMES_PER_REQUEST};

pub const CHARACTER: &str = "character";
pub const INVENTORY_TYPE: &str = "inventory_type";

/// Unique names in first-seen order, without empty ones (and without
/// `"0"`, which no EVE name is).
fn distinct(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    names
        .into_iter()
        .filter(|name| !name.is_empty() && name != "0")
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

/// One row to upsert, keyed like `{name}|character` / `{name}|type`.
struct Row {
    kind: &'static str,
    name: String,
    eve_id: Option<i64>,
}

/// An insertion-ordered map of rows where a later write to a key keeps
/// the key's place.
#[derive(Default)]
struct Rows {
    order: Vec<Row>,
    index: HashMap<(String, &'static str), usize>,
}

impl Rows {
    fn set(&mut self, kind: &'static str, name: &str, eve_id: Option<i64>) {
        let key = (name.to_owned(), kind);
        match self.index.get(&key) {
            Some(&position) => self.order[position].eve_id = eve_id,
            None => {
                self.index.insert(key, self.order.len());
                self.order.push(Row {
                    kind,
                    name: name.to_owned(),
                    eve_id,
                });
            }
        }
    }
}

/// Resolves the pilot slot names (`character_names`) and ship names
/// (`type_names`) of a log. A name in the pilot slot that ESI also knows
/// as an item type (a drone, a structure) is stored as that type and
/// never as a character, so a player who shares a drone's name never
/// lends it a portrait.
pub async fn resolve(
    pool: &PgPool,
    esi: &Esi,
    character_names: impl IntoIterator<Item = String>,
    type_names: impl IntoIterator<Item = String>,
) -> sqlx::Result<()> {
    let character_names = distinct(character_names);
    let type_names = distinct(type_names);
    let all: Vec<String> = character_names.iter().chain(&type_names).cloned().collect();

    // The cache, compared case- and accent-insensitively; the stored
    // spelling is what counts as known.
    let known: Vec<(String, String)> =
        sqlx::query_as("select kind, name from eve_entities where name = any($1) order by id")
            .bind(&all)
            .fetch_all(pool)
            .await?;
    let known_of = |kind: &str| -> HashSet<String> {
        known
            .iter()
            .filter(|(row_kind, _)| row_kind == kind)
            .map(|(_, name)| name.clone())
            .collect()
    };
    let known_characters = known_of(CHARACTER);
    let known_types = known_of(INVENTORY_TYPE);

    let missing = distinct(
        character_names
            .iter()
            .filter(|name| !known_characters.contains(*name))
            .chain(
                type_names
                    .iter()
                    .filter(|name| !known_types.contains(*name)),
            )
            .cloned(),
    );

    for chunk in missing.chunks(MAX_NAMES_PER_REQUEST) {
        let resolved = match esi.ids(chunk).await {
            Ok(resolved) => resolved,
            Err(error) => {
                tracing::warn!(%error, names = chunk.len(), "names left unresolved");
                continue;
            }
        };
        let in_chunk: HashSet<&String> = chunk.iter().collect();

        let mut rows = Rows::default();
        for name in character_names
            .iter()
            .filter(|name| in_chunk.contains(name))
        {
            match resolved.inventory_types.get(name) {
                Some(&type_id) => {
                    rows.set(CHARACTER, name, None);
                    rows.set(INVENTORY_TYPE, name, Some(type_id));
                }
                None => rows.set(CHARACTER, name, resolved.characters.get(name).copied()),
            }
        }
        for name in type_names.iter().filter(|name| in_chunk.contains(name)) {
            rows.set(
                INVENTORY_TYPE,
                name,
                resolved.inventory_types.get(name).copied(),
            );
        }

        // One statement per row: two spellings of one name in a chunk
        // land on the same row, the later one winning.
        let mut transaction = pool.begin().await?;
        for row in &rows.order {
            sqlx::query(
                "insert into eve_entities (kind, name, eve_id, created_at, updated_at)
                 values ($1, $2, $3, now(), now())
                 on conflict (kind, name) do update
                 set eve_id = excluded.eve_id, updated_at = excluded.updated_at",
            )
            .bind(row.kind)
            .bind(&row.name)
            .bind(row.eve_id)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
    }
    Ok(())
}

/// The resolved ids of a log's names for the analysis page: characters
/// and item types, keyed by the stored spelling, misses left out.
pub async fn ids_for(
    pool: &PgPool,
    names: &[String],
) -> sqlx::Result<(Vec<(String, i64)>, Vec<(String, i64)>)> {
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "select kind, name, eve_id from eve_entities
         where name = any($1) and eve_id is not null order by id",
    )
    .bind(names)
    .fetch_all(pool)
    .await?;
    let mut characters = Vec::new();
    let mut types = Vec::new();
    for (kind, name, eve_id) in rows {
        if kind == CHARACTER {
            characters.push((name, eve_id));
        } else {
            types.push((name, eve_id));
        }
    }
    Ok((characters, types))
}
