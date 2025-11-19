use anyhow::{Context, Result};
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::io::Write;
use std::path::{Path, PathBuf};
use url::Url;

fn main() -> Result<()> {
    // Read seed path from data/seed.lst
    let seed_content = fs::read_to_string("data/seed.lst")
        .or_else(|_| fs::read_to_string("../data/seed.lst"))
        .context("Failed to read data/seed.lst")?;
    
    let seed_arg = seed_content.trim().to_string();
    let mut seed_path = PathBuf::from(&seed_arg);

    if !seed_path.exists() {
        // Try prepending wikipedia-simple-html-dump/ if it doesn't exist
        let alt_path = Path::new("wikipedia-simple-html-dump").join(&seed_arg);
        if alt_path.exists() {
            seed_path = alt_path;
        } else {
             // Try relative to parent if we are in crawler dir
             let alt_seed = Path::new("../wikipedia-simple-html-dump").join(&seed_arg);
             if alt_seed.exists() {
                  seed_path = alt_seed;
             } else {
                 eprintln!("Error: Seed file {:?} does not exist.", seed_path);
                 return Ok(());
             }
        }
    }

    println!("Using seed file: {:?}", seed_path);
    crawl(seed_path, &env::current_dir()?)
}

fn crawl(seed_path: PathBuf, root_dir: &Path) -> Result<()> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut found_files = Vec::new();

    let abs_seed = fs::canonicalize(&seed_path).context("Failed to canonicalize seed path")?;
    
    queue.push_back(abs_seed.clone());
    visited.insert(abs_seed.clone());
    found_files.push(abs_seed.clone());

    let selector = Selector::parse("a").unwrap();

    // Setup cancellation via Ctrl+C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\nCtrl+C received. Stopping crawler gracefully...");
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    println!("Crawler started. Press Ctrl+C at any time to stop and save progress.");

    // Spawn a logger thread to print asynchronously
    let (tx, rx) = mpsc::channel::<String>();
    let logger_handle = thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            println!("{}", msg);
        }
    });

    while let Some(curr_path) = queue.pop_front() {
        // Check if we should stop
        if !running.load(Ordering::SeqCst) {
            break;
        }

        // Send log message asynchronously
        let _ = tx.send(format!("Crawling: {:?}", curr_path));

        // Removed hard limit of 10000 files to allow user to stop manually
        // if visited.len() >= 10000 { break; }

        let content = fs::read_to_string(&curr_path);
        if content.is_err() {
            eprintln!("Failed to read: {:?}", curr_path);
            continue;
        }
        let content = content.unwrap();

        let document = Html::parse_document(&content);
        
        // Create a base URL for resolving relative links
        let base_url = Url::from_file_path(&curr_path).map_err(|_| anyhow::anyhow!("Invalid file path"))?;

        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                // Ignore anchors
                if href.starts_with('#') {
                    continue;
                }

                // Only accept relative paths (no scheme, no leading slash usually implies relative in this context)
                // Url::parse will fail for relative paths, which is what we want.
                // If it succeeds, it means it's an absolute URL (like http://... or file://...)
                if Url::parse(href).is_ok() {
                    continue;
                }

                // Resolve URL
                let resolved_url = match base_url.join(href) {
                    Ok(u) => u,
                    Err(_) => continue,
                };

                // Convert back to path
                let file_path = match resolved_url.to_file_path() {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                // Check if file exists and is a file (not dir)
                if file_path.exists() && file_path.is_file() {
                    let canonical_path = match fs::canonicalize(&file_path) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    if !visited.contains(&canonical_path) {
                        visited.insert(canonical_path.clone());
                        queue.push_back(canonical_path.clone());
                        found_files.push(canonical_path);
                    }
                }
            }
        }
    }

    // Drop the sender to close the channel and wait for logger to finish
    drop(tx);
    let _ = logger_handle.join();

    // Write to lst file
    let output_path = Path::new("data/crawled.lst");
    let mut file = fs::File::create(output_path)?;
    for path in found_files {
        // Try to make path relative to root_dir
        let relative_path = if let Ok(rel) = path.strip_prefix(root_dir) {
            rel
        } else {
            path.as_path()
        };

        // Strip "wikipedia-simple-html-dump/" prefix if present to make it independent of directory structure
        let display_path = if let Ok(stripped) = relative_path.strip_prefix("wikipedia-simple-html-dump") {
            stripped.display().to_string()
        } else {
            relative_path.display().to_string()
        };

        writeln!(file, "{}", display_path)?;
    }
    
    let abs_output_path = fs::canonicalize(output_path)?;
    println!("Crawling complete. Found {} files. Saved to {:?}", visited.len(), abs_output_path);

    Ok(())
}
