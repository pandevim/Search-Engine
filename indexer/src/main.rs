use anyhow::{Context, Result};
use linguist::Linguist;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use trie_rs::map::{Trie, TrieBuilder};

#[derive(Serialize, Deserialize)]
struct IndexData {
    // Stores the occurrence lists (list of DocIDs) for each term.
    // The Trie will map a term to an index in this vector.
    occurrence_lists: Vec<Vec<u32>>,
}

#[derive(Serialize, Deserialize)]
struct DocumentMetadata {
    path: String,
    title: String,
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

    // 2. Load crawled file list
    let crawled_path = "data/crawled.lst";
    let file = File::open(crawled_path).context("Failed to open crawled.lst")?;
    let reader = BufReader::new(file);
    let doc_paths: Vec<String> = std::io::BufRead::lines(reader)
        .collect::<Result<_, _>>()?;

    println!("Found {} documents to index.", doc_paths.len());

    // 3. Build Inverted Index in Memory
    // Term -> List of DocIDs
    let mut temp_index: HashMap<String, Vec<u32>> = HashMap::new();
    let mut documents: Vec<DocumentMetadata> = Vec::with_capacity(doc_paths.len());

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

            documents.push(DocumentMetadata {
                path: rel_path.clone(),
                title,
            });

            // Extract text from body
            // We join all text nodes with spaces
            let text_content = document
                .select(&selector)
                .map(|element| element.text().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join(" ");

            let tokens = linguist.process(&text_content);

            for token in tokens {
                temp_index
                    .entry(token)
                    .or_insert_with(Vec::new)
                    .push(doc_id);
            }
        } else {
            eprintln!("Warning: Could not read file {:?}", full_path);
            documents.push(DocumentMetadata {
                path: rel_path.clone(),
                title: "Error reading file".to_string(),
            });
        }
    }

    println!("Indexing complete. Processing results...");

    // 4. Sort occurrence lists and prepare for storage
    // The requirement is that occurrence lists are sorted by address (DocID).
    // Since we processed docs in order (0, 1, 2...), they are naturally sorted if we only pushed.
    // However, a term might appear multiple times in a doc. We should dedup and ensure sort.
    
    let mut occurrence_lists: Vec<Vec<u32>> = Vec::new();
    let mut trie_builder = TrieBuilder::new();

    // We iterate over the hashmap, move lists to the vector, and add to Trie
    for (term, mut doc_ids) in temp_index {
        // Dedup: A term appearing multiple times in one doc should only be listed once per doc for a simple boolean/intersection search.
        // If we wanted frequency, we'd store (DocId, Freq). For now, let's assume simple occurrence.
        doc_ids.sort_unstable();
        doc_ids.dedup();

        let list_index = occurrence_lists.len() as u32; // Index in the occurrence_lists vector
        occurrence_lists.push(doc_ids);
        
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
    let index_data = IndexData { occurrence_lists };
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
