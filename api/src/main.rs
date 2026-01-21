use serde_json::json;
use simple_wiki_search::{load_app_state, search, AppState};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;
use tokio::fs;
use vercel_runtime::{run, service_fn, Error, Request};

static APP_STATE: OnceLock<AppState> = OnceLock::new();

const DOCS_URL: &str = "https://6yzjxguod4saepey.public.blob.vercel-storage.com/docs.bin";
const INDEX_URL: &str = "https://6yzjxguod4saepey.public.blob.vercel-storage.com/inverted_index.bin";
const TRIE_URL: &str = "https://6yzjxguod4saepey.public.blob.vercel-storage.com/trie.bin";

async fn download_file(url: &str, path: &Path) -> Result<(), Error> {
    if path.exists() {
        println!("File already exists: {:?}", path);
        return Ok(());
    }
    
    println!("Downloading {} to {:?}", url, path);
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    fs::write(path, bytes).await?;
    Ok(())
}

async fn setup_index() -> Result<(), Error> {
    let tmp_data = Path::new("/tmp/data");
    if !tmp_data.exists() {
        println!("Creating directory: {:?}", tmp_data);
        fs::create_dir_all(tmp_data).await?;
    }

    // Download binaries
    let downloads = vec![
        (DOCS_URL, "docs.bin"),
        (INDEX_URL, "inverted_index.bin"),
        (TRIE_URL, "trie.bin"),
    ];

    for (url, filename) in downloads {
        let dest = tmp_data.join(filename);
        download_file(url, &dest).await?;
    }

    // Copy config files from local ./data to /tmp/data
    // Vercel includes included files in the task root
    let local_data = Path::new("./data");
    let config_files = ["stopwords-en.txt", "lemmatization-en.txt", "whitelist.txt"];

    for filename in config_files {
        let src = local_data.join(filename);
        let dest = tmp_data.join(filename);
        
        if src.exists() && !dest.exists() {
            println!("Copying {:?} to {:?}", src, dest);
            fs::copy(&src, &dest).await?;
        } else if !src.exists() {
            println!("Warning: Local config file not found: {:?}", src);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("Starting search service...");
    
    // Prepare data in /tmp
    setup_index().await.map_err(|e| {
        println!("Failed to setup index: {}", e);
        e
    })?;

    // Initialize state
    println!("Loading app state...");
    // We pass "/tmp" because load_app_state appends "/data/..."
    // so it will look in "/tmp/data/..."
    let state = load_app_state(Some("/tmp"))
        .map_err(|e| Error::from(format!("Failed to load app state: {}", e)))?;
        
    APP_STATE.set(state).map_err(|_| Error::from("Failed to set APP_STATE"))?;
    println!("App state loaded successfully.");

    run(service_fn(handler)).await
}

use http::StatusCode;
use vercel_runtime::Response;

pub async fn handler(req: Request) -> Result<Response<String>, Error> {
    // For a public search API, we want to allow any origin to access it.
    // This avoids issues with Vercel previews, localhost, and production domains mismatching.
    let allowed_origin = "*".to_string();

    // Handle OPTIONS request for CORS preflight
    if req.method() == "OPTIONS" {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", &allowed_origin)
            .header("Access-Control-Allow-Methods", "GET, OPTIONS")
            .header("Access-Control-Allow-Headers", "*")
            .body(String::new())?);
    }

    let app_state = APP_STATE.get().ok_or_else(|| Error::from("App state not initialized"))?;

    let url = req.uri();
    let query_string = url.query().unwrap_or("");
    let query_params: Vec<(String, String)> = serde_urlencoded::from_str(query_string).unwrap_or_default();
    
    let query = query_params
        .iter()
        .find(|(key, _)| key == "q")
        .map(|(_, value)| value.trim())
        .unwrap_or("");

    if query.is_empty() {
        let json_response = json!({
            "error": "Missing query parameter 'q'",
            "results": [],
            "total_results": 0
        });
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", &allowed_origin)
            .header("Content-Type", "application/json")
            .body(json_response.to_string())?);
    }

    let start = Instant::now();
    let results = search(query, app_state);
    let duration = start.elapsed();

    let total_results = results.len();
    let top_results = results.into_iter().take(50).collect::<Vec<_>>();

    let json_response = json!({
        "query": query,
        "results": top_results,
        "total_results": total_results,
        "time_taken_ms": duration.as_secs_f64() * 1000.0
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", &allowed_origin)
        .header("Content-Type", "application/json")
        .body(json_response.to_string())?)
}
