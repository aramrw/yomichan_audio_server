use serde::{Deserialize, Serialize};
use sqlx::{ sqlite::SqlitePool, Error, Row};

#[derive(Deserialize, Serialize)]
pub struct Entry {
    expression: String,
    reading: Option<String>,
    source: String,
    speaker: Option<String>,
    display: String,
}

pub async fn query_database(
    term: &str,
    reading: &str,
) -> Result<Vec<Entry>, Error> {

    let sqlite_pool = SqlitePool::connect("./entries.db").await?;

    let result = sqlx::query("SELECT * FROM entries WHERE expression = ? AND reading = ?")
        .bind(term)
        .bind(reading)
        .fetch_all(&sqlite_pool)
        .await?;

    let mut query_entries: Vec<Entry> = Vec::new();

    result.iter().for_each(|row| {
        let reading: Option<String> = row.try_get("reading").unwrap_or_default();
        let speaker: Option<String> = row.try_get("speaker").unwrap_or_default();

        query_entries.push(Entry {
            expression: row.try_get("expression").unwrap_or_default(),
            reading: reading,
            source: row.get("source"),
            speaker: speaker,
            display: row.get("display"),
        });
    });

    Ok(query_entries)
}
