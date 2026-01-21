<script lang="ts">
  import { dev } from '$app/environment';
  import { env } from '$env/dynamic/public';

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

  let currentPage = 0;
  const perPage = 10;

  function getLink(path: string): string {
    if (dev) {
      return `/wiki/${path}`;
    } else {
      // Remove .html extension for live wikipedia
      const articleName = path.replace(/\.html$/, '');
      return `https://simple.wikipedia.org/wiki/${articleName}`;
    }
  }

  $: results = response ? response.results : [];
  $: totalRows = results.length;
  $: totalPages = Math.ceil(totalRows / perPage);
  
  // Reset page when results change
  $: totalRows, currentPage = 0;

  $: start = currentPage * perPage;
  $: end = currentPage === totalPages - 1 ? totalRows - 1 : start + perPage - 1;
  $: trimmedResults = results.slice(start, end + 1);

  $: if (!query.trim()) {
    response = null;
    error = null;
  }

  async function handleSearch() {
    if (!query.trim()) return;

    loading = true;
    error = null;
    response = null;

    const apiUrl = env.PUBLIC_API_URL || 'http://127.0.0.1:8080/search';

    try {
      const res = await fetch(`${apiUrl}?q=${encodeURIComponent(query)}`);
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

<main class="container mx-auto max-w-[800px] p-8">
  <div class="text-center mb-8">
    <img src="/logo.png" alt="Logo" class="mx-auto" />
    <div class="flex justify-center">
      <input
        type="text"
        bind:value={query}
        on:keydown={handleKeydown}
        placeholder="Search Wikipedia"
        disabled={loading}
        class="w-full max-w-[400px] px-4 py-3 text-base border border-[#ddd] border-r-0 focus:outline-none"
      />
      <button 
        on:click={handleSearch} 
        disabled={loading}
        class="px-6 py-3 text-base cursor-pointer border border-[#ddd] hover:not-disabled:bg-[#f3f3f3] disabled:bg-[#95a5a6] disabled:cursor-not-allowed"
      >
        {'Search'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="text-center mt-4 text-[#e74c3c]">
      <p>Error: {error}</p>
    </div>
  {/if}

  {#if response}
    <div class="mb-4 text-sm text-[#7f8c8d]">
      <p>
        Found <strong>{response.total_results}</strong> results in
        <strong>{response.time_taken_ms.toFixed(2)}ms</strong>
      </p>
    </div>

    <ul class="list-none p-0">
      {#each trimmedResults as result}
        <li class="bg-white p-4 mb-2">
          <a href={getLink(result.path)} target="_blank" rel="noopener noreferrer" class="block no-underline text-inherit group">
            <div class="text-[#2980b9] font-medium text-lg mb-1 group-hover:underline">{result.title}</div>
            <span class="block font-mono text-[#27ae60] text-sm">{getLink(result.path)}</span>
          </a>
        </li>
      {/each}
    </ul>

    {#if totalRows > perPage}
      <div class="flex items-center justify-center mt-8">
        <button
          on:click={() => (currentPage -= 1)}
          disabled={currentPage === 0}
          aria-label="Previous page"
          class="px-4 py-2 border-none disabled:bg-[#e1e1e1] disabled:opacity-50"
        >
          &lt;
        </button>
        <p class="mx-4">{start + 1} - {end + 1} of {totalRows}</p>
        <button
          on:click={() => (currentPage += 1)}
          disabled={currentPage === totalPages - 1}
          aria-label="Next page"
          class="px-4 py-2 border-none disabled:bg-[#e1e1e1] disabled:opacity-50 cursor-pointer"
        >
          &gt;
        </button>
      </div>
    {/if}
  {:else if !loading && !error && query}
    <!-- Optional: State when no search has been performed yet but query exists (maybe cleared results) -->
  {/if}
</main>


