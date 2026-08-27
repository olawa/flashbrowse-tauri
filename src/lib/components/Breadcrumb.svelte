<script lang="ts">
  import { goBack, goForward, goUp, navigatePane, leftPane, rightPane, isDualInspector } from '../stores/navigation';
  import { ChevronRight, ArrowLeft, ArrowRight, ArrowUp, RefreshCw, LayoutGrid, Server, HardDrive } from 'lucide-svelte';

  export let paneId: 'left' | 'right' = 'left';

  $: pane = paneId === 'left' ? $leftPane : $rightPane;
  $: pathSegments = pane.currentPath.split('/').filter(Boolean);

  function navigateToSegment(index: number) {
    const target = '/' + pathSegments.slice(0, index + 1).join('/');
    navigatePane(paneId, target);
  }
</script>

<div class="flex items-center gap-1.5 px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] text-xs text-[var(--text-secondary)] select-none">
  <!-- History Controls -->
  <div class="flex items-center gap-0.5 mr-1">
    <button
      class="p-1 rounded hover:bg-[var(--bg-hover)] disabled:opacity-30 disabled:hover:bg-transparent"
      disabled={pane.historyIndex <= 0}
      on:click={() => goBack(paneId)}
      title="Back"
    >
      <ArrowLeft size={13} />
    </button>

    <button
      class="p-1 rounded hover:bg-[var(--bg-hover)] disabled:opacity-30 disabled:hover:bg-transparent"
      disabled={pane.historyIndex >= pane.history.length - 1}
      on:click={() => goForward(paneId)}
      title="Forward"
    >
      <ArrowRight size={13} />
    </button>

    <button
      class="p-1 rounded hover:bg-[var(--bg-hover)]"
      on:click={() => goUp(paneId)}
      title="Up one level"
    >
      <ArrowUp size={13} />
    </button>
  </div>

  <!-- Breadcrumb Path Segments -->
  <div class="flex items-center gap-1 overflow-x-auto whitespace-nowrap flex-1 py-0.5">
    {#if pane.isSSH}
      <span class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-green-900/30 text-green-400 font-medium">
        <Server size={12} />
        {pane.sshHost}
      </span>
      <ChevronRight size={12} class="opacity-40" />
    {:else}
      <button
        class="flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] text-[var(--text-primary)]"
        on:click={() => navigatePane(paneId, '/')}
      >
        <HardDrive size={12} />
        <span>Root</span>
      </button>
      <ChevronRight size={12} class="opacity-40" />
    {/if}

    {#each pathSegments as segment, index}
      <button
        class="px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] transition-colors {index === pathSegments.length - 1 ? 'font-semibold text-[var(--accent)]' : 'text-[var(--text-primary)]'}"
        on:click={() => navigateToSegment(index)}
      >
        {segment}
      </button>
      {#if index < pathSegments.length - 1}
        <ChevronRight size={12} class="opacity-40" />
      {/if}
    {/each}
  </div>

  <!-- Dual Inspector toggle & Search -->
  <div class="flex items-center gap-1.5 ml-auto">
    <button
      class="px-2 py-0.5 rounded border border-[var(--border)] text-[11px] hover:bg-[var(--bg-hover)] flex items-center gap-1 {$isDualInspector ? 'bg-[var(--accent-subtle)] text-[var(--accent)] border-[var(--accent)]' : ''}"
      on:click={() => isDualInspector.update((v) => !v)}
      title="Toggle Dual Inspector"
    >
      <LayoutGrid size={11} />
      <span>Dual Inspector</span>
    </button>
  </div>
</div>
