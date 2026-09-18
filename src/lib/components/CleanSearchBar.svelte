<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Search, Sparkles, X, CornerDownLeft } from 'lucide-svelte';
  import { deepSearch } from '../invoke';
  import type { SearchMatch } from '../types';

  /** The folder the search is rooted in. */
  export let rootPath: string;
  export let rootLabel: string;
  /** Results for the main area to render; empty when the field is empty. */
  export let onResults: (matches: SearchMatch[] | null) => void;
  /** A question for the model, asked from this field with Cmd+Enter. */
  export let onAsk: (question: string) => void;
  export let isAsking = false;

  let query = '';
  let mode: 'search' | 'ask' = 'search';
  let isSearching = false;
  let matchCount: number | null = null;
  let searchTimer: any = null;
  let inputEl: HTMLInputElement;

  // This is a walk of the current folder, not an index of the disk. The label
  // says so: promising "search your files" and then only reaching the folder
  // you happen to be in is the kind of thing that teaches people not to trust
  // the field.
  const MAX_RESULTS = 60;

  $: if (rootPath) {
    // A new folder invalidates what the old one matched.
    resetResults();
  }

  function resetResults() {
    clearTimeout(searchTimer);
    matchCount = null;
    isSearching = false;
    if (query.trim()) {
      query = '';
      onResults(null);
    }
  }

  function runSearch() {
    const q = query.trim();
    clearTimeout(searchTimer);

    if (mode === 'ask' || q.length < 2 || !rootPath || rootPath.startsWith('ssh:')) {
      matchCount = null;
      isSearching = false;
      onResults(null);
      return;
    }

    isSearching = true;
    // The walk costs real work per keystroke, so wait for a pause in typing.
    searchTimer = setTimeout(async () => {
      try {
        const matches = await deepSearch(rootPath, q, MAX_RESULTS);
        // A slower search for a query the user has moved on from is stale.
        if (query.trim() !== q) return;
        matchCount = matches.length;
        onResults(matches);
      } catch (err) {
        console.warn('Sökning misslyckades:', err);
        matchCount = null;
        onResults(null);
      } finally {
        isSearching = false;
      }
    }, 180);
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      if (!query.trim()) {
        mode = mode === 'ask' ? 'search' : 'ask';
        return;
      }
      mode = 'ask';
      onAsk(query.trim());
      return;
    }

    if (e.key === 'Enter' && mode === 'ask' && query.trim()) {
      e.preventDefault();
      onAsk(query.trim());
      return;
    }

    if (e.key === 'Escape') {
      if (query) {
        query = '';
        runSearch();
      } else {
        mode = 'search';
      }
      inputEl?.blur();
    }
  }

  export function focus() {
    inputEl?.focus();
  }

  /** Drop the query and show the folder again. */
  export function clear() {
    query = '';
    mode = 'search';
    runSearch();
  }

  onDestroy(() => clearTimeout(searchTimer));
</script>

<div class="px-6 pt-5 pb-4 border-b border-[var(--border)] bg-[var(--bg-base)] shrink-0">
  <!-- Ask mode is framed and labelled: the model is never the default, and it
       is always visible when it is involved. -->
  <div
    class="flex items-center gap-3 px-4 py-3 rounded-lg border transition-colors {mode === 'ask'
      ? 'bg-blue-950/30 border-blue-500/60'
      : 'bg-[var(--bg-panel)] border-[var(--border)] focus-within:border-[var(--accent)]'}"
  >
    {#if mode === 'ask'}
      <Sparkles size={18} class="text-blue-400 shrink-0" />
      <span class="text-[11px] font-semibold text-blue-300 shrink-0 uppercase tracking-wider">Fråga</span>
    {:else}
      <Search size={18} class="text-[var(--text-muted)] shrink-0" />
    {/if}

    <input
      bind:this={inputEl}
      bind:value={query}
      on:input={runSearch}
      on:keydown={handleKeydown}
      type="text"
      placeholder={mode === 'ask'
        ? 'Fråga om innehållet i filerna…'
        : `Filtrera i ${rootLabel} och undermappar`}
      class="flex-1 min-w-0 bg-transparent text-[15px] text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:outline-none"
    />

    {#if query}
      <button
        class="p-0.5 rounded hover:bg-[var(--bg-hover)] text-[var(--text-muted)] hover:text-[var(--text-primary)] shrink-0"
        on:click={() => {
          query = '';
          runSearch();
          inputEl?.focus();
        }}
        title="Rensa"
      >
        <X size={14} />
      </button>
    {/if}

    {#if mode === 'ask'}
      <button
        class="shrink-0 flex items-center gap-1 px-2 py-1 rounded text-[10px] font-mono text-blue-300 border border-blue-500/40 hover:bg-blue-500/20"
        on:click={() => {
          mode = 'search';
          runSearch();
        }}
      >
        Tillbaka till filtrering
      </button>
    {:else}
      <span
        class="shrink-0 flex items-center gap-1 px-2 py-1 rounded text-[10px] font-mono text-[var(--text-muted)] border border-[var(--border)]"
        title="Fråga en lokal modell om innehållet i stället för att filtrera på filnamn"
      >
        <CornerDownLeft size={10} /> ⌘⏎ fråga i stället
      </span>
    {/if}
  </div>

  <!-- What the field just did, in its own words. -->
  <div class="flex items-center gap-2 mt-2 px-1 text-[11px] text-[var(--text-muted)]">
    {#if mode === 'ask'}
      <span class="text-blue-300/80">
        {isAsking ? 'Frågar den lokala modellen…' : 'Svaret kommer från en lokal modell och visas bredvid det det bygger på.'}
      </span>
    {:else if isSearching}
      <span>Går igenom {rootLabel}…</span>
    {:else if matchCount !== null}
      <span>
        {matchCount === MAX_RESULTS ? `Första ${MAX_RESULTS}` : matchCount} träffar i {rootLabel} och undermappar
      </span>
      <span class="opacity-70">· filnamn, inte innehåll · ingen modell inblandad</span>
    {:else}
      <span>Filtrerar på filnamn i den här mappen och undermapparna. Inget diskomfattande index.</span>
    {/if}
  </div>
</div>
