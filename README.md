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

### 2. Normalization

The `normalization` component is a text processing library responsible for preparing raw text for indexing.

#### Pipeline

1.  **Casefolding**: Converts all text to lowercase to ensure case-insensitive matching.
2.  **Whitelist Check**: Checks tokens against a whitelist (`data/whitelist.txt`) to preserve special terms (e.g., "C++", ".NET") that would otherwise be split by the tokenizer.
3.  **Tokenization**: Splits text into individual tokens (words) based on punctuation and whitespace.
4.  **Stopword Removal**: Filters out common English filler words (e.g., "the", "is", "at") to reduce index size and improve relevance. Uses `data/stopwords-en.txt`.
5.  **Lemmatization**: Reduces words to their root form (e.g., "running" $\rightarrow$ "run") using a dictionary-based approach sourced from `data/lemmatization-en.txt` ([Source](https://github.com/michmech/lemmatization-lists)).

### 3. Indexer

The indexer builds an inverted index from the processed documents.

#### Architecture

Following the standard inverted index design:

1.  **Inverted Index (Occurrence Lists)**: A binary file (`data/inverted_index.bin`) storing the list of postings for each term. Each posting contains the Document ID and the list of positions where the term appears (for window scoring).
2.  **Document Metadata**: A binary file (`data/docs.bin`) storing metadata for each document, including its path, title, and length (for BM25 scoring).
3.  **Compressed Trie**: An in-memory compressed trie (implemented using [`trie-rs`](https://github.com/laysakura/trie-rs)) that maps every term in the vocabulary to the index of its occurrence list in the binary file. The trie is serialized to `data/trie.bin`.

#### Process

1.  **Initialization**: Loads the `crawled.lst` to map Document IDs (line numbers) to file paths.
2.  **Processing**: Iterates through every document, parses the HTML to extract text and title, and uses the `Normalizer` library to tokenize and stem the content.
3.  **Index Construction**: Builds a temporary in-memory map of `Term -> DocID -> [Positions]`.
4.  **Optimization**: Sorts postings by Document ID.
5.  **Serialization**: Saves the occurrence lists, document metadata, and the Trie to disk using `bincode`.

#### Optimization: Compact Integer Encoding

To reduce the size of the inverted index, the system creates a specialized binary format using two techniques:

1.  **Delta Encoding**: Instead of storing absolute Document IDs and positions, we store the difference (delta) between consecutive values. Since these lists are sorted, the deltas are much smaller integers than the original values.
2.  **VByte (VarInt) Encoding**: These small integer deltas are compressed using Variable Byte encoding, which uses fewer bytes for smaller numbers (e.g., 1 byte for numbers < 128).

**Impact**: This optimization reduces the `inverted_index.bin` file size by approximately **75%** (e.g., from ~280 MB to ~67 MB for the full dataset).

### 4. Search & Ranking

The search component provides the user interface for querying the index and ranking results.

#### Boolean Logic

The search engine enforces a strict Boolean AND logic for multi-term queries. Given a query $Q = \{q_1, q_2, \dots, q_n\}$, the result set $R$ is defined as the intersection of the posting lists for each term:

$$R = \bigcap_{i=1}^{n} \text{Postings}(q_i)$$

This ensures that every returned document contains all the terms in the query. The intersection is computed efficiently by sorting the posting lists by Document ID and using a linear scan algorithm.

#### Ranking Algorithm

The search engine uses a ranking function that combines two relevance signals:

1.  **BM25 Score**: A probabilistic information retrieval model that ranks documents based on the query terms appearing in each document.

    The BM25 score for a document $D$ and query $Q$ is calculated as:

    $$\text{BM25}(D, Q) = \sum_{i=1}^{n} \text{IDF}(q_i) \cdot \frac{f(q_i, D) \cdot (k_1 + 1)}{f(q_i, D) + k_1 \cdot (1 - b + b \cdot \frac{|D|}{\text{avgdl}})}$$

    Where the Inverse Document Frequency (IDF) is:

    $$\text{IDF}(q_i) = \ln \left( \frac{N - n(q_i) + 0.5}{n(q_i) + 0.5} + 1 \right)$$

    **Parameters:**
    - $f(q_i, D)$: Term frequency of $q_i$ in $D$.
    - $|D|$: Length of document $D$.
    - $\text{avgdl}$: Average document length in the corpus.
    - $N$: Total number of documents.
    - $n(q_i)$: Number of documents containing $q_i$.
    - $k_1 = 1.2$, $b = 0.75$: Tunable parameters.

2.  **Window Score**: Measures the proximity of query terms within the document.

    $$\text{Window}(D, Q) = \frac{|Q|}{\text{min\\_window}(Q, D)}$$

    Where $\text{min\\_window}(Q, D)$ is the size of the smallest span of text in $D$ containing all terms in $Q$.

**Final Score Formula**:

$$\text{Score}(D, Q) = \alpha \cdot \text{Window}(D, Q) + \beta \cdot \text{BM25}(D, Q)$$

Where $\alpha$ and $\beta$ are weights for the respective scores.

#### Features

- **Ranked Results**: Returns results sorted by relevance score.
- **Fast Retrieval**: Uses optimized data structures for sub-millisecond query times.

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

The client is a web interface built with **SvelteKit** that consumes the REST API.

**Note**: The client source code has been moved to a separate repository: [search-engine-client](https://github.com/pandevim/search-engine-client)

### 7. Vercel API (Serverless)

The `api` component wraps the core search logic in a Vercel Serverless Function, allowing the search engine to be deployed to the cloud without managing a dedicated server. It shares the same `AppState` loading mechanism as the main server but is optimized for "cold starts" using `OnceLock`.

## Usage

### Prerequisites

- Rust toolchain (Cargo, rustc)
- Local Wikipedia dump extracted to `wikipedia-simple-html-dump/` directory
- Lemmatization dictionary at `data/lemmatization-en.txt` ([Source](https://github.com/michmech/lemmatization-lists))
- Stopwords list at `data/stopwords-en.txt`
- Whitelist at `data/whitelist.txt`

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

### Running the Server

To start the REST API server on `http://127.0.0.1:8080`.

```bash
cargo run --manifest-path server/Cargo.toml --release
```

### Running the Client

The web interface is hosted in a separate repository. To set it up:

1. Clone the client repository:

   ```bash
   git clone https://github.com/pandevim/search-engine-client.git
   ```

2. Follow the setup instructions in the `search-engine-client` [README](https://github.com/pandevim/search-engine-client/blob/main/README.md).

3. Ensure your backend server (from this repository) is running on http://127.0.0.1:8080 (or configure the client to point to your custom port).

### Running Tests

To run the unit tests for the Normalizer library (lemmatization, tokenizer, etc.):

```bash
cargo test --manifest-path normalization/Cargo.toml
```

### Generating Test Fixtures

A Python script is included to generate a set of test cases for the search API, covering various boundary conditions. This script requires the server to be running.

1.  Ensure the server is running (see above).
2.  Run the script from the project root:
    ```bash
    python3 generate_fixtures.py
    ```

This will create a `fixtures/` directory populated with input (`.txt`) and output (`.json`) files for each test case, such as basic searches, empty queries, and queries with special characters.

#### Test Case Screenshots

The `screenshots/` directory contains captured images of these test cases, demonstrating the system's correct handling of various boundary conditions and edge cases.

## Deployment

This project is configured for serverless deployment on [Vercel](https://vercel.com).

### Vercel Serverless Function

The `api` directory contains a Rust-based serverless function that powers the search backend. It is optimized for the Vercel Runtime v2 to ensure high performance and low latency.

#### Configuration

- **`vercel.json`**: Configures the rewrites to route `/api/search` to the Rust function.
- **`api/Cargo.toml`**: Defines the `search` binary and its dependencies, including `vercel_runtime`.

#### Deploying to Vercel

1.  **Install Vercel CLI**:

    ```bash
    npm i -g vercel
    ```

2.  **Deploy**:
    Run the following command in the project root:

    ```bash
    vercel
    ```

    **Note**: The project includes ~85MB of data in the `data/` directory. Vercel's serverless function size limit is 50MB (zipped). If deployment fails due to size limits, consider hosting the data files externally (e.g., S3, Vercel Blob) or pruning the index size.

## Data Source

The data used for this search engine is sourced from the Wikimedia Dumps, specifically the [Simple English Wikipedia Database dump](https://dumps.wikimedia.org/other/static_html_dumps/current/en/), which can be found at [dumps.wikimedia.org](https://dumps.wikimedia.org/) or my personal [Google Drive](https://drive.google.com/file/d/1LqYcD7N9H8YC9W1YI6puTX3JwZ2NlGOZ/view?usp=sharing).
The lemmatization list is sourced from [michmech/lemmatization-lists](https://github.com/michmech/lemmatization-lists).
The stopwords list is sourced from the `nltk` python library.
