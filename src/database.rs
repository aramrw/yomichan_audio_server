use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Error, Row};

#[derive(Default, Deserialize, Serialize, Debug)]
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

    let result = sqlx::query("SELECT * FROM entries WHERE expression = ? AND reading = ?")
        .bind(term)
        .bind(reading)
        .fetch_all(&sqlite_pool)
        .await?;

    let forvo_result =
        sqlx::query("SELECT * FROM entries WHERE expression = ? AND source = 'forvo' ORDER BY speaker DESC")
            .bind(term)
            .fetch_all(&sqlite_pool)
            .await?;

    let mut query_entries: Vec<Entry> = Vec::new();

    result.iter().for_each(|row| {
        let reading: Option<String> = row.try_get("reading").unwrap_or_default();
        let speaker: Option<String> = row.try_get("speaker").unwrap_or_default();

        // file starts with the folder name so cut it out
        let mut file: String = row.get("file");
        file = file.rsplit_once('\\').unwrap().1.to_string();

        query_entries.push(Entry {
            expression: row.try_get("expression").unwrap_or_default(),
            reading,
            source: row.get("source"),
            speaker,
            display: row.get("display"),
            file,
        });
    });

    forvo_result.iter().for_each(|row| {
        let reading: Option<String> = row.try_get("reading").unwrap_or_default();
        let speaker: Option<String> = row.try_get("speaker").unwrap_or_default();

        // file starts with the folder name so cut it out
        let mut file: String = row.get("file");
        file = file.rsplit_once('\\').unwrap().1.to_string();

        query_entries.push(Entry {
            expression: row.try_get("expression").unwrap_or_default(),
            reading,
            source: row.get("source"),
            speaker,
            display: row.get("display"),
            file,
        });
    });

    query_entries.sort_by(|a, b| {
        let order = ["nhk16", "shinmeikai8", "forvo", "jpod"]; 
        let a_index = order.iter().position(|&x| x == a.source).unwrap();
        let b_index = order.iter().position(|&x| x == b.source).unwrap();
        a_index.cmp(&b_index)
    });
     

    Ok(query_entries)
}
