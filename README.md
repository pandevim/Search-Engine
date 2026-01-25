# Simple Wikipedia Search Engine

**Author:** Aniruddha Pandey  
**Course:** CS 600-A Advanced Algorithm Design and Implementation

---

## Project Overview

This project is a search engine built in Rust, designed to index and search a local copy of the Simple English Wikipedia.

## Project Components

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

### 2. Indexer

The indexer builds an inverted index from the processed documents.

#### Architecture

Following the standard inverted index design:

1.  **Inverted Index (Occurrence Lists)**: A binary file (`data/inverted_index.bin`) storing the list of postings for each term. Each posting contains the Document ID and the list of positions where the term appears (for window scoring).
2.  **Document Metadata**: A binary file (`data/docs.bin`) storing metadata for each document, including its path, title, and length (for BM25 scoring).
3.  **Compressed Trie**: An in-memory compressed trie (implemented using [`trie-rs`](https://github.com/laysakura/trie-rs)) that maps every term in the vocabulary to the index of its occurrence list in the binary file. The trie is serialized to `data/trie.bin`.

#### Process

1.  **Initialization**: Loads the `crawled.lst` to map Document IDs (line numbers) to file paths.
2.  **Processing**: Iterates through every document, parses the HTML to extract text and title, and uses the [`search_normalizer`](https://github.com/pandevim/search_normalizer) crate to tokenize and stem the content.
3.  **Index Construction**: Builds a temporary in-memory map of `Term -> DocID -> [Positions]`.
4.  **Optimization**: Sorts postings by Document ID.
5.  **Serialization**: Saves the occurrence lists, document metadata, and the Trie to disk using `bincode`.

#### Optimization: Compact Integer Encoding

To reduce the size of the inverted index, the system creates a specialized binary format using two techniques:

1.  **Delta Encoding**: Instead of storing absolute Document IDs and positions, we store the difference (delta) between consecutive values. Since these lists are sorted, the deltas are much smaller integers than the original values.
2.  **VByte (VarInt) Encoding**: These small integer deltas are compressed using Variable Byte encoding, which uses fewer bytes for smaller numbers (e.g., 1 byte for numbers < 128).

**Impact**: This optimization reduces the `inverted_index.bin` file size by approximately **75%** (e.g., from ~280 MB to ~67 MB for the full dataset).

### 3. Server (REST API)

The server component exposes the search functionality via a RESTful API using `axum` moved to a separate repository: [search-engine-serve](https://github.com/pandevim/search-engine-serve).

### 4. Client (Web UI)

The client is a web interface built with **SvelteKit** that consumes the REST API.

**Note**: The client source code has been moved to a separate repository: [search-engine-client](https://github.com/pandevim/search-engine-client)

## Usage

### Prerequisites

- Rust toolchain (Cargo, rustc)
- Local Wikipedia dump extracted to `wikipedia-simple-html-dump/` directory

### Configuration

The crawler uses a `seed.lst` file in the `data/` directory to determine the entry point.

```text
simple/index.html
```

### Running the Crawler

To execute the crawler from the project root. This will traverse the `wikipedia-simple-html-dump` directory and generate `data/crawled.lst`.

```bash
cargo run --manifest-path crawler/Cargo.toml --release
```

### Running the Indexer

To build the index from the crawled data. This will process the files listed in `crawled.lst` and generate `data/inverted_index.bin` and `data/trie.bin`.

```bash
cargo run --manifest-path indexer/Cargo.toml --release
```

## Data Source

The data used for this search engine is sourced from the Wikimedia Dumps, specifically the [Simple English Wikipedia Database dump](https://dumps.wikimedia.org/other/static_html_dumps/current/en/), which can be found at [dumps.wikimedia.org](https://dumps.wikimedia.org/) or my personal [Google Drive](https://drive.google.com/file/d/1LqYcD7N9H8YC9W1YI6puTX3JwZ2NlGOZ/view?usp=sharing).
