use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use anyhow::{Context, Result};
use linguist::Linguist;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;
use trie_rs::map::Trie;

#[derive(Deserialize)]
struct IndexData {
    occurrence_lists: Vec<Vec<u32>>,
}

struct AppState {
    linguist: Linguist,
    trie: Trie<u8, u32>,
    occurrence_lists: Vec<Vec<u32>>,
    doc_paths: Vec<String>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
struct SearchResult {
    id: u32,
    path: String,
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

    let result_ids = search(q, &data.linguist, &data.trie, &data.occurrence_lists);
    let duration = start.elapsed();

    let results: Vec<SearchResult> = result_ids
        .iter()
        .take(50) // Limit to top 50 for API
        .filter_map(|&id| {
            data.doc_paths.get(id as usize).map(|path| SearchResult {
                id,
                path: path.clone(),
            })
        })
        .collect();

    HttpResponse::Ok().json(SearchResponse {
        query: q.to_string(),
        results,
        total_results: result_ids.len(),
        time_taken_ms: duration.as_secs_f64() * 1000.0,
    })
}

fn search(
    query: &str,
    linguist: &Linguist,
    trie: &Trie<u8, u32>,
    occurrence_lists: &Vec<Vec<u32>>,
) -> Vec<u32> {
    let tokens = linguist.process(query);
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut result_sets: Vec<&Vec<u32>> = Vec::new();

    for token in tokens {
        if let Some(list_index) = trie.exact_match(token) {
            if let Some(list) = occurrence_lists.get(*list_index as usize) {
                result_sets.push(list);
            } else {
                return Vec::new();
            }
        } else {
            return Vec::new();
        }
    }

    if result_sets.is_empty() {
        return Vec::new();
    }

    let mut intersection = result_sets[0].clone();

    for other_list in result_sets.iter().skip(1) {
        intersection = intersect_sorted(&intersection, other_list);
        if intersection.is_empty() {
            break;
        }
    }

    intersection
}

fn intersect_sorted(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if a[i] < b[j] {
            i += 1;
        } else if a[i] > b[j] {
            j += 1;
        } else {
            result.push(a[i]);
            i += 1;
            j += 1;
        }
    }

    result
}

#[actix_web::main]
async fn main() -> Result<()> {
    println!("Initializing Server...");

    // 1. Initialize Linguist
    let mut linguist = Linguist::new();
    linguist
        .load_stopwords("data/stopwords-en.txt")
        .context("Failed to load stopwords")?;
    linguist
        .load_lemmatization_file("data/lemmatization-en.txt")
        .context("Failed to load lemmatization file")?;

    // 2. Load Data
    println!("Loading index...");
    
    let trie_file = File::open("data/trie.bin").context("Failed to open trie.bin")?;
    let reader = BufReader::new(trie_file);
    let trie: Trie<u8, u32> = bincode::deserialize_from(reader).context("Failed to deserialize trie")?;

    let index_file = File::open("data/inverted_index.bin").context("Failed to open inverted_index.bin")?;
    let reader = BufReader::new(index_file);
    let index_data: IndexData = bincode::deserialize_from(reader).context("Failed to deserialize index")?;

    let crawled_path = "data/crawled.lst";
    let file = File::open(crawled_path).context("Failed to open crawled.lst")?;
    let reader = BufReader::new(file);
    let doc_paths: Vec<String> = std::io::BufRead::lines(reader)
        .collect::<Result<_, _>>()?;

    println!("Index loaded. {} documents available.", doc_paths.len());

    let app_state = web::Data::new(AppState {
        linguist,
        trie,
        occurrence_lists: index_data.occurrence_lists,
        doc_paths,
    });

    println!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive()) // Allow all CORS for now
            .app_data(app_state.clone())
            .service(search_handler)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
