<script lang="ts">
  import { Folder, FileText } from 'lucide-svelte';
  import type { SearchMatch } from '../types';

  export let matches: SearchMatch[] = [];
  export let rootLabel: string;
  /** Selecting a hit previews it; a directory opens. */
  export let onSelect: (match: SearchMatch) => void;
  export let onOpen: (match: SearchMatch) => void;

  let selectedPath: string | null = null;

  function highlight(name: string, _query: string) {
    return name;
  }

  function choose(match: SearchMatch) {
    selectedPath = match.path;
    onSelect(match);
  }
</script>

<div class="flex-1 min-h-0 overflow-y-auto bg-[var(--bg-base)]">
  {#if matches.length === 0}
    <div class="p-12 flex flex-col items-center justify-center gap-2 text-center text-[var(--text-muted)]">
      <FileText size={28} class="opacity-30" />
      <span class="text-xs">Inget i {rootLabel} matchar det du skrev.</span>
      <span class="text-[11px] opacity-70">Sökningen går igenom filnamn här och i undermapparna.</span>
    </div>
  {:else}
    <div class="divide-y divide-[var(--border)]/40">
      {#each matches as match (match.path)}
        <button
          class="w-full flex items-center gap-3 px-4 py-2 text-left transition-colors {selectedPath === match.path
            ? 'bg-[var(--accent-subtle)]'
            : 'hover:bg-[var(--bg-hover)]'}"
          on:click={() => choose(match)}
          on:dblclick={() => onOpen(match)}
        >
          <div
            class="w-5 h-5 rounded flex items-center justify-center shrink-0 {match.is_dir
              ? 'bg-amber-500/20 text-amber-400'
              : 'bg-blue-500/20 text-blue-400'}"
          >
            <svelte:component this={match.is_dir ? Folder : FileText} size={12} />
          </div>

          <div class="flex-1 min-w-0 flex flex-col leading-tight">
            <span class="truncate text-xs text-[var(--text-primary)]">{highlight(match.name, '')}</span>
            <span class="truncate text-[10.5px] font-mono text-[var(--text-muted)]" title={match.path}>
              {match.relative_path || match.path}
            </span>
          </div>

          <span class="shrink-0 text-[11px] font-mono tabular-nums text-[var(--text-secondary)]">
            {match.is_dir ? '—' : match.formatted_size}
          </span>
        </button>
      {/each}
    </div>
  {/if}
</div>
