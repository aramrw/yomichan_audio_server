#![allow(unused_imports, clippy::result_large_err)]
mod cli;
mod config;
mod database;
mod helper;
mod indexing;

use crate::helper::AudioResult;

use actix_web::{
    http::header::ContentType, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};

use clap::Parser;
use cli::{Cli, CliLog};
use color_eyre::eyre::eyre;
use color_eyre::owo_colors::OwoColorize;
use color_print::{ceprintln, cprintln};
use config::spawn_headless;
use database::DatabaseEntry;

use rapidhash::RapidHashMap;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::fs::{self, read_dir, File, FileType};
use std::io::{self, Error, ErrorKind, Write};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::process;
use std::str::FromStr;
use std::{collections::HashMap, path::PathBuf};
use strum::EnumIter;
use tokio::sync::OnceCell;
use tracing::debug;
use tracing_subscriber::EnvFilter;
#[cfg(target_os = "windows")]
use tray_item::{IconSource, TrayItem};
use walkdir::WalkDir;

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! eprint_pretty {
        ($e:expr) => {
            let r = eyre!("{}", $e);
            eprintln!("{:?}", r);
        };
    }
}

#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, sqlx::Type, EnumIter, Hash,
)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
pub enum AudioSource {
    #[default]
    Daijisen,
    Nhk16,
    Shinmeikai8,
    Jpod,
    #[sqlx(rename = "forvo_jp")]
    ForvoJp,
    #[sqlx(rename = "forvo_zh")]
    ForvoZh,
    #[sqlx(rename = "forvo_es")]
    ForvoEs,
    Other,
}

impl Eq for AudioSource {}

impl std::fmt::Display for AudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dbg = match self {
            Self::ForvoJp => "forvo_jp",
            Self::ForvoZh => "forvo_zh",
            Self::ForvoEs => "forvo_es",
            _ => &format!("{self:?}").to_lowercase(),
        };
        write!(f, "{dbg}")
    }
}

impl FromStr for AudioSource {
    type Err = AudioSource;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_lower = s.to_lowercase();
        match s_lower.as_str() {
            s if s.contains("forvo_jp") => Ok(AudioSource::ForvoJp),
            s if s.contains("forvo_zh") => Ok(AudioSource::ForvoZh),
            s if s.contains("forvo_es") => Ok(AudioSource::ForvoEs),
            s if s.contains("shinmeikai") => Ok(AudioSource::Shinmeikai8),
            s if s.contains("nhk") => Ok(AudioSource::Nhk16),
            s if s.contains("daijisen") => Ok(AudioSource::Daijisen),
            s if s.contains("jpod") => Ok(AudioSource::Jpod),
            _ => Ok(AudioSource::Other),
        }
    }
}

pub(crate) struct ProgramInfo {
    pub pkg_name: String,
    pub version: String,
    pub current_exe: PathBuf,
    pub cli: Cli,
    pub db: SqlitePool,
    pub sort: Vec<String>,
    pub audio_source_map: AudioSourceMap,
}

#[derive(Default, PartialEq)]
struct AudioSourceMap {
    pub map: RapidHashMap<String, PathBuf>,
}
impl Eq for AudioSourceMap {}
impl Deref for AudioSourceMap {
    type Target = RapidHashMap<String, PathBuf>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl DerefMut for AudioSourceMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

struct DirectoryLayout {
    has_top_level_audio: bool,
    subdirs: Vec<PathBuf>,
}
impl AudioSourceMap {
    /// The main entry point. Iterates through the top-level directories in the audio folder.
    async fn create_source_map(audio_dir: &Path, db: &SqlitePool) -> Option<Self> {
        if !audio_dir.exists() {
            return None;
        }

        let mut audio_source_map = AudioSourceMap::default();
        let Ok(entries) = read_dir(audio_dir) else {
            return None;
        };

        for entry in entries.flatten() {
            if entry.path().is_dir() {
                // Process each directory individually.
                audio_source_map.process_directory(&entry.path(), db).await;
            }
        }
        Some(audio_source_map)
    }

