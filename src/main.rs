mod database;
use actix_web::{
    http::header::ContentType, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct AudioSource {
    name: String,
    url: String,
}


fn map_entry_object(word_object: &serde_json::Value) -> Option<database::Entry> {
    let mut word_entry = database::Entry::default();

    match word_object {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if key == "expression" {
                    word_entry.expression = value.as_str().unwrap_or("").to_string();
                } else if key == "reading" {
                    word_entry.reading = Some(value.as_str().unwrap_or("").to_string());
                } else if key == "source" {
                    word_entry.source = value.as_str().unwrap_or("").to_string();
                } else if key == "speaker" {
                    word_entry.speaker = Some(value.as_str().unwrap_or("").to_string());
                } else if key == "display" {
                    word_entry.display = value.as_str().unwrap_or("").to_string();
                } else if key == "file" {
                    word_entry.file = value.as_str().unwrap_or("").to_string();
                }
            }
        }
        _ => eprintln!("Not an object"),
    }

    //println!("{:?}", serde_json::to_string_pretty(&word_entry).unwrap());

    Some(word_entry)
}

fn find_audio_file(entry: &database::Entry) -> Option<AudioSource> {
    //println!("Searching for file: {}", entry.file);

    if entry.source == "shinmeikai8" {
        let shinmeikai_dir_path = "audio/shinmeikai8_files/media";
        let shinmeikai_dir = std::fs::read_dir(shinmeikai_dir_path);

        for file in shinmeikai_dir.unwrap() {
            let file = file.unwrap();
            if file.file_name() == *entry.file {
                //println!("Found file: {:?}", file.file_name());
                let audio_source =
                    construct_audio_source("smk", &entry.display, shinmeikai_dir_path, &entry.file);
                return Some(audio_source);
            }
        }
    }

    if entry.source == "nhk16" {
        let nhk16_dir_path = "audio/nhk16_files/media";
        let nhk16_dir = std::fs::read_dir(nhk16_dir_path);

        for file in nhk16_dir.unwrap() {
            let file = file.unwrap();
            if file.file_name() == *entry.file {
                //println!("Found file: {:?}", file.file_name());
                let audio_source =
                    construct_audio_source("nhk", &entry.display, nhk16_dir_path, &entry.file);
                return Some(audio_source);
            }
        }
    }

    if entry.source == "jpod" {
        let jpod_dir_path = "audio/jpod_files/audio";
        let jpod_dir = std::fs::read_dir(jpod_dir_path);

        for file in jpod_dir.unwrap() {
            let file = file.unwrap();
            if file.file_name() == *entry.file {
                //println!("Found file: {:?}", file.file_name());
                let audio_source =
                    construct_audio_source("jpod", "", jpod_dir_path, &entry.file);
                return Some(audio_source);
            }
        }
    }

    None
}

fn construct_audio_source(dict_name: &str, entry_display: &str, main_dir: &str, file_name: &str) -> AudioSource {
    if entry_display.is_empty() {
        return AudioSource {
            name: dict_name.to_string(),
            url: format!("http://localhost:8080/{}/{}", main_dir, file_name),
        };
    }

    let display = format!("{} - {}", dict_name, entry_display);
    AudioSource {
        name: display, 
        url: format!("http://localhost:8080/{}/{}", main_dir, file_name),
    }
}

async fn index(req: HttpRequest) -> impl Responder {
    let missing = "MISSING".to_string();
    // Access query parameters
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let term = query.get("term").unwrap_or(&missing);
    let reading = query.get("reading").unwrap_or(&missing);

    //println!("Term: {}", term.clone());
    //println!("Reading: {}", reading.clone());

    let result = database::query_database(term, reading).await.unwrap();
    let result_string = serde_json::to_string_pretty(&result).unwrap();
    let result_json: serde_json::Value = serde_json::from_str(&result_string).unwrap();
    let mut entries: Vec<database::Entry> = Vec::new();

    match result_json {
        serde_json::Value::Array(vec) => {
            for obj in vec {
                //println!("{}", obj);
                entries.push(map_entry_object(&obj).unwrap());
            }
        }
        _ => eprintln!("Not an object or array"),
    }

    // contruct the list of audio sources
    let mut audio_sources_list: Vec<AudioSource> = Vec::new();

    if !entries.is_empty() {
        for entry in entries {
            match find_audio_file(&entry) {
                Some(audio_source) => {
                    audio_sources_list.push(audio_source);
                }
                None => {
                    //println!("No audio source found");
                }
            }
        }
    }

    // https://github.com/FooSoft/yomichan/blob/master/ext/data/schemas/custom-audio-list-schema.json
    // construct the JSON response yomitan is expecting
    
    let resp = serde_json::json!({
        "type": "audioSourceList",
        "audioSources": audio_sources_list
    });

    // writing the JSON contents with UTF-8 encoding

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(resp)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(actix_files::Files::new("/audio", "audio"))
            .route("/", web::get().to(index))
    })
    .bind("localhost:8080")?
    .run()
    .await
}

// if entry.source == "forvo" {
//
//     let forvo_dir = std::fs::read_dir(
//     "C:\\Users\\arami\\AppData\\Roaming\\Anki2\\addons21\\1045800357\\user_files\\forvo_files",
// );
//
//     forvo_dir.unwrap().for_each(|file| {
//         let file = file.unwrap();
//         if file.file_name() == *entry.file {
//             println!("Found file: {:?}", file.file_name());
//         }
//     })
// }
