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
use std::ffi::OsString;
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

impl AudioSourceMap {
    async fn create_source_map(audio_dir: &Path, db: &SqlitePool) -> Option<Self> {
        if !audio_dir.exists() {
            return None;
        }

        let mut audio_source_map = AudioSourceMap::default();
        let entries = read_dir(audio_dir).ok()?;

        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                ceprintln!("<r>[error]</> could not get file type");
                continue;
            };

            if !file_type.is_dir() {
                continue;
            }

            let dir_path = entry.path();

            let maybe_json_path: Option<PathBuf> = fs::read_dir(&dir_path)
                .ok() // Convert Result into an Option, discarding the error
                .and_then(|read_dir| {
                    read_dir
                        .flatten() // Ignore entries that cause an error
                        // Find the first entry that has a "json" extension
                        .find(|entry| entry.path().extension().map_or(false, |ext| ext == "json"))
                        // If an entry was found, get its path
                        .map(|entry| entry.path())
                });

            // Now you can use the discovered path
            let Some(found_path) = maybe_json_path else {
                ceprintln!("<r>[error]</> No <r>.json</> file found in: <r>{dir_path:?}</>");
                continue;
            };

            let index_path = found_path;

            if !index_path.exists() {
                continue;
            }

            match indexing::index_file(db, &index_path).await {
                Ok(source_name) => {
                    audio_source_map.insert(source_name, dir_path);
                }
                Err(e) => {
                    ceprintln!(
                        "<r>[error]</> Failed to index source in {:?}: {}",
                        dir_path,
                        e
                    );
                }
            }
        }
        Some(audio_source_map)
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

    // --- THIS IS THE FIX ---
    // Connect to the database. The `rwc` (read/write/create) mode will automatically
    // create the `entries.db` file if it does not exist. The old logic that
    // overwrote the database on every startup has been removed.
    let db = SqlitePool::connect("sqlite:entries.db?mode=rwc")
        .await
        .expect("Failed to connect to database. Ensure you have write permissions.");

    let audio_source_map = match AudioSourceMap::create_source_map(&cli.audio, &db).await {
        // The `if !map.is_empty()` part is a "match guard".
        // This arm only executes if we get `Some(map)` AND the map is NOT empty.
        Some(map) if !map.is_empty() => map,
        // 1. None & 2. Some(map) where the map IS empty.
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
