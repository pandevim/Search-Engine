use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use simple_wiki_search::{load_app_state, search, AppState, SearchResult};

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
struct SearchResponse {
    query: String,
    results: Vec<SearchResult>,
    total_results: usize,
    time_taken_ms: f64,
}

#[get("/search")]
async fn search_handler(
    query: web::Query<SearchQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let start = Instant::now();
    let q = query.q.trim();

    if q.is_empty() {
        return HttpResponse::BadRequest().json(SearchResponse {
            query: q.to_string(),
            results: vec![],
            total_results: 0,
            time_taken_ms: 0.0,
        });
    }

    let all_results = search(q, &data);
    let total_results = all_results.len();
    let duration = start.elapsed();

    let results: Vec<SearchResult> = all_results.into_iter().take(50).collect();

    HttpResponse::Ok().json(SearchResponse {
        query: q.to_string(),
        results,
        total_results,
        time_taken_ms: duration.as_secs_f64() * 1000.0,
    })
}

#[actix_web::main]
async fn main() -> Result<()> {
    println!("Initializing Server...");

    let app_state = load_app_state()?;

    println!("Index loaded successfully. {} documents available.", app_state.documents.len());

    let app_data = web::Data::new(app_state);

    println!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive()) // Allow all CORS for now
            .app_data(app_data.clone())
            .service(search_handler)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
