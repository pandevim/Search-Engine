use anyhow::{Context, Result};
use linguist::Linguist;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use trie_rs::map::{Trie, TrieBuilder};

#[derive(Serialize, Deserialize, Debug)]
struct Posting {
    doc_id: u32,
    positions: Vec<u32>,
}

#[derive(Serialize, Deserialize)]
struct IndexData {
    // Stores the occurrence lists for each term.
    occurrence_lists: Vec<Vec<Posting>>,
    avgdl: f64,
}

#[derive(Serialize, Deserialize)]
struct DocumentMetadata {
    path: String,
    title: String,
    len: u32,
}

fn main() -> Result<()> {
    println!("Initializing Indexer...");

    // 1. Initialize Linguist
    let mut linguist = Linguist::new();
    linguist
        .load_stopwords("data/stopwords-en.txt")
        .context("Failed to load stopwords")?;
    linguist
        .load_lemmatization_file("data/lemmatization-en.txt")
        .context("Failed to load lemmatization file")?;
    linguist
        .load_whitelist("data/whitelist.txt")
        .context("Failed to load whitelist")?;

    // 2. Load crawled file list
    let crawled_path = "wikipedia-simple-html-dump/html.lst";
    // let crawled_path = "data/crawled.lst";

    let file = File::open(crawled_path).context("Failed to open crawled.lst")?;
    let reader = BufReader::new(file);
    let doc_paths: Vec<String> = std::io::BufRead::lines(reader)
        .collect::<Result<_, _>>()?;

    println!("Found {} documents to index.", doc_paths.len());

    // 3. Build Inverted Index in Memory
    // Term -> List of Postings (DocID, Positions)
    // We use a temporary map: Term -> DocID -> Positions
    let mut temp_index: HashMap<String, HashMap<u32, Vec<u32>>> = HashMap::new();
    let mut documents: Vec<DocumentMetadata> = Vec::with_capacity(doc_paths.len());
    let mut total_doc_len: u64 = 0;

    let selector = Selector::parse("body").unwrap();
    let title_selector = Selector::parse("title").unwrap();

    for (doc_id, rel_path) in doc_paths.iter().enumerate() {
        let doc_id = doc_id as u32;
        let full_path = Path::new("wikipedia-simple-html-dump").join(rel_path);

        if doc_id % 100 == 0 {
            println!("Indexing document {}/{}", doc_id, doc_paths.len());
        }

        let content = fs::read_to_string(&full_path);
        if let Ok(html_content) = content {
            let document = Html::parse_document(&html_content);
            
            let title = document
                .select(&title_selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(" "))
                .unwrap_or_else(|| "Untitled".to_string());

            // Extract text from body
            let text_content = document
                .select(&selector)
                .map(|element| element.text().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join(" ");

            let tokens = linguist.process(&text_content);
            let doc_len = tokens.len() as u32;
            total_doc_len += doc_len as u64;

            documents.push(DocumentMetadata {
                path: rel_path.clone(),
                title,
                len: doc_len,
            });

            for (pos, token) in tokens.into_iter().enumerate() {
                temp_index
                    .entry(token)
                    .or_insert_with(HashMap::new)
                    .entry(doc_id)
                    .or_insert_with(Vec::new)
                    .push(pos as u32);
            }
        } else {
            eprintln!("Warning: Could not read file {:?}", full_path);
            documents.push(DocumentMetadata {
                path: rel_path.clone(),
                title: "Error reading file".to_string(),
                len: 0,
            });
        }
    }

    println!("Indexing complete. Processing results...");

    let avgdl = if !documents.is_empty() {
        total_doc_len as f64 / documents.len() as f64
    } else {
        0.0
    };

    // 4. Sort occurrence lists and prepare for storage
    let mut occurrence_lists: Vec<Vec<Posting>> = Vec::new();
    let mut trie_builder = TrieBuilder::new();

    // We iterate over the hashmap, move lists to the vector, and add to Trie
    for (term, doc_map) in temp_index {
        let mut postings: Vec<Posting> = doc_map
            .into_iter()
            .map(|(doc_id, positions)| Posting { doc_id, positions })
            .collect();
        
        // Sort by DocID for efficient intersection
        postings.sort_unstable_by_key(|p| p.doc_id);

        let list_index = occurrence_lists.len() as u32;
        occurrence_lists.push(postings);
        
        trie_builder.push(term, list_index);
    }

    println!("Building Trie...");
    let trie: Trie<u8, u32> = trie_builder.build();

    // 5. Save to disk
    println!("Saving index to disk...");
    
    // Save Document Metadata
    let docs_file = File::create("data/docs.bin")?;
    let mut writer = BufWriter::new(docs_file);
    bincode::serialize_into(&mut writer, &documents)?;
    
    // Save Occurrence Lists
    let index_data = IndexData { occurrence_lists, avgdl };
    let index_file = File::create("data/inverted_index.bin")?;
    let mut writer = BufWriter::new(index_file);
    bincode::serialize_into(&mut writer, &index_data)?;
    
    // Save Trie
    // trie-rs supports serde serialization
    let trie_file = File::create("data/trie.bin")?;
    let mut writer = BufWriter::new(trie_file);
    bincode::serialize_into(&mut writer, &trie)?;

    println!("Done! Index saved to data/inverted_index.bin and data/trie.bin");

    Ok(())
}
