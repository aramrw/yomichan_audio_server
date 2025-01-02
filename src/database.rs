use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::Error as SqlxError;
use sqlx::{prelude::FromRow, sqlite::SqlitePool};
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use tokio::join;

use crate::helper::KANA_MAP;

#[derive(Default, Deserialize, Serialize, Debug, FromRow)]
pub struct Entry {
    pub expression: String,
    pub reading: Option<String>,
    pub source: String,
    pub speaker: Option<String>,
    pub display: String,
    pub file: String,
}

impl Entry {
    pub fn strip_folder_name_prefix(&mut self) {
        // file _might_ start with the folder name so cut it out
        let file_path = Path::new(&self.file);
        if let Some(file_name) = file_path.file_name() {
            self.file = file_name.to_string_lossy().to_string();
        }
    }
}

// Define your custom error type
#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLx Error Occurred: {source}")]
    SqlxError {
        #[from]
        source: SqlxError,
    },
    #[error("audio folder is missing from the current directory: {0}")]
    MissingAudioFolder(PathBuf),
    #[error("entries.db is missing from the audio folder")]
    MissingEntriesDB,
}

async fn query_forvo_base(
    source: &str,
    term: &str,
    pool: &SqlitePool,
) -> Result<Vec<Entry>, sqlx::Error> {
    sqlx::query_as::<_, Entry>(
        "SELECT * FROM entries 
            WHERE expression = ? AND source = ? 
            ORDER BY speaker DESC",
    )
    .bind(term)
    .bind(source)
    .fetch_all(pool)
    .await
}

pub async fn query_database(
    term: &str,
    reading: &str,
    pool: &SqlitePool,
) -> color_eyre::Result<Vec<Entry>> {
    let fetch_dict_result = sqlx::query_as::<_, Entry>(
        "SELECT * FROM entries 
        WHERE expression = ? AND reading = ?",
    )
    .bind(term)
    .bind(reading)
    .fetch_all(pool);

    // decides whether to serve chinese audio or japanese audio.
    let first_char = reading.chars().next().unwrap();
    let mut tmp = [0u8; 4];
    let first = first_char.encode_utf8(&mut tmp);
    let fetch_forvo_result =
        if KANA_MAP.get_by_right(first).is_some() || KANA_MAP.get_by_left(first).is_some() {
            query_forvo_base("forvo", term, pool)
        } else {
            query_forvo_base("forvo_zh", term, pool)
        };

    // await them concurrently
    let (result, forvo_result) = join!(fetch_dict_result, fetch_forvo_result);
    let mut dict_entries = result?;
    let mut forvo_entries = forvo_result?;

    let (de_len, fe_len) = (dict_entries.len(), forvo_entries.len());

    /* Handle Results */
    dict_entries
        .par_iter_mut()
        .chain(forvo_entries.par_iter_mut())
        .for_each(|e| e.strip_folder_name_prefix());

    let mut query_entries: Vec<Entry> = Vec::with_capacity(de_len + fe_len);
    query_entries.extend(dict_entries.into_iter().chain(forvo_entries.into_iter()));

    query_entries.par_sort_unstable_by(|a, b| {
        let order = ["daijisen", "nhk16", "shinmeikai8", "forvo", "jpod"];

        // Find the index or use a default value if not found
        let a_index = order
            .iter()
            .position(|&x| x == a.source)
            .unwrap_or(order.len());
        let b_index = order
            .iter()
            .position(|&x| x == b.source)
            .unwrap_or(order.len());

        // Compare the indices
        a_index.cmp(&b_index)
    });

    Ok(query_entries)
}

#[cfg(test)]
mod db_tests {
    use sqlx::SqlitePool;

    use super::query_database;
    use crate::helper::AudioSource;
    use std::time::Instant;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_query() {
        //tracing_subscriber::fmt::init();
        let instant = Instant::now();
        let term = "本";
        let reading = "ほん";
        let pool = SqlitePool::connect("audio/entries.db").await.unwrap();
        let entries = query_database(term, reading, &pool).await.unwrap();
        assert!(!entries.is_empty());

        let audio_source_list = AudioSource::create_list(&entries);
        AudioSource::print_list(&audio_source_list);

        tracing::info!("\nelapsed: {:.3}ms\n", instant.elapsed().as_millis());
    }
}
