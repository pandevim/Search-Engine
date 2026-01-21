use normalization::Normalizer;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use trie_rs::map::Trie;
use anyhow::{Context, Result};

#[derive(Debug)]
pub struct Posting {
    doc_id: u32,
    positions: Vec<u32>,
}

#[derive(Deserialize)]
pub struct IndexData {
    occurrence_lists: Vec<Vec<u8>>,
    avgdl: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DocumentMetadata {
    path: String,
    title: String,
    len: u32,
}

pub struct AppState {
    pub normalizer: Normalizer,
    pub trie: Trie<u8, u32>,
    pub occurrence_lists: Vec<Vec<u8>>,
    pub documents: Vec<DocumentMetadata>,
    pub avgdl: f64,
}

#[derive(Serialize)]
pub struct SearchResult {
    id: u32,
    path: String,
    title: String,
    score: f64,
}

fn decode_varint(data: &[u8], index: &mut usize) -> u32 {
    let mut result = 0;
    let mut shift = 0;
    loop {
        let byte = data[*index];
        *index += 1;
        result |= ((byte & 127) as u32) << shift;
        if byte & 128 == 0 {
            break;
        }
        shift += 7;
    }
    result
}

fn decode_posting_list(data: &[u8]) -> Vec<Posting> {
    let mut postings = Vec::new();
    let mut index = 0;
    let mut last_doc_id = 0;

    while index < data.len() {
        // Delta DocID
        let doc_delta = decode_varint(data, &mut index);
        let doc_id = last_doc_id + doc_delta;
        last_doc_id = doc_id;

        // Frequency
        let freq = decode_varint(data, &mut index);

        // Positions
        let mut positions = Vec::with_capacity(freq as usize);
        let mut last_pos = 0;
        for _ in 0..freq {
            let pos_delta = decode_varint(data, &mut index);
            let pos = last_pos + pos_delta;
            positions.push(pos);
            last_pos = pos;
        }

        postings.push(Posting { doc_id, positions });
    }
    postings
}


fn calculate_min_window(term_positions: &Vec<Vec<u32>>) -> u32 {
    if term_positions.is_empty() {
        return 0;
    }
    
    // We need to find the smallest range [min, max] that contains at least one position from each list.
    // This is equivalent to finding the smallest range in K sorted lists.
    
    // Current indices in each list
    let mut indices = vec![0; term_positions.len()];
    let mut min_window = u32::MAX;

    loop {
        let mut current_min = u32::MAX;
        let mut current_max = 0;
        let mut min_list_idx = 0;

        // Find the current range covered by the pointers
        for (i, list) in term_positions.iter().enumerate() {
            if indices[i] >= list.len() {
                return if min_window == u32::MAX { 0 } else { min_window };
            }
            let pos = list[indices[i]];
            if pos < current_min {
                current_min = pos;
                min_list_idx = i;
            }
            if pos > current_max {
                current_max = pos;
            }
        }

        let window_size = current_max - current_min + 1;
        if window_size < min_window {
            min_window = window_size;
        }

        // Advance the pointer of the list that had the minimum value
        indices[min_list_idx] += 1;
        
        // If any list is exhausted, we can't find any more valid windows containing all terms
        if indices[min_list_idx] >= term_positions[min_list_idx].len() {
            break;
        }
    }

    if min_window == u32::MAX { 0 } else { min_window }
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

pub fn search(query: &str, data: &AppState) -> Vec<SearchResult> {
    let tokens = data.normalizer.process(query);
    if tokens.is_empty() {
        return Vec::new();
    }

    // 1. Retrieve postings lists for all query terms
    // Decoded lists
    let mut decoded_postings: Vec<Vec<Posting>> = Vec::new();

    for token in &tokens {
        if let Some(list_index) = data.trie.exact_match(token) {
            if let Some(encoded_list) = data.occurrence_lists.get(*list_index as usize) {
                decoded_postings.push(decode_posting_list(encoded_list));
            } else {
                return Vec::new(); // Term found in trie but not in list? Should not happen.
            }
        } else {
            return Vec::new(); // Term not found, AND logic implies 0 results
        }
    }

    if decoded_postings.is_empty() {
        return Vec::new();
    }

    // 2. Find intersection of DocIDs
    // We start with the DocIDs of the first term
    let mut common_doc_ids: Vec<u32> = decoded_postings[0].iter().map(|p| p.doc_id).collect();

    for other_list in decoded_postings.iter().skip(1) {
        let other_ids: Vec<u32> = other_list.iter().map(|p| p.doc_id).collect();
        common_doc_ids = intersect_sorted(&common_doc_ids, &other_ids);
        if common_doc_ids.is_empty() {
            return Vec::new();
        }
    }

    // 3. Rank the documents
    let mut scored_results: Vec<SearchResult> = Vec::new();
    let n = data.documents.len() as f64;
    let avgdl = data.avgdl;

    // BM25 Constants
    let k1 = 1.2;
    let b = 0.75;

    for doc_id in common_doc_ids {
        let doc_idx = doc_id as usize;
        if let Some(doc_meta) = data.documents.get(doc_idx) {
            let doc_len = doc_meta.len as f64;
            
            let mut bm25_score = 0.0;
            let mut term_positions: Vec<Vec<u32>> = Vec::new();

            for (i, _) in tokens.iter().enumerate() {
                // Find the posting for this term and doc_id
                // Since we know doc_id is in the list, we can find it.
                // Optimization: We could have collected these during intersection, but binary search is fast enough.
                let posting_list = &decoded_postings[i];
                if let Ok(idx) = posting_list.binary_search_by_key(&doc_id, |p| p.doc_id) {
                    let posting = &posting_list[idx];
                    let tf = posting.positions.len() as f64;
                    
                    // IDF
                    // n(qi) is the number of docs containing the term
                    let n_qi = posting_list.len() as f64;
                    let idf = ((n - n_qi + 0.5) / (n_qi + 0.5) + 1.0).ln();

                    // BM25 Term Score
                    let numerator = tf * (k1 + 1.0);
                    let denominator = tf + k1 * (1.0 - b + b * (doc_len / avgdl));
                    bm25_score += idf * (numerator / denominator);

                    term_positions.push(posting.positions.clone());
                }
            }

            // Window Score
            let min_window = calculate_min_window(&term_positions);
            let window_score = if min_window > 0 {
                tokens.len() as f64 / min_window as f64
            } else {
                0.0
            };

            // Final Score
            // Weights can be adjusted.
            let alpha = 1.0;
            let beta = 1.0;
            let final_score = alpha * window_score + beta * bm25_score;

            scored_results.push(SearchResult {
                id: doc_id,
                path: doc_meta.path.clone(),
                title: doc_meta.title.clone(),
                score: final_score,
            });
        }
    }

    // Sort by score descending
    scored_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    scored_results
}

pub fn load_app_state() -> Result<AppState> {
    // 1. Initialize Normalizer
    let mut normalizer = Normalizer::new();
    normalizer
        .load_stopwords("data/stopwords-en.txt")
        .context("Failed to load stopwords")?;
    normalizer
        .load_lemmatization_file("data/lemmatization-en.txt")
        .context("Failed to load lemmatization file")?;
    normalizer
        .load_whitelist("data/whitelist.txt")
        .context("Failed to load whitelist")?;

    // 2. Load Data
    println!("Loading index...");

    let trie_file = File::open("data/trie.bin").context("Failed to open trie.bin")?;
    let reader = BufReader::new(trie_file);
    let trie: Trie<u8, u32> = bincode::deserialize_from(reader).context("Failed to deserialize trie")?;

    let index_file = File::open("data/inverted_index.bin").context("Failed to open inverted_index.bin")?;
    let reader = BufReader::new(index_file);
    let index_data: IndexData = bincode::deserialize_from(reader).context("Failed to deserialize index")?;

    let docs_file = File::open("data/docs.bin").context("Failed to open docs.bin")?;
    let reader = BufReader::new(docs_file);
    let documents: Vec<DocumentMetadata> = bincode::deserialize_from(reader).context("Failed to deserialize docs")?;

    Ok(AppState {
        normalizer,
        trie,
        occurrence_lists: index_data.occurrence_lists,
        documents,
        avgdl: index_data.avgdl,
    })
}