    /// Determines how to handle a single directory.
    async fn process_directory(&mut self, dir_path: &Path, db: &SqlitePool) {
        let dir_name_os = dir_path.file_name().unwrap_or_default();
        let dir_name = dir_name_os.to_string_lossy();

        // Case 1: An 'index.json' already exists. This is the highest priority.
        let index_path = dir_path.join("index.json");
        if index_path.exists() {
            if let Ok(source_name) = indexing::index_file(db, &index_path).await {
                self.insert(source_name, dir_path.to_path_buf());
            }
            return;
        }

        // Case 2: An 'entries.json' exists. Warn the user and skip.
        let entries_json_path = dir_path.join("entries.json");
        if entries_json_path.exists() {
            ceprintln!("<y>[warn]</> Directory '{}' contains 'entries.json'. Please rename to 'index.json'.", dir_name);
            return;
        }

        // Case 3: No index file. Analyze the directory contents to decide the next step.
        let layout = self.analyze_directory_contents(dir_path);

        if layout.has_top_level_audio {
            // Treat as a single source with audio files at its root.
            self.handle_single_source_dir(dir_path, &dir_name, db).await;
        } else if !layout.subdirs.is_empty() {
            // Treat as a multi-source directory where each subdirectory is a source.
            self.handle_multi_source_dir(&dir_name, &layout.subdirs, db)
                .await;
        } else {
            ceprintln!(
                "<y>[warn]</> Skipping directory '{}' (no index or audio files found).",
                dir_name
            );
        }
    }

    /// Analyzes a directory's immediate contents to classify it.
    fn analyze_directory_contents(&self, dir_path: &Path) -> DirectoryLayout {
        let mut layout = DirectoryLayout {
            has_top_level_audio: false,
            subdirs: vec![],
        };

        if let Ok(dir_contents) = read_dir(dir_path) {
            for entry in dir_contents.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    layout.subdirs.push(path);
                } else if !layout.has_top_level_audio {
                    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
                        if ["mp3", "wav", "ogg", "flac", "mp4"]
                            .contains(&ext.to_lowercase().as_str())
                        {
                            layout.has_top_level_audio = true;
                        }
                    }
                }
            }
        }
        layout
    }

    /// Handles a directory with audio files directly inside it.
    async fn handle_single_source_dir(&mut self, dir_path: &Path, dir_name: &str, db: &SqlitePool) {
        if let Ok(new_path) = indexing::create_index_from_directory(dir_path, dir_name).await {
            if let Ok(source_name) = indexing::index_file(db, &new_path).await {
                self.insert(source_name, dir_path.to_path_buf());
            }
        }
    }

    /// Handles a directory that contains multiple source subdirectories.
    async fn handle_multi_source_dir(
        &mut self,
        parent_name: &str,
        subdirs: &[PathBuf],
        db: &SqlitePool,
    ) {
        ceprintln!(
            "<cyan>[multi-source]</> Scanning inside '{}'...",
            parent_name
        );
        for sub_dir_path in subdirs {
            let sub_dir_name = sub_dir_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let combined_source_name = format!("{}-{}", parent_name, sub_dir_name);

            if let Ok(new_path) =
                indexing::create_index_from_directory(sub_dir_path, &combined_source_name).await
            {
                if let Ok(source_name) = indexing::index_file(db, &new_path).await {
                    self.insert(source_name, sub_dir_path.clone());
                }
            }
        }
    }
}

pub(crate) static PROGRAM_INFO: OnceCell<ProgramInfo> = OnceCell::const_new();
pub(crate) async fn init_program() -> ProgramInfo {
    fn print_arg(arg: &str, x: impl Debug) {
        cprintln!("<b>--{arg}</>: {x:?}");
    }

    // init program data
    let version = env!("CARGO_PKG_VERSION").to_string();
    cprintln!("[yomichan audio server <y>v{version}</>]");
    let current_exe = std::env::current_exe().unwrap();
    let cli = Cli::parse();
    let pkg_name = env!("CARGO_PKG_NAME").to_string();
    print_arg("port", &cli.port.inner);
    print_arg("log", cli.log);

    // create the `entries.db` file if it does not exist. The old logic that
    // overwrote the database on every startup has been removed.
    let db = SqlitePool::connect("sqlite:entries.db?mode=rwc")
        .await
        .expect("Failed to connect to database. Ensure you have write permissions.");

    let audio_source_map = match AudioSourceMap::create_source_map(&cli.audio, &db).await {
        Some(map) if !map.is_empty() => map,
        _ => {
            ceprintln!(
                "<r>[panic]</> No recognizable audio source folders found in '{}'",
                cli.audio.display()
            );
            panic!();
        }
    };

    ceprintln!("<g>[ready]</>");
    let sort = Vec::new();
    ProgramInfo {
        audio_source_map,
        pkg_name,
        version,
        current_exe,
        cli,
        db,
        sort,
    }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if cli.log == CliLog::Headless {
        config::kill_previous_instance();
        config::spawn_headless(&cli);
        cprintln!("<g>✓</> [started]");
        std::process::exit(0);
    }

    PROGRAM_INFO.get_or_init(init_program).await;
    let pi = PROGRAM_INFO.get().unwrap();

    let pkg_name = &pi.pkg_name;

    let init_fulltrace_subscriber = || {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(format!("{}=trace", pkg_name,)))
            .init();
    };

    let audio_path = &pi.cli.audio;
    if !audio_path.exists() {
        let default = OsString::from("yas*");
        let current_exe = pi.current_exe.file_name().unwrap_or(&default).display();
        ceprintln!(
            "\n<r>[error]</> The 'audio' folder was not found at: {}",
            audio_path.display()
        );
        ceprintln!("<cyan>[help]</> create one in the same folder as the exe",);
        ceprintln!("<cyan>[help]</> or run with: {current_exe} <b>--audio PATH</>");

        process::exit(1);
    }
    let paths: Vec<PathBuf> = read_dir(audio_path)?.flatten().map(|f| f.path()).collect();
    if paths.is_empty() {
        ceprintln!(
            "\n<r>[error]</> {audio_path:?} folder contains no files to serve <r>[EXITCODE 1]</r>"
        );
        process::exit(1);
    }

    match pi.cli.log {
        CliLog::Headless => {
            spawn_headless(&cli);
            process::exit(0);
        }
        CliLog::HeadlessInstance => {}
        CliLog::Dev => {
            init_fulltrace_subscriber();
        }
        CliLog::Full => {
            std::env::set_var("RUST_BACKTRACE", "1");
            init_fulltrace_subscriber();
        }
    }

    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(actix_files::Files::new("/audio", &pi.cli.audio))
            .route("/", web::get().to(index))
    })
    .bind(&pi.cli.port.inner)?
    .run();

    #[cfg(target_os = "windows")]
    tokio::spawn(async move {
        init_tray().await;
    });

    server.await
}

