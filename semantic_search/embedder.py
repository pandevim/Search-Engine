import os
import json
import numpy as np
from sentence_transformers import SentenceTransformer
from usearch.index import Index
from bs4 import BeautifulSoup
from tqdm import tqdm

# --- Configuration ---
DATA_DIR = "data"
CRAWLED_LIST = os.path.join(DATA_DIR, "crawled.lst")
WIKI_ROOT = "wikipedia-simple-html-dump"
EMBEDDING_MODEL = "all-MiniLM-L6-v2"
USEARCH_INDEX_PATH = os.path.join(DATA_DIR, "semantic_index.usearch")

def load_documents(crawled_list_path):
    """
    Reads the list of crawled files and generator that yields (doc_id, filepath, text_content).
    """
    print(f"Loading document list from {crawled_list_path}...")
    with open(crawled_list_path, "r") as f:
        doc_paths = [line.strip() for line in f if line.strip()]
    
    print(f"Found {len(doc_paths)} documents.")
    return doc_paths

def extract_content(filepath):
    """
    Extracts title and first paragraph (abstract) from the HTML file.
    """
    full_path = os.path.join(WIKI_ROOT, filepath)
    try:
        if not os.path.exists(full_path):
            return None
            
        with open(full_path, "r", encoding="utf-8", errors="ignore") as f:
            soup = BeautifulSoup(f, "lxml")
            
        # Title
        title = soup.title.string if soup.title else ""
        if title:
            title = title.replace(" - Wikipedia", "")
            
        # Abstract (first paragraph(s) of the content)
        # Simple Wikipedia structure usually has content in <div id="bodyContent"> or just <p> tags
        # We try to grab the first non-empty paragraph.
        text_parts = []
        for p in soup.select("p"):
            text = p.get_text().strip()
            if text:
                text_parts.append(text)
                if len(text_parts) >= 2: # Grab first 2 paragraphs for better context
                    break
        
        abstract = " ".join(text_parts)
        
        # Combine Title + Abstract for embedding
        combined_text = f"{title}. {abstract}"
        return combined_text
        
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return None

def main():
    print("--- Starting Semantic Embedder ---")
    
    # 1. Load Documents
    doc_paths = load_documents(CRAWLED_LIST)
    
    # 2. Extract Text
    print("Extracting content from documents...")
    documents = []
    doc_ids = [] # Keep track of original line numbers (0-indexed) as stable IDs
    
    # Limit for testing/debugging if needed, remove slice for full run
    # doc_paths = doc_paths[:1000] 
    
    for i, path in enumerate(tqdm(doc_paths)):
        text = extract_content(path)
        if text:
            documents.append(text)
            doc_ids.append(i) # Store the original index from crawled.lst
    
    print(f"Successfully extracted text from {len(documents)} documents.")

    # 3. Generate Embeddings
    print(f"Loading model {EMBEDDING_MODEL}...")
    model = SentenceTransformer(EMBEDDING_MODEL)
    
    print("Generating embeddings (this may take a while)...")
    embeddings = model.encode(documents, show_progress_bar=True, convert_to_numpy=True, normalize_embeddings=True)
    
    # 4. Create and Save USearch Index
    print("Building USearch index...")
    # metrics: ip (inner product), l2 (euclidean), cos (cosine).
    # Since vectors are normalized, ip is identical to cosine and usually faster.
    d = embeddings.shape[1]
    us_index = Index(ndim=d, metric="ip")
    
    # USearch expects keys to be integers. 
    # We use the doc_ids (original line numbers) we collected earlier.
    keys = np.array(doc_ids, dtype=np.longlong)
    us_index.add(keys, embeddings)
    
    print(f"Saving USearch index to {USEARCH_INDEX_PATH}...")
    us_index.save(USEARCH_INDEX_PATH)
    
    print("--- Processing Complete ---")
    
    # Simple Verification Test
    test_query = "computer science"
    print(f"\nRunning verification query: '{test_query}'")
    query_vector = model.encode([test_query], normalize_embeddings=True)
    
    k = 5
    matches = us_index.search(query_vector, k)
    
    print(f"Top {k} results:")
    
    # Safe way for single query (flattening if needed):
    us_keys = matches.keys.flatten()
    us_dists = matches.distances.flatten()
    
    for i in range(min(k, len(us_keys))):
        idx = us_keys[i]
        score = us_dists[i]
        
        # Look up directly in the original loaded doc_paths list
        if 0 <= idx < len(doc_paths):
            doc_path = doc_paths[idx]
        else:
            doc_path = "Unknown (Index out of bounds)"
            
        print(f"{i+1}. {doc_path} (Score: {score:.4f})")

if __name__ == "__main__":
    main()
