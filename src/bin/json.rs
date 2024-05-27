use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::collections::hash_map::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Default, Deserialize, Serialize, Debug)]
pub struct Entry {
    pub expression: String,
    pub reading: Option<String>,
    pub source: String,
    pub speaker: Option<String>,
    pub display: String,
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
struct EntryFile {
    kana_reading: String,
    pitch_pattern: String,
    pitch_number: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexJson {
    meta: Meta,
    headwords: HashMap<String, Vec<String>>,
    files: HashMap<String, EntryFile>,
}

#[tokio::main]
async fn main() {
    const INDEX_PATH: &str = "C:\\Users\\arami\\Desktop\\index.json";
    let file = File::open(INDEX_PATH).unwrap();
    let reader = BufReader::new(file);
    let stream = Deserializer::from_reader(reader).into_iter::<IndexJson>();

    let pool = sqlx::SqlitePool::connect("F:\\Programming\\Rust\\yomichan_http_server\\audio\\entries.db")
        .await
        .unwrap();
    create_test_table(&pool).await;

    let mut transaction = pool.begin().await.unwrap();
    let mut count = 0;

