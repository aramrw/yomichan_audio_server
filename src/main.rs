mod database;
mod helper;
use actix_web::{
    http::header::ContentType, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use std::collections::HashMap;

async fn index(req: HttpRequest) -> impl Responder {
    let missing = "MISSING".to_string();
    // Access query parameters
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let term = query.get("term").unwrap_or(&missing);
    let reading = query.get("reading").unwrap_or(&missing);

    let result = database::query_database(term, reading).await.unwrap();
    let result_string = serde_json::to_string_pretty(&result).unwrap();
    let result_json: serde_json::Value = serde_json::from_str(&result_string).unwrap();
    let mut entries: Vec<database::Entry> = Vec::new();

    match result_json {
        serde_json::Value::Array(vec) => {
            for obj in vec {
                entries.push(helper::map_entry_object(&obj).unwrap());
            }
        }
        _ => eprintln!("Not an object or array"),
    }

    // contruct the list of audio sources
    let mut audio_sources_list: Vec<helper::AudioSource> = Vec::new();

    if !entries.is_empty() {
        for entry in entries {
            match helper::find_audio_file(&entry) {
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

    // Return the JSON response
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
