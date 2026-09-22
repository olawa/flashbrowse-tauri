<script lang="ts">
  import { listArchiveContents } from '../invoke';
  import type { FileItem, ArchiveSummary, ArchiveEntry } from '../types';
  import { Archive, RefreshCw, Search, FileText, Folder } from 'lucide-svelte';

  export let item: FileItem;

  let summary: ArchiveSummary | null = null;
  let isLoading = false;
  let errorMessage = '';
  let filterText = '';

  $: if (item) {
    loadArchive(item.path);
  }

  async function loadArchive(path: string) {
    isLoading = true;
    errorMessage = '';
    summary = null;
    try {
      summary = await listArchiveContents(path);
    } catch (e: any) {
      errorMessage = String(e);
    } finally {
      isLoading = false;
    }
  }

  $: filteredEntries = (summary?.entries || []).filter((e) => {
    if (!filterText.trim()) return true;
    return e.name.toLowerCase().includes(filterText.toLowerCase());
  });
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden bg-[var(--bg-base)] text-[var(--text-primary)]">
  <!-- Header Bar -->
  <div class="px-3 py-2 bg-[var(--bg-surface)] border-b border-[var(--border)] flex items-center justify-between">
    <div class="flex items-center gap-2 min-w-0">
      <div class="w-6 h-6 rounded bg-amber-500/20 text-amber-400 flex items-center justify-center shrink-0">
        <Archive size={14} />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1.5 flex-wrap">
          <span class="font-bold text-xs text-[var(--text-primary)] break-all leading-snug select-text" title={item.path}>{item.name}</span>
          <span class="px-1.5 py-0.2 rounded bg-amber-950 text-amber-400 text-[10px] font-mono border border-amber-800 shrink-0">
            ARKIV
          </span>
        </div>
        {#if summary}
          <div class="text-[11px] text-[var(--text-secondary)] font-mono flex items-center gap-1.5 mt-0.5">
            <span>{summary.total_files} filer</span>
            <span>•</span>
            <span class="text-amber-300 font-semibold">{summary.formatted_uncompressed_size} uppackad</span>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Search Filter -->
  <div class="px-3 py-2 border-b border-[var(--border)] bg-[var(--bg-surface)]">
    <div class="relative">
      <Search size={12} class="absolute left-2.5 top-2 text-[var(--text-muted)]" />
      <input
        type="text"
        bind:value={filterText}
        placeholder="Sök i arkivets innehåll..."
        class="w-full bg-[var(--bg-base)] border border-[var(--border)] rounded-lg pl-7 pr-3 py-1 text-xs text-[var(--text-primary)] focus:outline-none focus:border-amber-400"
      />
    </div>
  </div>

  <!-- Entries List -->
  <div class="flex-1 overflow-auto p-2 text-xs select-text">
    {#if isLoading}
      <div class="h-full flex items-center justify-center text-[var(--text-secondary)] gap-2">
        <RefreshCw size={14} class="animate-spin text-amber-400" />
        <span>Läser in arkivets filförteckning...</span>
      </div>
    {:else if errorMessage}
      <div class="p-4 rounded-xl bg-red-950/30 border border-red-800 text-red-400 space-y-2">
        <span class="font-bold block">Kunde inte läsa arkiv:</span>
        <p class="text-xs font-mono">{errorMessage}</p>
      </div>
    {:else if summary}
      <div class="border border-[var(--border)] rounded-lg bg-[var(--bg-base)] overflow-hidden">
        <table class="w-full text-left font-mono text-[11px] border-collapse">
          <thead>
            <tr class="border-b border-[var(--border)] bg-[var(--bg-panel)] text-[var(--text-secondary)] text-[10px]">
              <th class="p-1.5 pl-3">Filnamn</th>
              <th class="p-1.5 text-right">Storlek</th>
              <th class="p-1.5 text-right pr-3">Datum</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredEntries as e}
              <tr class="border-b border-[var(--border)] hover:bg-white/5">
                <td class="p-1.5 pl-3 flex items-center gap-1.5 truncate max-w-[200px]" title={e.name}>
                  {#if e.is_dir}
                    <Folder size={12} class="text-amber-400 shrink-0" />
                  {:else}
                    <FileText size={12} class="text-[var(--text-secondary)] shrink-0" />
                  {/if}
                  <span class="truncate {e.is_dir ? 'font-semibold text-amber-200' : 'text-[var(--text-primary)]'}">{e.name}</span>
                </td>
                <td class="p-1.5 text-right text-[var(--text-secondary)] whitespace-nowrap">{e.formatted_size}</td>
                <td class="p-1.5 text-right pr-3 text-[var(--text-muted)] text-[10px] whitespace-nowrap">{e.modified_str}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>
