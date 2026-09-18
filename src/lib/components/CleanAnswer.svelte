<script lang="ts">
  import { Sparkles, X, AlertTriangle } from 'lucide-svelte';
  import { renderMarkdown } from '../markdown';

  /** The question as the user asked it. */
  export let question: string;
  export let answer = '';
  export let isGenerating = false;
  export let error = '';
  /** Elapsed time, so the cost of asking is visible. */
  export let seconds: number | null = null;
  export let modelName = '';
  /** What the app had to start or load before it could ask. */
  export let startupNote = '';
  export let onClose: () => void;

  // The frame and the label are the point: an answer from a model must never
  // be mistaken for something the app read off the disk.
</script>

<div class="mx-6 mt-4 rounded-lg border border-blue-500/60 bg-blue-950/20 overflow-hidden shrink-0">
  <div class="flex items-center gap-2 px-4 py-2 border-b border-blue-500/30 bg-blue-950/30">
    <Sparkles size={14} class="text-blue-400 shrink-0" />
    <span class="text-[11px] font-semibold uppercase tracking-wider text-blue-300 shrink-0">Fråga</span>
    <span class="flex-1 min-w-0 truncate text-xs text-[var(--text-primary)]">{question}</span>
    {#if modelName}
      <span class="shrink-0 text-[10px] font-mono text-blue-300/70" title={startupNote}>
        {modelName}{seconds !== null ? ` · ${seconds.toFixed(1)} s` : ''}
      </span>
    {/if}
    <button class="shrink-0 p-0.5 rounded hover:bg-blue-500/20 text-blue-300" on:click={onClose} title="Stäng svaret">
      <X size={14} />
    </button>
  </div>

  <div class="px-4 py-3 text-xs text-[var(--text-primary)] max-h-64 overflow-y-auto">
    {#if error}
      <div class="flex items-start gap-2 text-amber-400">
        <AlertTriangle size={14} class="shrink-0 mt-0.5" />
        <div>
          <div>{error}</div>
          <div class="text-[11px] text-[var(--text-muted)] mt-1">
            Filtreringen fungerar utan modell — det är bara frågorna som kräver en.
          </div>
        </div>
      </div>
    {:else if answer}
      {@html renderMarkdown(answer)}
    {:else if isGenerating}
      <div class="flex items-center gap-2 text-[var(--text-muted)]">
        <div class="w-3 h-3 border-2 border-blue-400 border-t-transparent rounded-full animate-spin"></div>
        <span>{startupNote ? `${startupNote} — läser och svarar…` : 'Läser och svarar…'}</span>
      </div>
    {/if}
  </div>

  {#if answer && !error}
    <div class="px-4 py-2 border-t border-blue-500/30 text-[10.5px] text-blue-300/70">
      Svaret är modellens. Filerna nedanför är appens — läs dem om något förvånar dig.
    </div>
  {/if}
</div>
