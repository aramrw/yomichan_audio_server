mod cli;
mod config;
mod database;
mod helper;

//use crate::database::Entry;
use crate::helper::AudioSource;

use actix_web::{
    http::header::ContentType, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use clap::Parser;
use cli::{Cli, CliLog};
use color_eyre::eyre::eyre;
use config::spawn_headless;
use database::{DbError, Entry};
use sqlx::SqlitePool;
use std::path::Path;
use std::process;
use std::sync::{mpsc, LazyLock};
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf};
use tracing::debug;
use tracing_subscriber::EnvFilter;
use tray_item::{IconSource, TrayItem};

pub(crate) struct ProgramInfo {
    pub pkg_name: String,
    pub version: String,
    pub current_exe: PathBuf,
    pub cli: Cli,
}

pub(crate) static PROGRAM_INFO: LazyLock<ProgramInfo> = LazyLock::new(|| {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let current_exe = std::env::current_exe().unwrap();
    let cli = Cli::parse();
    let pkg_name = env!("CARGO_PKG_NAME").to_string();

    ProgramInfo {
        pkg_name,
        version,
        current_exe,
        cli,
    }
});

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pkg_name = &PROGRAM_INFO.pkg_name;

    println!("--debug-level: {:#?}", &PROGRAM_INFO.cli.log);
    let print_debug_info_fn = || {
        debug!(port = ?PROGRAM_INFO.cli.port.inner, "\n   raw port:");
        debug!(name = %PROGRAM_INFO.pkg_name, "\n   pkg_info");
    };
    let init_fulltrace_subscriber = || {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(format!("{}=trace", pkg_name,)))
            .init();
    };

    match &PROGRAM_INFO.cli.log {
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
            .service(actix_files::Files::new("/audio", "audio"))
            .route("/", web::get().to(index))
    })
    .bind(&*PROGRAM_INFO.cli.port.inner)?
    .run();

    print_greeting();

    // macOS doesnt allow running programs in threads other than main,
    // -> it is not possible to listen for events in a new thread
    #[cfg(target_os = "windows")]
    tokio::spawn(async move {
        init_tray().await;
    });

    server.await
}

async fn index(req: HttpRequest) -> impl Responder {
    let missing = "MISSING".to_string();

    // access query parameters
    let query =
        match actix_web::web::Query::<HashMap<String, String>>::from_query(req.query_string()) {
            Ok(q) => q,
            Err(e) => return HttpResponse::from_error(e),
        };
    let instant = std::time::Instant::now();
    let term = query.get("term").unwrap_or(&missing);
    let reading = query.get("reading").unwrap_or(&missing);

    match Path::new("./audio").exists() {
        true => {
            if !Path::new("./audio/entries.db").exists() {
                let report = eyre!("{}", DbError::MissingEntriesDB);
                eprintln!("{:?}", report);
            }
        }
        false => {
            let report = eyre!(
                "{}",
                DbError::MissingAudioFolder(PROGRAM_INFO.current_exe.clone())
            );
            eprintln!("{:?}", report);
        }
    }

    // should use a real error for more context;
    let pool = SqlitePool::connect("./audio/entries.db").await.unwrap();
    let entries: Vec<Entry> = match database::query_database(term, reading, &pool).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("{:?}", e);
            Vec::new()
        }
    };

    let audio_source_list = AudioSource::create_list(&entries);

    match PROGRAM_INFO.cli.log {
        CliLog::Dev | CliLog::Full => {
            println!();
            let span = tracing::span!(tracing::Level::INFO, 
                "serving\n  ", term=%term, reading=%reading);
            let _enter = span.enter();

            tracing::debug!(
                "( {:.3}ms ) .. c={}",
                instant.elapsed().as_millis(),
                audio_source_list.len()
            );
            AudioSource::print_list(&audio_source_list);
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
    let mut tray = TrayItem::new(
        "Yomichan Audio Server",
        IconSource::Resource("tray-default"),
    )
    .unwrap();
    //tray.add_label("Tray Label").unwrap();
    let (tx, rx) = mpsc::sync_channel(1);

    let debug_tx = tx.clone();
    #[allow(clippy::single_match)]
    match PROGRAM_INFO.cli.log {
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
    } = &*PROGRAM_INFO;
    let port = &cli.port.inner;
    // program version
    println!("Yomichan Audio Server (v{version}) --");
    // port
    println!("   Port: {port}");
}
