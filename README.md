# Simple Wikipedia Search Engine

## Project Overview

This project is a search engine built in Rust, designed to index and search a local copy of the Simple English Wikipedia. It consists of multiple components, starting with a high-performance web crawler.

## Project Structure

```text
.
├── crawler/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── client/
│   ├── package.json
│   └── src/
│       └── routes/
│           └── +page.svelte
├── data/
│   ├── crawled.lst
│   ├── inverted_index.bin
│   ├── lemmatization-en.txt
│   ├── seed.lst
│   ├── stopwords-en.txt
│   └── trie.bin
├── indexer/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── linguist/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── stemmer.rs
├── search/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── server/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── wikipedia-simple-html-dump/
│   └── ... (HTML files)
└── README.md
```

## Components

### 1. Crawler

The crawler is responsible for traversing the local Wikipedia HTML files to discover and list all available articles.

#### How it Works

1.  **Initialization**: Reads a seed file (`data/seed.lst`) to find the starting point (e.g., `simple/index.html`).
2.  **Traversal**: It uses a Breadth-First Search (BFS) algorithm to visit pages.
3.  **Parsing**: For each visited page, it parses the HTML to find `<a>` tags (links).
4.  **Filtering**: It strictly filters for **relative local file paths**, ignoring external links (http/https) and anchors (#).
5.  **Output**: It generates a list of all discovered unique file paths.

#### Output

The crawler produces a file named `crawled.lst` in the `data/` directory. This file contains the relative paths of all visited documents, stripped of the `wikipedia-simple-html-dump/` prefix to ensure portability.

Example output in `data/crawled.lst`:

```text
simple/index.html
simple/articles/a/r/t/Article_Name.html
...
```

### 2. Linguist

The `linguist` component is a text processing library responsible for preparing raw text for indexing.

#### Pipeline

1.  **Casefolding**: Converts all text to lowercase to ensure case-insensitive matching.
2.  **Tokenization**: Splits text into individual tokens (words) based on punctuation and whitespace.
3.  **Stopword Removal**: Filters out common English filler words (e.g., "the", "is", "at") to reduce index size and improve relevance. Uses `data/stopwords-en.txt` ([Source](https://github.com/stopwords-iso/stopwords-en)).
4.  **Stemming/Lemmatization**: Reduces words to their root form (e.g., "running" -> "run") using a dictionary-based approach sourced from `data/lemmatization-en.txt` ([Source](https://github.com/michmech/lemmatization-lists)).

### 3. Indexer

The indexer builds an efficient inverted index from the processed documents.

#### Architecture

Following the standard inverted index design:

1.  **Inverted Index (Occurrence Lists)**: A binary file (`data/inverted_index.bin`) storing the list of document IDs for each term. These lists are sorted by Document ID to allow for efficient intersection algorithms (like sorted sequence merging).
2.  **Compressed Trie**: An in-memory compressed trie (implemented using `trie-rs`) that maps every term in the vocabulary to the index of its occurrence list in the binary file. The trie is serialized to `data/trie.bin`.

#### Process

1.  **Initialization**: Loads the `crawled.lst` to map Document IDs (line numbers) to file paths.
2.  **Processing**: Iterates through every document, parses the HTML to extract text, and uses the `Linguist` library to tokenize and stem the content.
3.  **Index Construction**: Builds a temporary in-memory map of `Term -> [DocID]`.
4.  **Optimization**: Sorts and deduplicates document IDs within each list.
5.  **Serialization**: Saves the occurrence lists and the Trie to disk using `bincode` for compact binary storage.

### 4. Search

The search component provides the user interface for querying the index.

#### Features

- **Interactive CLI**: A command-line loop that accepts user queries.
- **Boolean AND Search**: Supports multi-word queries by finding the intersection of documents containing all terms (e.g., "computer science" finds docs with both "computer" AND "science").
- **Fast Retrieval**: Uses the compressed Trie for O(L) lookups (where L is word length) and efficient sorted list intersection.
- **Result Display**: Shows the top 10 matching document paths and the total count of results.

### 5. Server (REST API)

The server component exposes the search functionality via a RESTful API using `actix-web`.

#### Endpoints

- `GET /search?q=<query>`: Performs a search and returns JSON results.

#### Response Format

```json
{
  "query": "wikipedia",
  "results": [
    { "id": 1, "path": "simple/index.html" },
    ...
  ],
  "total_results": 9145,
  "time_taken_ms": 2.34
}
```

### 6. Client (Web UI)

The client is a modern web interface built with **SvelteKit** that consumes the REST API.

#### Features

- **Clean UI**: Simple search bar and results display.
- **Real-time Feedback**: Shows search time and total results found.
- **Direct Links**: Links directly to the local file paths (requires browser permission or local setup).

## Usage

### Prerequisites

- Rust toolchain (Cargo, rustc)
- Local Wikipedia dump extracted to `wikipedia-simple-html-dump/` directory
- Lemmatization dictionary at `data/lemmatization-en.txt` ([Source](https://github.com/michmech/lemmatization-lists))
- Stopwords list at `data/stopwords-en.txt` ([Source](https://github.com/stopwords-iso/stopwords-en))

### Configuration

The crawler uses a `seed.lst` file in the `data/` directory to determine the entry point.

```text
simple/index.html
```

### 1. Running the Crawler

To execute the crawler from the project root. This will traverse the `wikipedia-simple-html-dump` directory and generate `data/crawled.lst`.

```bash
cargo run --manifest-path crawler/Cargo.toml
```

### 2. Running the Indexer

To build the index from the crawled data. This will process the files listed in `crawled.lst` and generate `data/inverted_index.bin` and `data/trie.bin`.

```bash
cargo run --manifest-path indexer/Cargo.toml
```

### 3. Running the Search

To start the search engine interface. This loads the generated index files and allows you to query the dataset.

```bash
cargo run --manifest-path search/Cargo.toml
```

### 4. Running the Server

To start the REST API server on `http://127.0.0.1:8080`.

```bash
cargo run --manifest-path server/Cargo.toml
```

### 5. Running the Client

To start the web interface. Ensure you have Node.js installed.

**Important:** The client requires a symbolic link to the Wikipedia dump to serve the files locally.

1.  Create the symbolic link (if not already created):

    ```bash
    # Run from the project root
    ln -s "../../wikipedia-simple-html-dump" "client/static/wiki"
    ```

2.  Install dependencies and run the development server:
    ```bash
    cd client
    npm install
    npm run dev
    ```

The client will be available at `http://localhost:5173`. Clicking on search results will open the local Wikipedia pages directly in your browser.

### Running Tests

To run the unit tests for the Linguist library (stemmer, tokenizer, etc.):

```bash
cargo test --manifest-path linguist/Cargo.toml
```

## Data Source

The data used for this search engine is sourced from the Wikimedia Dumps, specifically the Simple English Wikipedia Database dump, which can be found at https://dumps.wikimedia.org/.
The lemmatization list is sourced from [michmech/lemmatization-lists](https://github.com/michmech/lemmatization-lists).
The stopwords list is sourced from [stopwords-iso/stopwords-en](https://github.com/stopwords-iso/stopwords-en).
The stemming algorithm implementation is adapted from [rust-stem](https://github.com/minhnhdo/rust-stem).
