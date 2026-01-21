use serde_json::{json, Value};
use simple_wiki_search::{load_app_state, search, AppState};
use std::sync::OnceLock;
use vercel_runtime::{run, service_fn, Error, Request};

static APP_STATE: OnceLock<AppState> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}

pub async fn handler(req: Request) -> Result<Value, Error> {
    // initialize global state if not already initialized
    let app_state = APP_STATE.get_or_init(|| {
        println!("Cold start: Loading index...");
        // On Vercel, files are included in the root lambda directory.
        // We pass "." to look in current directory.
        load_app_state(Some(".")).expect("Failed to load app state")
    });

    let url = req.uri();
    let query_string = url.query().unwrap_or("");
    let query_params: Vec<(String, String)> = serde_urlencoded::from_str(query_string).unwrap_or_default();
    
    let query = query_params
        .iter()
        .find(|(key, _)| key == "q")
        .map(|(_, value)| value.trim())
        .unwrap_or("");

    if query.is_empty() {
        return Ok(json!({
            "error": "Missing query parameter 'q'",
            "results": [],
            "total_results": 0
        }));
    }

    let results = search(query, app_state);
    let total_results = results.len();
    let top_results = results.into_iter().take(50).collect::<Vec<_>>();

    Ok(json!({
        "query": query,
        "results": top_results,
        "total_results": total_results
    }))
}