async fn index(req: HttpRequest) -> impl Responder {
    let pi = &PROGRAM_INFO.get().unwrap();
    // access query parameters
    let query =
        match actix_web::web::Query::<HashMap<String, String>>::from_query(req.query_string()) {
            Ok(q) => q,
            Err(e) => return HttpResponse::from_error(e),
        };
    let start = std::time::Instant::now();
    let (Some(term), Some(reading)) = (query.get("term"), query.get("reading")) else {
        return HttpResponse::BadRequest().body("Missing query parameters: 'term' and 'reading'.");
    };

    // if !program.exists() {
    //     let e = DbError::MissingAudioFolder(pi.current_exe.clone());
    //     println!();
    //     eprint_pretty!(e);
    //     std::process::exit(1);
    // }

    let entries: Vec<DatabaseEntry> = match database::query_database(term, reading).await {
        Ok(res) => res,
        Err(e) => {
            eprint_pretty!(e);
            return HttpResponse::from_error(Error::other(e));
        }
    };

    let audio_source_list = AudioResult::create_list(&entries);

    match pi.cli.log {
        CliLog::Dev | CliLog::Full => {
            println!();
            let span = tracing::span!(tracing::Level::INFO,
                "serving\n  ", term=%term, reading=%reading);
            let _enter = span.enter();

            tracing::debug!(
                "( {:.3}ms ) .. c={}",
                start.elapsed().as_millis(),
                audio_source_list.len()
            );
            AudioResult::print_list(&audio_source_list);
        }
        _ => {}
    }

    // github.com/FooSoft/yomichan/blob/master/ext/data/schemas/custom-audio-list-schema.json
    // JSON response yomitan is expecting

    let resp = serde_json::json!({
        "type": "audioSourceList",
        "audioSources": audio_source_list
    });

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(resp)
}

#[cfg(target_os = "windows")]
enum Message {
    Quit,
    Debug,
}

#[cfg(target_os = "windows")]
async fn init_tray() {
    let pi = PROGRAM_INFO.get().unwrap();
    let mut tray = TrayItem::new(
        "Yomichan Audio Server",
        IconSource::Resource("tray-default"),
    )
    .unwrap();
    //tray.add_label("Tray Label").unwrap();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);

    let debug_tx = tx.clone();
    #[allow(clippy::single_match)]
    match pi.cli.log {
        CliLog::Headless => {
            #[cfg(target_os = "windows")]
            tray.add_menu_item("Debug", move || {
                debug_tx.send(Message::Debug).unwrap();
            })
            .unwrap();
        }
        _ => {}
    }

    let quit_tx = tx.clone();
    tray.add_menu_item("Quit", move || {
        quit_tx.send(Message::Quit).unwrap();
    })
    .unwrap();

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Ok(msg) = rx.recv() {
            match msg {
                Message::Quit => process::exit(0),
                Message::Debug => {
                    spawn_headless();
                }
            }
        }
    }
}
