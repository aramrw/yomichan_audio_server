#![allow(unused_imports, clippy::result_large_err)]
mod cli;
mod config;
mod database;
mod helper;

use crate::helper::AudioResult;

use actix_web::{
    http::header::ContentType, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};

use clap::Parser;
use cli::{Cli, CliLog};
use color_eyre::eyre::eyre;
use color_print::{ceprintln, cprintln};
use config::spawn_headless;
use database::{AudioSource, DatabaseEntry};
use json::eprint_pretty;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::ffi::OsString;
use std::fmt::Debug;
use std::fs::{self, read_dir, File};
use std::io::{self, Error, ErrorKind, Write};
use std::path::Path;
use std::process;
use std::str::FromStr;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::OnceCell;
use tracing::debug;
use tracing_subscriber::EnvFilter;
#[cfg(target_os = "windows")]
use tray_item::{IconSource, TrayItem};

pub(crate) struct ProgramInfo {
    pub pkg_name: String,
    pub version: String,
    pub current_exe: PathBuf,
    pub cli: Cli,
    pub db: SqlitePool,
    pub sort: Vec<AudioSource>,
    pub file_cache: HashMap<String, PathBuf>,
}

pub(crate) static PROGRAM_INFO: OnceCell<ProgramInfo> = OnceCell::const_new();
async fn init_program() -> ProgramInfo {
    let dbpath = Path::new("./entries.db");
    if !dbpath.exists() {
        println!("you are missing an entries.db file in the main directory.\ndownload the latest entries.db:\nhttps://github.com/aramrw/yomichan_audio_server/releases/download/v0.0.1/entries.db");
    }

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

    // init database - use existing entries.db file
    if !dbpath.exists() {
        eprintln!("ERROR: entries.db not found in working directory!");
        std::process::exit(1);
    }
    let db = SqlitePool::connect("entries.db").await.unwrap();

    let sort = AudioSource::read_sort_file();
    
    // Build file cache at startup for fast lookups
    cprintln!("<cyan>[info]</> Building audio file cache...");
    let cache_start = std::time::Instant::now();
    let file_cache = build_file_cache(&cli.audio);
    cprintln!("<g>[done]</> Cached {} files in {:.2}s", file_cache.len(), cache_start.elapsed().as_secs_f64());
    
    ProgramInfo {
        pkg_name,
        version,
        current_exe,
        cli,
        db,
        sort,
        file_cache,
    }
}

/// Metadata to track changes. We only track the top-level folder modification time
/// and the count of items in it to detect added/removed source folders quickly.
#[derive(Serialize, Deserialize)]
struct CacheMetadata {
    last_modified: std::time::SystemTime,
}

/// Recursively walks the audio directory and builds a cache of filename -> full path
/// Cache is persisted to disk and automatically rebuilds when top-level audio directory changes
fn build_file_cache(audio_dir: &Path) -> HashMap<String, PathBuf> {
    use rayon::prelude::*;
    
    let cache_file = Path::new("./audio_cache.json");
    let metadata_file = Path::new("./audio_cache_metadata.json");
    
    // Only look at the audio directory itself
    // This catches adding/removing source folders, which is the main use case
    let current_modified = audio_dir.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::now());
    
    let should_rebuild = (|| {
        // Try to read and parse metadata
        let meta_contents = std::fs::read_to_string(metadata_file).ok()?;
        let metadata = serde_json::from_str::<CacheMetadata>(&meta_contents).ok()?;
        
        // If the modification time of 'audio/' is different, something changed
        let changed = metadata.last_modified != current_modified;
        if changed {
            cprintln!("<y>[cache]</> Audio library changed, rebuilding cache...");
        }
        
        // Return true if changed, false if not changed
        // This is wrapped in Some(bool) which the closure returns
        Some(changed)
    })()
    // If any step failed (returned None), default to true (rebuild)
    .unwrap_or(true);
    
    // Try to load existing cache if valid
    if !should_rebuild {
        if let Ok(cache_contents) = std::fs::read_to_string(cache_file) {
            if let Ok(cache) = serde_json::from_str::<HashMap<String, PathBuf>>(&cache_contents) {
                cprintln!("<g>[cache]</> Loaded {} files from cache", cache.len());
                return cache;
            }
        }
    }
    
    // Cache doesn't exist or is invalid, build it
    fn walk_dir(dir: &Path, cache: &mut HashMap<String, PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk_dir(&path, cache);
                } else if path.is_file() {
                    if let Some(filename) = path.file_name() {
                        let filename_str = filename.to_string_lossy().to_string();
                        // Only cache audio files, don't overwrite existing entries
                        // (first found wins, matching the original search behavior)
                        cache.entry(filename_str).or_insert(path);
                    }
                }
            }
        }
    }
    
    let mut cache = HashMap::new();
    walk_dir(audio_dir, &mut cache);
    
    // Save cache to disk for next startup
    if let Ok(cache_json) = serde_json::to_string(&cache) {
        let _ = std::fs::write(cache_file, cache_json);
        cprintln!("<g>[cache]</> Saved {} files to cache", cache.len());
    }
    
    // Save metadata (only the root dir modified time)
    let metadata = CacheMetadata { last_modified: current_modified };
    if let Ok(metadata_json) = serde_json::to_string(&metadata) {
        let _ = std::fs::write(metadata_file, metadata_json);
    }
    
    cache
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    PROGRAM_INFO.get_or_init(init_program).await;
    let pi = PROGRAM_INFO.get().unwrap();

    if pi.cli.sources {
        AudioSource::display_all_variants();
        process::exit(0);
    }

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
            spawn_headless();
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
