use anyhow::{Context, Result};
use linguist::Linguist;
use serde::Deserialize;
use std::fs::File;
use std::io::{self, BufReader, Write};
use trie_rs::map::Trie;

#[derive(Deserialize)]
struct IndexData {
    occurrence_lists: Vec<Vec<u32>>,
}

fn main() -> Result<()> {
    println!("Initializing Search Engine...");

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
    
    // Load Trie
    let trie_file = File::open("data/trie.bin").context("Failed to open trie.bin")?;
    let reader = BufReader::new(trie_file);
    let trie: Trie<u8, u32> = bincode::deserialize_from(reader).context("Failed to deserialize trie")?;

    // Load Inverted Index
    let index_file = File::open("data/inverted_index.bin").context("Failed to open inverted_index.bin")?;
    let reader = BufReader::new(index_file);
    let index_data: IndexData = bincode::deserialize_from(reader).context("Failed to deserialize index")?;

    // Load Document Paths
    let crawled_path = "data/crawled.lst";
    let file = File::open(crawled_path).context("Failed to open crawled.lst")?;
    let reader = BufReader::new(file);
    let doc_paths: Vec<String> = std::io::BufRead::lines(reader)
        .collect::<Result<_, _>>()?;

    println!("Engine ready! (Loaded {} documents)", doc_paths.len());
    println!("Enter your search query (or 'exit' to quit):");

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let query = input.trim();

        if query.eq_ignore_ascii_case("exit") {
            break;
        }

        if query.is_empty() {
            continue;
        }

        let start = std::time::Instant::now();
        let results = search(query, &linguist, &trie, &index_data.occurrence_lists);
        let duration = start.elapsed();

        println!("Found {} results in {:.2?}", results.len(), duration);
        for (i, doc_id) in results.iter().take(10).enumerate() {
            if let Some(path) = doc_paths.get(*doc_id as usize) {
                println!("{}. {}", i + 1, path);
            }
        }
        if results.len() > 10 {
            println!("... and {} more", results.len() - 10);
        }
        println!();
    }

    Ok(())
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
                // Should not happen if index is consistent
                return Vec::new();
            }
        } else {
            // If any term is not found, AND query returns empty
            return Vec::new();
        }
    }

    if result_sets.is_empty() {
        return Vec::new();
    }

    // Intersect all lists
    // Start with the first list
    let mut intersection = result_sets[0].clone();

    for other_list in result_sets.iter().skip(1) {
        intersection = intersect_sorted(&intersection, other_list);
        if intersection.is_empty() {
            break;
        }
    }

    intersection
}

// Efficient intersection of two sorted vectors
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
