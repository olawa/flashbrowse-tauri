<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    terminalLines,
    commandHistory,
    historyIndex,
    isTerminalOpen,
    terminalDockPosition,
    toggleTerminalDock,
    executeTerminalCommand,
    requestTabCompletion,
    isExecuting,
  } from '../stores/terminal';
  import { leftPane, rightPane, activePaneId } from '../stores/navigation';
  import { terminalHeight, terminalWidth } from '../stores/layoutStore';
  import {
    Terminal as TerminalIcon,
    X,
    Trash2,
    LayoutPanelLeft,
    PanelBottom,
    CornerDownLeft,
  } from 'lucide-svelte';

  let inputVal = '';
  let inputEl: HTMLInputElement;
  let scrollContainer: HTMLDivElement;

  $: activeId = $activePaneId;
  $: currentPane = activeId === 'left' ? $leftPane : $rightPane;
  $: isSideDocked = $terminalDockPosition === 'side';

  $: if ($terminalLines.length && scrollContainer) {
    tick().then(() => {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    });
  }

  async function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      const cmd = inputVal;
      inputVal = '';
      await executeTerminalCommand(cmd);
    } else if (e.key === 'Tab') {
      e.preventDefault();
      const completed = await requestTabCompletion(inputVal);
      inputVal = completed;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const hist = $commandHistory;
      if (hist.length === 0) return;
      historyIndex.update((idx) => {
        const next = idx === -1 ? hist.length - 1 : Math.max(0, idx - 1);
        inputVal = hist[next] || '';
        return next;
      });
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      const hist = $commandHistory;
      historyIndex.update((idx) => {
        if (idx === -1) return -1;
        const next = idx + 1;
        if (next >= hist.length) {
          inputVal = '';
          return -1;
        } else {
          inputVal = hist[next] || '';
          return next;
        }
      });
    }
  }

  onMount(() => {
    inputEl?.focus();
  });
</script>

<div
  class="flex flex-col bg-[#0c0d10] border-[var(--border)] select-none shrink-0 {isSideDocked
    ? 'h-full border-l'
    : 'w-full border-t'}"
  style={isSideDocked ? `width: ${$terminalWidth}px; min-width: 240px; max-width: 850px;` : `height: ${$terminalHeight}px; min-height: 120px; max-height: 600px;`}
>
  <!-- Terminal Header -->
  <div class="flex items-center justify-between px-3 py-1.5 bg-[var(--bg-surface)] border-b border-[var(--border)] text-xs">
    <div class="flex items-center gap-2 text-[var(--text-secondary)]">
      <TerminalIcon size={13} class="text-[var(--accent)]" />
      <span class="font-bold text-[var(--text-primary)]">Terminal</span>
      <span class="text-[var(--text-muted)] font-mono text-[10px] truncate max-w-[180px]">
        {currentPane.currentPath}
      </span>
    </div>

    <div class="flex items-center gap-1">
      <!-- Dock Toggle Button -->
      <button
        class="flex items-center gap-1 px-2 py-0.5 rounded border border-[var(--border)] text-[10px] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)]"
        on:click={toggleTerminalDock}
        title="Toggle Dock Position (Bottom / Side)"
      >
        {#if isSideDocked}
          <PanelBottom size={11} />
          <span>Bottom</span>
        {:else}
          <LayoutPanelLeft size={11} />
          <span>Side</span>
        {/if}
      </button>

      <!-- Clear Output -->
      <button
        class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)]"
        on:click={() => terminalLines.set([])}
        title="Clear Terminal Output"
      >
        <Trash2 size={12} />
      </button>

      <!-- Close Terminal -->
      <button
        class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)]"
        on:click={() => isTerminalOpen.set(false)}
        title="Close Terminal (Cmd+J)"
      >
        <X size={12} />
      </button>
    </div>
  </div>

  <!-- Terminal Output Lines -->
  <div
    bind:this={scrollContainer}
    class="flex-1 overflow-y-auto p-2.5 font-mono text-xs text-slate-200 space-y-1 select-text"
  >
    {#each $terminalLines as line (line.id)}
      <div
        class="leading-relaxed break-words whitespace-pre-wrap {line.isError
          ? 'text-red-400 font-semibold'
          : line.isPrompt
          ? 'text-cyan-400 font-bold'
          : 'text-slate-300'}"
      >
        {line.text}
      </div>
    {/each}
  </div>

  <!-- Terminal Prompt & Input -->
  <div class="flex items-center gap-1.5 px-3 py-2 bg-[var(--bg-surface)] border-t border-[var(--border)]">
    <span class="text-cyan-400 font-mono text-xs font-bold">$</span>
    <input
      bind:this={inputEl}
      type="text"
      bind:value={inputVal}
      on:keydown={handleKeyDown}
      placeholder="Type command (Tab to complete)..."
      class="flex-1 bg-transparent text-xs text-white font-mono focus:outline-none border-none p-0"
    />
    {#if $isExecuting}
      <div class="w-3 h-3 rounded-full border-2 border-[var(--accent)] border-t-transparent animate-spin"></div>
    {/if}
  </div>
</div>
