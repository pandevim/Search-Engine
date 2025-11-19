<script lang="ts">
  import { onMount } from 'svelte';

  interface SearchResult {
    id: number;
    path: string;
    title: string;
  }

  interface SearchResponse {
    query: string;
    results: SearchResult[];
    total_results: number;
    time_taken_ms: number;
  }

  let query = '';
  let response: SearchResponse | null = null;
  let loading = false;
  let error: string | null = null;

  async function handleSearch() {
    if (!query.trim()) return;

    loading = true;
    error = null;
    response = null;

    try {
      const res = await fetch(`http://127.0.0.1:8080/search?q=${encodeURIComponent(query)}`);
      if (!res.ok) {
        throw new Error('Failed to fetch results');
      }
      response = await res.json();
    } catch (err) {
      error = (err as Error).message;
    } finally {
      loading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleSearch();
    }
  }
</script>

<main class="container">
  <div class="search-box">
    <img src="/logo.png" alt="Logo" />
    <div class="input-group">
      <input
        type="text"
        bind:value={query}
        on:keydown={handleKeydown}
        placeholder="Search Wikipedia"
        disabled={loading}
      />
      <button on:click={handleSearch} disabled={loading}>
        {'Search'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="error">
      <p>Error: {error}</p>
    </div>
  {/if}

  {#if response}
    <div class="results-info">
      <p>
        Found <strong>{response.total_results}</strong> results in
        <strong>{response.time_taken_ms.toFixed(2)}ms</strong>
      </p>
    </div>

    <ul class="results-list">
      {#each response.results as result}
        <li class="result-item">
          <a href={`/wiki/${result.path}`} target="_blank" rel="noopener noreferrer">
            <div class="title">{result.title}</div>
            <span class="path">{result.path}</span>
          </a>
        </li>
      {/each}
    </ul>
  {:else if !loading && !error && query}
    <!-- Optional: State when no search has been performed yet but query exists (maybe cleared results) -->
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen,
      Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
    background-color: #f9f9f9;
    color: #333;
  }

  .container {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
  }

  .search-box {
    text-align: center;
    margin-bottom: 2rem;
  }

  .input-group {
    display: flex;
    justify-content: center;
  }

  input {
    padding: 0.75rem 1rem;
    font-size: 1rem;
    border: 1px solid #ddd;
    border-right: none;
    width: 100%;
    max-width: 400px;
  }

  input:focus {
    outline: none;
    border-color: none;
  }

  button {
    padding: 0.75rem 1.5rem;
    font-size: 1rem;
    cursor: pointer;
    border: 1px solid #ddd;
  }

  button:hover:not(:disabled) {
    background-color: #f3f3f3;
  }

  button:disabled {
    background-color: #95a5a6;
    cursor: not-allowed;
  }

  .results-info {
    margin-bottom: 1rem;
    color: #7f8c8d;
    font-size: 0.9rem;
  }

  .results-list {
    list-style: none;
    padding: 0;
  }

  .result-item {
    background: white;
    padding: 1rem;
    margin-bottom: 0.5rem;
  }

  .result-item a {
    text-decoration: none;
    color: inherit;
    display: block;
  }

  .result-item a:hover .title {
    text-decoration: underline;
  }

  .title {
    color: #2980b9;
    font-weight: 500;
    font-size: 1.1rem;
    margin-bottom: 0.25rem;
  }

  .path {
    font-family: monospace;
    color: #27ae60;
    font-size: 0.85rem;
    display: block;
  }

  .error {
    color: #e74c3c;
    text-align: center;
    margin-top: 1rem;
  }
</style>
