use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::Error as SqlxError;
use sqlx::{prelude::FromRow, sqlite::SqlitePool};
use std::fs::read_dir;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use tokio::join;

use crate::helper::{AudioFileError, AudioResult, AUDIO_FILE_STEMS, KANA_MAP};
use crate::PROGRAM_INFO;

#[derive(Default, Deserialize, Serialize, Debug, FromRow, Clone)]
pub struct DatabaseEntry {
    pub expression: String,
    pub reading: Option<String>,
    pub source: AudioSource,
    pub speaker: Option<String>,
    pub display: String,
    pub file: String,
}

impl DatabaseEntry {
    pub fn strip_folder_name_prefix(&mut self) {
        // file _might_ start with the folder name so cut it out
        let file_path = Path::new(&self.file);
        if let Some(file_name) = file_path.file_name() {
            self.file = file_name.to_string_lossy().to_string();
        }
    }

    /// recursively searches a directory searching for nested folders (ignoring files) and
    /// formats the audio path by appending the file name to each directory path.
    ///
    /// 詰まり..for each directory, it checks if the file exists
    /// without needing to loop over every file.
    pub fn find_audio_file(&self, dir: impl AsRef<Path>) -> Result<PathBuf, AudioFileError> {
        let format = |p: &Path| p.join(&self.file);
        for item in read_dir(&dir).unwrap() {
            let item = item.unwrap();
            let path = item.path();
            let p_name = path.file_name().unwrap().to_str().unwrap();
            if path.is_dir() {
                let f = format(&path);
                let f_name = f.file_name().unwrap().to_str().unwrap();
                if f.exists() && f_name == self.file {
                    if p_name == self.display || p_name == "media" {
                        return Ok(f);
                    }
                } else if let Ok(f) = self.find_audio_file(&path) {
                    return Ok(f);
                }
            }
        }
        return Err(AudioFileError::MissingAudioFile {
            entry: self.clone(),
            dir: dir.as_ref().display().to_string(),
        });
    }

