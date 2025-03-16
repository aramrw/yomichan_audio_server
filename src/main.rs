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
use config::spawn_headless;
use database::{DatabaseEntry, DbError};
use json::eprint_pretty;
use sqlx::SqlitePool;
use std::io::Write;
use std::path::Path;
use std::process;
use std::sync::mpsc;
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::OnceCell;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use tray_item::{IconSource, TrayItem};

pub(crate) struct ProgramInfo {
    pub pkg_name: String,
    pub version: String,
    pub current_exe: PathBuf,
    pub cli: Cli,
    pub db: SqlitePool,
}

pub(crate) static PROGRAM_INFO: OnceCell<ProgramInfo> = OnceCell::const_new();

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Initializing Server Info..");
    PROGRAM_INFO
        .get_or_init(async || {
            let buf = include_bytes!("../entries.db");
            let mut db_file = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open("entries.db")
                .unwrap();
            db_file.write_all(buf).unwrap();
            let version = env!("CARGO_PKG_VERSION").to_string();
            let current_exe = std::env::current_exe().unwrap();
            let cli = Cli::parse();
            let pkg_name = env!("CARGO_PKG_NAME").to_string();
            let db = SqlitePool::connect("entries.db").await.unwrap();

            ProgramInfo {
                pkg_name,
                version,
                current_exe,
                cli,
                db,
            }
        })
        .await;
    let pi = PROGRAM_INFO.get().unwrap();
    let pkg_name = &pi.pkg_name;

    println!("--debug-level: {:#?}", pi.cli.log);
    let print_debug_info_fn = || {
        debug!(port = ?pi.cli.port.inner, "\n   raw port:");
        debug!(name = %pkg_name, "\n   pkg_info");
    };
    let init_fulltrace_subscriber = || {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(format!("{}=trace", pkg_name,)))
            .init();
    };

    let audio_path = &PROGRAM_INFO.get().unwrap().cli.audio;
    if !audio_path.exists() {
        eprintln!(
            "\nThe `audio` folder was not found at: {}",
            audio_path.display()
        );
        eprintln!("Create one in the same folder as the exe, or run the exe with: `yomichan*.exe --audio <PATH>`\n");
        process::exit(1);
    }

    match pi.cli.log {
        CliLog::Headless => {
            println!("YOMICHAN_AUDIO_SERVER\n   --HEADLESS");
            spawn_headless();
            process::exit(0);
        }
        CliLog::HeadlessInstance => {}
        CliLog::Dev => {
            print_debug_info_fn();
            init_fulltrace_subscriber();
        }
        CliLog::Full => {
            std::env::set_var("RUST_BACKTRACE", "1");
            print_debug_info_fn();
            init_fulltrace_subscriber();
        }
    }
    config::kill_previous_instance();

    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(actix_files::Files::new(
                "/audio",
                &pi.cli.audio,
            ))
            .route("/", web::get().to(index))
    })
    .bind(&pi.cli.port.inner)?
    .run();

    print_greeting();

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
            return HttpResponse::from_error(std::io::Error::new(std::io::ErrorKind::Other, e));
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

enum Message {
    Quit,
    Debug,
}

async fn init_tray() {
    let pi = PROGRAM_INFO.get().unwrap();
    let mut tray = TrayItem::new(
        "Yomichan Audio Server",
        IconSource::Resource("tray-default"),
    )
    .unwrap();
    //tray.add_label("Tray Label").unwrap();
    let (tx, rx) = mpsc::sync_channel(1);

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
        tokio::time::sleep(Duration::from_millis(100)).await;
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

pub fn print_greeting() {
    #[allow(unused_variables)]
    let ProgramInfo {
        pkg_name,
        version,
        current_exe,
        cli,
        ..
    } = PROGRAM_INFO.get().unwrap();
    let port = &cli.port.inner;
    // program version
    println!("Yomichan Audio Server (v{version}) --");
    // port
    println!("   Port: {port}");
}
