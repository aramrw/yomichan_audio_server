mod config;
mod database;
mod helper;

use crate::database::Entry;
use crate::helper::AudioSource;

use actix_web::{
    http::header::ContentType, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

async fn index(req: HttpRequest) -> impl Responder {
    let missing = "MISSING".to_string();
    // Access query parameters
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let term = query.get("term").unwrap_or(&missing);
    let reading = query.get("reading").unwrap_or(&missing);

    let entries: Vec<Entry> = database::query_database(term, reading).await.unwrap();
    // contruct the list of audio sources
    let mut audio_sources_list: Vec<Option<AudioSource>> = Vec::new();

    if !entries.is_empty() {
        let audo_files_res: Vec<Option<AudioSource>> = entries
            .par_iter()
            .map(helper::find_audio_file)
            .collect();
        audio_sources_list = audo_files_res;
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
    let config = config::create_config();
    config::handle_debugger(&config);
    config::kill_previous_instance();

    let timer = Instant::now();

    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(actix_files::Files::new("/audio", "audio"))
            .route("/", web::get().to(index))
    })
    .bind("localhost:8080")?
    .run();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await; // Check every minute
            if timer.elapsed().as_secs() / 60 >= config.exit_minutes {
                println!("Exiting after {} minutes", config.exit_minutes);
                std::process::exit(0);
            }
        }
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