    // Construct the audio source based on the file path
    pub fn to_audio_result(&self) -> Result<AudioResult, AudioFileError> {
        let entry = &self;
        let DatabaseEntry {
            source,
            display,
            file,
            ..
        } = entry;

        let main_dir = format!("audio/{}", source.to_string());
        let mut file_path = Path::new(&format!("{}/media", &main_dir)).join(&file);
        if !file_path.exists() {
            file_path = Path::new(&format!("{}/{}", &main_dir, self.display)).join(&file);
            if !file_path.exists() {
                let p = self.find_audio_file(&main_dir)?;
                file_path = p;
            }
        }

        // if is forvo file
        if display.is_empty() {
            let res = AudioResult {
                name: source.to_string(),
                url: format!(
                    "http://{}/{}",
                    PROGRAM_INFO.cli.port.inner,
                    &file_path.display(),
                ),
            };
            return Ok(res);
        }

        // dict files
        let display = format!("{} {}", source, display);
        let res = AudioResult {
            name: display,
            url: format!("http://localhost:8080/{}", file_path.display()),
        };

        Ok(res)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AudioSourceError {
    #[error("unknown audio source: {src}")]
    UnkownSource { src: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
pub enum AudioSource {
    #[sqlx(rename = "forvo_jp")]
    ForvoJp,
    #[sqlx(rename = "forvo_zh")]
    ForvoZh,
    Shinmeikai8,
    Nhk16,
    #[default]
    Daijisen,
    Jpod,
}

impl std::fmt::Display for AudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dbg = match self {
            Self::ForvoJp => "forvo_jp",
            Self::ForvoZh => "forvo_zh",
            _ => &format!("{self:?}").to_lowercase(),
        };
        write!(f, "{dbg}")
    }
}

impl std::str::FromStr for AudioSource {
    type Err = AudioSourceError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "forvo_jp" => Ok(AudioSource::ForvoJp),
            "forvo_zh" => Ok(AudioSource::ForvoZh),
            "shinmeikai8" => Ok(AudioSource::Shinmeikai8),
            "nhk16" => Ok(AudioSource::Nhk16),
            "daijisen" => Ok(AudioSource::Daijisen),
            "jpod" => Ok(AudioSource::Jpod),
            _ => Err(AudioSourceError::UnkownSource { src: s.to_string() }),
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
) -> Result<Vec<DatabaseEntry>, sqlx::Error> {
    sqlx::query_as::<_, DatabaseEntry>(
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
) -> color_eyre::Result<Vec<DatabaseEntry>> {
    let fetch_dict_result = sqlx::query_as::<_, DatabaseEntry>(
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
            query_forvo_base("forvo_jp", term, pool)
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

    let mut query_entries: Vec<DatabaseEntry> = Vec::with_capacity(de_len + fe_len);
    query_entries.extend(dict_entries.into_iter().chain(forvo_entries.into_iter()));

    query_entries.par_sort_unstable_by(|a, b| {
        let order = ["daijisen", "nhk16", "shinmeikai8", "forvo", "jpod"];

        // Find the index or use a default value if not found
        let a_index = order
            .iter()
            .position(|&x| x == a.source.to_string())
            .unwrap_or(order.len());
        let b_index = order
            .iter()
            .position(|&x| x == b.source.to_string())
            .unwrap_or(order.len());

        // Compare the indices
        a_index.cmp(&b_index)
    });

    Ok(query_entries)
}

fn index_files(dir: impl AsRef<Path>) -> Vec<&'static str> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in read_dir(dir).unwrap().into_iter() {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();

        if path.is_file() {
            if let Some(stem) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|stem| AUDIO_FILE_STEMS.get(stem).copied())
            {
                files.push(stem);
            }
        } else if path.is_dir() {
            dirs.push(path);
        }
    }
    let d_files = dirs
        .into_par_iter()
        .flat_map(|dir| index_files(dir))
        .collect::<Vec<_>>();
    files.extend(d_files);
    files
}

#[cfg(test)]
mod db {
    use sqlx::SqlitePool;

    use super::{index_files, query_database};
    use crate::{database::DatabaseEntry, helper::AudioResult};
    use pretty_assertions::assert_eq;
    use std::time::Instant;

    #[test]
    fn find_audio_file() {
        let e = DatabaseEntry {
            expression: "日本語".to_string(),
            reading: Some("にほんご".to_string()),
            source: super::AudioSource::ForvoJp,
            speaker: Some("strawberrybrown".to_string()),
            display: "strawberrybrown".to_string(),
            file: "日本語.mp3".to_string(),
        };
        let instant = Instant::now();
        e.find_audio_file("F:/Programming/Rust/yomichan_http_server/audio")
            .unwrap();
        println!("sync_elapsed: {:?}", instant.elapsed());
        e.par_find_audio_files("audio").unwrap();
        println!("par_elapsed: {:?}", instant.elapsed());
    }

    #[test]
    fn index_audio() {
        let start = Instant::now();
        let res = index_files("audio");
        assert!(!res.is_empty());
        println!("elapsed: {:?}", start.elapsed());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn count_entries() {
        let pool = SqlitePool::connect("audio/entries.db").await.unwrap();
        let entries: Vec<DatabaseEntry> = sqlx::query_as("SELECT * FROM entries")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(entries.len(), 924_637);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_query() {
        tracing_subscriber::fmt::init();
        let instant = Instant::now();
        let term = "本";
        let reading = "ほん";
        let pool = SqlitePool::connect("audio/entries.db").await.unwrap();
        let entries = query_database(term, reading, &pool).await.unwrap();
        assert!(!entries.is_empty());

        let audio_source_list = AudioResult::create_list(entries.as_slice());
        AudioResult::print_list(&audio_source_list);

        tracing::info!("\nelapsed: {:.3}ms\n", instant.elapsed().as_millis());
    }
}
