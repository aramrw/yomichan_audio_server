use std::process::Output;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, sqlite::SqlitePool, Error};
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

pub async fn query_database(term: &str, reading: &str) -> Result<Vec<Entry>, Error> {
    let sqlite_pool = SqlitePool::connect("./audio/entries.db").await?;

    let fetch_result =
        sqlx::query_as::<_, Entry>("SELECT * FROM entries WHERE expression = ? AND reading = ?")
            .bind(term)
            .bind(reading)
            .fetch_all(&sqlite_pool);

    let fetch_forvo_result;

    if KANA_MAP
        .get_by_right(reading.chars().next().unwrap().to_string().as_str())
        .is_some()
    {
        fetch_forvo_result = sqlx::query_as::<_, Entry>(
            "SELECT * FROM entries WHERE expression = ? AND source = 'forvo' ORDER BY speaker DESC",
        )
        .bind(term)
        .fetch_all(&sqlite_pool);
    } else {
        fetch_forvo_result = sqlx::query_as::<_, Entry>(
            "SELECT * FROM entries WHERE expression = ? AND source = 'forvo_zh' ORDER BY speaker DESC",
        )
        .bind(term)
        .fetch_all(&sqlite_pool);
    }

    // Await them concurrently
    let (result, forvo_result) = join!(fetch_result, fetch_forvo_result);
    let result = result?;
    let forvo_result = forvo_result?;

    /* Handle Results */
    let mut query_entries: Vec<Entry> = Vec::new();
    let dict_entries: Vec<Entry> = result
        .into_iter()
        .map(|mut ent| {
            // file _might_ start with the folder name so cut it out
            let file: String = ent.file;
            ent.file = file.rsplit_once('\\').unwrap().1.to_string();
            ent
        })
        .collect();

    let forvo_entries: Vec<Entry> = forvo_result
        .into_iter()
        .map(|mut ent| {
            // file _might_ start with the folder name so cut it out
            let file: String = ent.file;
            // Use rsplit_once properly
            let file = file
                .rsplit_once('\\')
                .map(|(_, suffix)| suffix.to_string()) // Extract the suffix and convert to String
                .unwrap_or(file); // If rsplit_once returns None, use the original file

            ent.file = file;
            //println!("{:?}", ent);
            ent
        })
        .collect();

    query_entries.extend(dict_entries);
    query_entries.extend(forvo_entries);

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
