mod config;
mod database;
mod error;
mod helper;

//use crate::database::Entry;
use crate::helper::AudioSource;

use actix_web::{
    http::header::ContentType, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use config::handle_debugger;
use database::Entry;
use rayon::prelude::*;
use std::collections::HashMap;
use std::process;
use std::sync::mpsc;
use std::time::Duration;
use tokio::io::stderr;
use tray_item::{IconSource, TrayItem};

async fn index(req: HttpRequest) -> impl Responder {
    let missing = "MISSING".to_string();
    // Access query parameters
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let term = query.get("term").unwrap_or(&missing);
    let reading = query.get("reading").unwrap_or(&missing);
    println!("term: {term} | reading: {reading}");

    let entries: Vec<Entry> = match database::query_database(term, reading).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error Querying the Database: {e}");
            Vec::new()
        }
    };
    // contruct the list of audio sources
    let mut audio_sources_list: Vec<AudioSource> = Vec::new();

    if !entries.is_empty() {
        print!("\n{:#?}\n", entries);
        let audio_files_res: Vec<AudioSource> = entries
            .par_iter()
            .filter_map(helper::find_audio_file) // Directly filter out `None` values
            .collect();
        audio_sources_list = audio_files_res;
    }

    // https://github.com/FooSoft/yomichan/blob/master/ext/data/schemas/custom-audio-list-schema.json
    // construct the JSON response yomitan is expecting

    let resp = serde_json::json!({
        "type": "audioSourceList",
        "audioSources": audio_sources_list
    });

    // Return the JSON response
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(resp)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    config::kill_previous_instance();

    match get_args() {
        Some(arg) => {
            // Starts the server again except with the "debug" arg.
            println!("{arg} INSTANCE");
        }
        // If this is the first run (i.e., no "hidden" or "debug" arguments), start a hidden instance.
        None => {
            println!("FIRST RUN INSTANCE");
            handle_debugger(true)
        }
    }

    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(actix_files::Files::new("/audio", "audio"))
            .route("/", web::get().to(index))
    })
    .bind("localhost:8080")?
    .run();

    // MacOS does not allow running applications in threads other than main,
    // meaning that it is not possible to listen for events in a new thread
    #[cfg(target_os = "windows")]
    tokio::spawn(async move {
        init_tray().await;
    });

    server.await
}

enum Message {
    Quit,
    Debug,
}

fn get_args() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        return args.get(1).cloned();
    }
    None
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
    if let Some(arg) = get_args() {
        if arg != "debug" {
            #[cfg(target_os = "windows")]
            tray.add_menu_item("Debug", move || {
                debug_tx.send(Message::Debug).unwrap();
            })
            .unwrap();
        }
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
                    handle_debugger(false);
                }
            }
        }
    }
}
