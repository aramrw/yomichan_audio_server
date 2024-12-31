mod test;

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::collections::hash_map::HashMap;
use std::fs::{read_dir, File};
use std::io::{BufReader, Write};
use std::path::Path;

#[derive(Default, Deserialize, Serialize, Debug)]
pub struct Entry {
    pub expression: String,
    pub reading: Option<String>,
    pub source: String,
    pub speaker: Option<String>,
    pub display: Option<String>,
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    name: String,
    year: u16,
    version: u8,
    media_dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JapaneseEntryFile {
    kana_reading: String,
    pitch_pattern: String,
    pitch_number: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenericEntryFile {
    pub speaker: Option<String>,
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexJson<T> {
    meta: Meta,
    headwords: Option<HashMap<String, Vec<String>>>,
    files: HashMap<String, Vec<T>>,
}

#[tokio::main]
pub async fn update_entries() {
    const INDEX_PATH: &str = "C:\\Users\\arami\\Desktop\\index.json";
    let file = File::open(INDEX_PATH).unwrap();
    let reader = BufReader::new(file);
    let stream = Deserializer::from_reader(reader).into_iter::<IndexJson<GenericEntryFile>>();

    let pool =
        sqlx::SqlitePool::connect("F:\\Programming\\Rust\\yomichan_http_server\\audio\\entries.db")
            .await
            .unwrap();
    create_test_table(&pool).await;

    let mut transaction = pool.begin().await.unwrap();
    for json in stream {
        let json = json.unwrap();
        for entry in &json.files {
            let expression = entry.0.clone();
            for item in entry.1 {
                let file = item.file.clone();
                let speaker = item.speaker.clone();
                let entry = Entry {
                    expression: expression.clone(),
                    reading: None,
                    source: json.meta.name.clone(),
                    speaker: speaker.clone(),
                    display: speaker,
                    file,
                };

                insert_entry(&mut transaction, entry).await;
            }
        }
    }

    transaction.commit().await.unwrap();
}

pub fn create_index_json(
    audio_path: &Path,
    meta_name: &str,
    year: Option<u16>,
    version: u8,
) -> Result<(), std::io::Error> {
    if !audio_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{:?} is not a valid directory.", audio_path),
        ));
    }

    let mut files: HashMap<String, Vec<GenericEntryFile>> = HashMap::new();

    for folder in read_dir(audio_path)? {
        let folder = folder?;
        let folder_name = folder.file_name();
        let speaker = folder_name.to_str().unwrap().to_string();

        for entry in read_dir(folder.path())? {
            let entry = entry?;
            let full_file_name = entry.file_name().to_string_lossy().to_string();
            if let Some((file_name, _)) = full_file_name.rsplit_once('.') {
                files
                    .entry(file_name.to_string())
                    .or_default()
                    .push(GenericEntryFile {
                        speaker: Some(speaker.clone()),
                        file: full_file_name,
                    });
            }
        }
    }

    let index = IndexJson {
        meta: Meta {
            name: String::from(meta_name),
            year: year.unwrap_or(2024),
            version,
            media_dir: String::from("Media"),
        },
        headwords: None,
        files,
    };

    let index_json = serde_json::to_string_pretty(&index)?;
    let mut file = File::create("index.json")?;
    file.write_all(index_json.as_bytes())?;

    Ok(())
}

// fn parse_file_name(file_name: &str) -> Option<String> {
//     if file_name.chars().all(|c| c.is_numeric()) {
//         Some(file_name)
//     } else {
//         Some(file_name.to_string())
//     }
// }

async fn create_test_table(pool: &sqlx::SqlitePool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS entries
    (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        expression TEXT NOT NULL,
        reading TEXT,
        source TEXT NOT NULL,
        speaker TEXT,
        display TEXT,
        file TEXT NOT NULL
    )",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn entry_exists(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    entry: &Entry,
) -> bool {
    let rows = sqlx::query(
        "
        SELECT *
        FROM entries
        WHERE
        expression = ? AND
        reading = ? AND
        source = ? AND
        file = ?",
    )
    .bind(&entry.expression)
    .bind(&entry.reading)
    .bind(&entry.source)
    .bind(&entry.file)
    .fetch_all(&mut **transaction)
    .await
    .unwrap();

    !rows.is_empty()
}

async fn insert_entry(transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entry: Entry) {
    if !entry_exists(transaction, &entry).await {
        sqlx::query(
            "
        INSERT into entries
        (expression, reading, source, speaker, display, file)
        VALUES
        (?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.expression)
        .bind(entry.reading)
        .bind(entry.source)
        .bind(entry.speaker)
        .bind(entry.display)
        .bind(entry.file)
        .execute(&mut **transaction)
        .await
        .unwrap();
    }
}

// fn format_pitch_display(pattern: &str, number: &str) -> String {
//     let mut pattern = pattern.replace('↓', "＼");
//     pattern = pattern.replace('○', "");
//     format!("{} [{}]", pattern.trim(), number)
// }
