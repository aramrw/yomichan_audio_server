
use std::collections::HashMap;

use actix_web::{web, App, HttpRequest, HttpServer, Responder, middleware};

async fn index(req: HttpRequest) -> impl Responder {
    let missing = "MISSING".to_string();
    // Access query parameters
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let term = query.get("term").unwrap_or(&missing); 
    let reading = query.get("reading").unwrap_or(&missing); 

    println!("Term: {}", term.clone());
    println!("Reading: {}", reading.clone());

    "Hello world!" 
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default()) 
            .route("/", web::get().to(index)) 
    })
    .bind("localhost:8080")? 
    .run()
    .await 
}


