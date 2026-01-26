
import os
import json
import numpy as np
import faiss
from sentence_transformers import SentenceTransformer
from bs4 import BeautifulSoup
from tqdm import tqdm

# --- Configuration ---
DATA_DIR = "data"
CRAWLED_LIST = os.path.join(DATA_DIR, "crawled.lst")
WIKI_ROOT = "wikipedia-simple-html-dump"
EMBEDDING_MODEL = "all-MiniLM-L6-v2"
FAISS_INDEX_PATH = os.path.join(DATA_DIR, "semantic_index.faiss")
RAW_VECTORS_PATH = os.path.join(DATA_DIR, "embeddings.npy")
DOC_MAP_PATH = os.path.join(DATA_DIR, "doc_map.json")

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
    valid_doc_paths = [] # Keep track of docs we actually successfully processed
    
    # Limit for testing/debugging if needed, remove slice for full run
    # doc_paths = doc_paths[:1000] 
    
    for i, path in enumerate(tqdm(doc_paths)):
        text = extract_content(path)
        if text:
            documents.append(text)
            valid_doc_paths.append(path)
    
    print(f"Successfully extracted text from {len(documents)} documents.")

    # 3. Generate Embeddings
    print(f"Loading model {EMBEDDING_MODEL}...")
    model = SentenceTransformer(EMBEDDING_MODEL)
    
    print("Generating embeddings (this may take a while)...")
    embeddings = model.encode(documents, show_progress_bar=True, convert_to_numpy=True)
    
    # Normalize embeddings for cosine similarity (FAISS defaults to L2, but normalized L2 == Cosine)
    print("Normalizing embeddings...")
    faiss.normalize_L2(embeddings)
    
    # 4. Save Raw Vectors (for Rust integration option)
    print(f"Saving raw vectors to {RAW_VECTORS_PATH}...")
    np.save(RAW_VECTORS_PATH, embeddings)
    
    # 5. Create and Save FAISS Index
    print("Building FAISS index...")
    d = embeddings.shape[1] # Dimension (384 for MiniLM)
    index = faiss.IndexFlatIP(d) # Inner Product (Cosine Similarity since normalized)
    index.add(embeddings)
    
    print(f"Saving FAISS index to {FAISS_INDEX_PATH}...")
    faiss.write_index(index, FAISS_INDEX_PATH)
    
    # 6. Save Document Map
    # Map index ID -> File Path for retrieval
    print(f"Saving document map to {DOC_MAP_PATH}...")
    doc_map = {i: path for i, path in enumerate(valid_doc_paths)}
    with open(DOC_MAP_PATH, "w") as f:
        json.dump(doc_map, f, indent=2)
        
    print("--- Processing Complete ---")
    
    # Simple Verification Test
    test_query = "computer science"
    print(f"\nRunning verification query: '{test_query}'")
    query_vector = model.encode([test_query])
    faiss.normalize_L2(query_vector)
    
    k = 5
    D, I = index.search(query_vector, k)
    
    print(f"Top {k} results:")
    for i in range(k):
        idx = I[0][i]
        score = D[0][i]
        doc_path = doc_map[idx]
        print(f"{i+1}. {doc_path} (Score: {score:.4f})")

if __name__ == "__main__":
    main()
