use color_eyre::{eyre::Context, Result};
use color_print::ceprintln;
use rapidhash::{HashMapExt, RapidHashMap as HashMap};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// e.g., `{ "headwords": { "word1": ["file.mp3"], "word2": ["file2.mp3"] } }`
#[derive(Debug, Serialize, Deserialize)]
struct HeadwordsFormat {
    headwords: HashMap<String, Vec<String>>,
}

/// Represents a single item in the "entries" array format.
/// e.g., `{ "kanji": "word1", "kana": "...", "audio_file": "file.mp3" }`
#[derive(Debug, Serialize, Deserialize)]
struct EntryItem {
    kanji: String,
    // kana: String,
    #[serde(rename = "audio_file")]
    file: String,
}

/// **Format 2:** An array of entry objects.
/// e.g., `{ "entries": [ { "kanji": "...", "audio_file": "..." } ] }`
#[derive(Debug, Serialize, Deserialize)]
struct EntriesFormat {
    entries: Vec<EntryItem>,
}

/// An **untagged enum** that can deserialize into EITHER format.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum IndexData {
    Headwords(HeadwordsFormat),
    Entries(EntriesFormat),
}

impl IndexData {
    /// Normalizes either variant into the desired HashMap structure.
    fn into_headwords_map(self) -> HashMap<String, Vec<String>> {
        match self {
            // If it's already in the right format, just return the inner map.
            IndexData::Headwords(data) => data.headwords,

            // If it's the entries array, we need to convert it.
            IndexData::Entries(data) => {
                // 1. Explicitly define the full type here.
                let mut map: HashMap<String, Vec<String>> =
                    HashMap::with_capacity(data.entries.len());
                for item in data.entries {
                    // For each item, find or create a vector for its kanji,
                    // then push the audio file into that vector.

                    // 2. Use .push() to add the file to the Vec<String>.
                    map.entry(item.kanji).or_default().push(item.file);
                }
                map
            }
        }
    }
}

/// The main `index.json` structure.
/// **`#[serde(flatten)]`** tells Serde to look for the fields of `IndexData`
/// (`headwords` or `entries`) directly in this struct, not in a nested `data` field.
#[derive(Debug, Serialize, Deserialize)]
struct IndexJson {
    meta: Meta,
    #[serde(flatten)]
    data: IndexData,
}

#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    name: String,
    year: usize,
    version: usize,
    // This field isn't used, but we keep it for complete deserialization.
    // media_dir: PathBuf,
}

/// Your `Entry` struct for the database remains the same.
#[derive(Default, Deserialize, Serialize, Debug, Clone)]
pub struct Entry {
    pub expression: String,
    pub reading: Option<String>,
    pub source: String,
    pub speaker: Option<String>,
    pub display: String,
    pub file: String,
}

// --- SECTION 2: Core Database Logic ---

/// Creates the database table with the new, simplified schema.
async fn create_table_if_not_exists(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            expression TEXT NOT NULL,
            reading TEXT,
            source TEXT NOT NULL,
            speaker TEXT,
            display TEXT NOT NULL,
            file TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .with_context(|| "Failed to create the 'entries' table")?;
    Ok(())
}

/// Inserts a vector of Entry structs into the database within a single transaction.
async fn insert_entries(pool: &SqlitePool, entries: Vec<Entry>) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut transaction = pool
        .begin()
        .await
        .with_context(|| "Failed to begin database transaction")?;

    for entry in entries {
        sqlx::query(
            "INSERT into entries (expression, reading, source, speaker, display, file)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.expression)
        .bind(entry.reading)
        .bind(entry.source)
        .bind(entry.speaker)
        .bind(entry.display)
        .bind(entry.file)
        .execute(&mut *transaction)
        .await
        .context("Failed to insert entry into database")?;
    }

    transaction
        .commit()
        .await
        .context("Failed to commit database transaction")?;

    Ok(())
}

// --- SECTION 3: Public API for Indexing ---

/// The primary entry point for indexing a source from a JSON file.
/// This function handles file parsing, data transformation, and database insertion.
pub async fn index_file(pool: &SqlitePool, index_path: &Path) -> Result<String> {
    create_table_if_not_exists(pool).await?;

    let file = File::open(index_path)
        .with_context(|| format!("cant open index file at: {index_path:?}"))?;
    let reader = BufReader::new(file);

    // Now, this line can parse BOTH formats of index.json files!
    let index_json: IndexJson = serde_json::from_reader(reader)
        .with_context(|| format!("cant parse JSON from: {index_path:?}"))?;

    let mut src_name = index_json.meta.name;
    src_name = src_name
        .as_mut()
        .trim()
        .to_lowercase()
        .replace(" ", "-")
        .to_string();

    //println!("parsed index for source: '{src_name}'");

    if is_indexed(pool, &src_name).await? {
        ceprintln!("<cyan>[skipping]:</> '{src_name}'");
        return Ok(src_name);
    }

    ceprintln!("<cyan>[indexing]</>: {index_path:?}");
    // Here we convert the parsed data into the unified HashMap.
    let headwords = index_json.data.into_headwords_map();

    let mut entries_to_insert = Vec::new();
    for (expression, files) in headwords {
        for file in files {
            let entry = Entry {
                expression: expression.clone(),
                source: src_name.clone(),
                file,
                display: expression.clone(),
                ..Default::default()
            };
            entries_to_insert.push(entry);
        }
    }

    ceprintln!(
        "<g>found {} entries</> to insert @:'{src_name}'",
        entries_to_insert.len(),
    );
    insert_entries(pool, entries_to_insert).await?;
    ceprintln!("<g>[indexed]</>: '{src_name}'",);

    Ok(src_name)
}

/// Checks if any entries for a given source name already exist in the database.
pub async fn is_indexed(pool: &SqlitePool, source_name: &str) -> Result<bool> {
    create_table_if_not_exists(pool).await?;

    let result = sqlx::query("SELECT 1 FROM entries WHERE source = ? LIMIT 1")
        .bind(source_name)
        .fetch_optional(pool)
        .await
        .with_context(|| "Database query failed while checking if source is indexed")?
        .is_some();
    Ok(result)
}
