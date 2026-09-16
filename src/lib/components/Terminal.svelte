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
    Server,
  } from 'lucide-svelte';

  let inputVal = '';
  let inputEl: HTMLInputElement;
  let scrollContainer: HTMLDivElement;

  $: activeId = $activePaneId;
  $: currentPane = activeId === 'left' ? $leftPane : $rightPane;
  $: isSideDocked = $terminalDockPosition === 'side';
  $: isSSH = currentPane.isSSH;
  $: sshHostShort = currentPane.sshHost ? currentPane.sshHost.split('.')[0] : '';

  $: if ($terminalLines.length && scrollContainer) {
    tick().then(() => {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    });
  }

  function focusInput() {
    inputEl?.focus();
  }

  $: if ($isTerminalOpen) {
    tick().then(focusInput);
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
    tick().then(focusInput);
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
    <div class="flex items-center gap-2 text-[var(--text-secondary)] min-w-0">
      <TerminalIcon size={13} class={isSSH ? 'text-emerald-400' : 'text-[var(--accent)]'} />
      <span class="font-bold text-[var(--text-primary)]">Terminal</span>
      {#if isSSH}
        <span class="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 shrink-0">
          <Server size={10} />
          {sshHostShort}
        </span>
      {/if}
      <span class="font-mono text-[10px] truncate max-w-[200px] {isSSH ? 'text-emerald-400/80' : 'text-[var(--text-muted)]'}" title={currentPane.currentPath}>
        {currentPane.currentPath || (isSSH ? '~' : '/')}
      </span>
    </div>

    <div class="flex items-center gap-1 shrink-0">
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
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    bind:this={scrollContainer}
    on:click={focusInput}
    role="region"
    aria-label="Terminal log"
    class="flex-1 overflow-y-auto p-2.5 font-mono text-xs text-slate-200 space-y-1 select-text cursor-text"
  >
    {#each $terminalLines as line (line.id)}
      <div
        class="leading-relaxed break-words whitespace-pre-wrap {line.isError
          ? 'text-red-400 font-semibold'
          : line.isPrompt
          ? (line.text.startsWith('[') ? 'text-emerald-400 font-bold' : 'text-cyan-400 font-bold')
          : 'text-slate-300'}"
      >
        {line.text}
      </div>
    {/each}
  </div>

  <!-- Terminal Prompt & Input -->
  <div class="flex items-center gap-1.5 px-3 py-2 bg-[var(--bg-surface)] border-t border-[var(--border)]">
    {#if isSSH}
      <span class="text-emerald-400 font-mono text-xs font-bold shrink-0">{sshHostShort}:$</span>
    {:else}
      <span class="text-cyan-400 font-mono text-xs font-bold shrink-0">$</span>
    {/if}
    <input
      bind:this={inputEl}
      type="text"
      bind:value={inputVal}
      on:keydown={handleKeyDown}
      placeholder={isSSH ? `SSH remote-kommando på ${sshHostShort} (Tab för förslag)...` : 'Skriv kommando (Tab för förslag)...'}
      class="flex-1 bg-transparent text-xs text-white font-mono focus:outline-none border-none p-0"
    />
    {#if $isExecuting}
      <div class="w-3 h-3 rounded-full border-2 border-[var(--accent)] border-t-transparent animate-spin shrink-0"></div>
    {/if}
  </div>
</div>
