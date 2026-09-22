<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    isTerminalOpen,
    terminalDockPosition,
    toggleTerminalDock,
    activeTerminalTab,
    pendingTerminalNav,
  } from '../stores/terminal';
  import { leftPane, rightPane, activePaneId, navigatePane } from '../stores/navigation';
  import { terminalHeight, terminalWidth } from '../stores/layoutStore';
  import XtermTerminal from './XtermTerminal.svelte';
  import {
    Terminal as TerminalIcon,
    X,
    Trash2,
    RotateCw,
    LayoutPanelLeft,
    PanelBottom,
    Server,
    Laptop,
  } from 'lucide-svelte';

  let localTermRef: XtermTerminal;
  let sshTermRef: XtermTerminal;

  $: activeId = $activePaneId;
  $: currentPane = activeId === 'left' ? $leftPane : $rightPane;
  $: isSideDocked = $terminalDockPosition === 'side';

  // Track SSH connection availability
  $: isCurrentPaneSSH = currentPane.isSSH;
  $: sshHost = currentPane.sshHost || '';
  $: sshHostShort = sshHost ? sshHost.split('.')[0] : '';
  $: sshSessionId = sshHost ? `ssh-${sshHost.replace(/[^a-zA-Z0-9_-]/g, '_')}` : '';

  // Remember if SSH was ever used during this session so tab stays accessible
  let knownSshHost = '';
  $: if (sshHost) {
    knownSshHost = sshHost;
  }

  // Switch tab automatically when active pane mode changes (unless user explicitly switched)
  let lastPaneWasSSH: boolean | null = null;
  $: if (isCurrentPaneSSH !== lastPaneWasSSH) {
    lastPaneWasSSH = isCurrentPaneSSH;
    if (isCurrentPaneSSH && knownSshHost) {
      activeTerminalTab.set('ssh');
    } else if (!isCurrentPaneSSH && $activeTerminalTab === 'ssh' && !knownSshHost) {
      activeTerminalTab.set('local');
    }
  }

  // Handle pending navigation request (e.g. from context menu "Open in Terminal")
  let lastHandledNavTimestamp = 0;
  $: if ($pendingTerminalNav && $pendingTerminalNav.timestamp !== lastHandledNavTimestamp) {
    const nav = $pendingTerminalNav;
    lastHandledNavTimestamp = nav.timestamp;
    tick().then(() => {
      if (nav.isSSH && sshTermRef) {
        sshTermRef.sendCd(nav.path);
        sshTermRef.focus();
      } else if (!nav.isSSH && localTermRef) {
        localTermRef.sendCd(nav.path);
        localTermRef.focus();
      }
    });
  }

  // Focus active terminal when opened or tab changed
  $: if ($isTerminalOpen) {
    tick().then(() => {
      if ($activeTerminalTab === 'ssh') {
        sshTermRef?.focus();
      } else {
        localTermRef?.focus();
      }
    });
  }

  function handleDirectoryChange(newPath: string, isSSH: boolean) {
    if (!newPath) return;
    const pane = activeId === 'left' ? $leftPane : $rightPane;
    if (pane.isSSH === isSSH && pane.currentPath !== newPath) {
      navigatePane(activeId, newPath, false);
    }
  }

  function handleRestart() {
    if ($activeTerminalTab === 'ssh') {
      sshTermRef?.restart();
    } else {
      localTermRef?.restart();
    }
  }

  function handleClear() {
    if ($activeTerminalTab === 'ssh') {
      sshTermRef?.clear();
    } else {
      localTermRef?.clear();
    }
  }

  function selectTab(tab: 'local' | 'ssh') {
    activeTerminalTab.set(tab);
    tick().then(() => {
      if (tab === 'ssh') {
        sshTermRef?.focus();
      } else {
        localTermRef?.focus();
      }
    });
  }
</script>

<div
  class="flex flex-col bg-[var(--bg-base)] border-[var(--border)] select-none shrink-0 overflow-hidden {isSideDocked
    ? 'h-full border-l'
    : 'w-full border-t'}"
  style={isSideDocked
    ? `width: ${$terminalWidth}px; min-width: 260px; max-width: 950px;`
    : `height: ${$terminalHeight}px; min-height: 140px; max-height: 700px;`}
>
  <!-- Terminal Header -->
  <div class="flex items-center justify-between px-3 py-1.5 bg-[var(--bg-surface)] border-b border-[var(--border)] text-xs shrink-0">
    <div class="flex items-center gap-2 text-[var(--text-secondary)] min-w-0">
      <TerminalIcon size={13} class={$activeTerminalTab === 'ssh' ? 'text-emerald-400' : 'text-[var(--accent)]'} />
      <span class="font-bold text-[var(--text-primary)]">Terminal</span>

      <!-- Tab Switcher -->
      <div class="flex items-center bg-[var(--bg-panel)] rounded p-0.5 border border-[var(--border)] ml-1">
        <button
          class="flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-medium transition-colors {$activeTerminalTab === 'local'
            ? 'bg-[var(--accent)] text-white shadow-sm'
            : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)]'}"
          on:click={() => selectTab('local')}
          title="Lokal Terminal (zsh/bash)"
        >
          <Laptop size={11} />
          <span>Lokalt</span>
        </button>

        {#if knownSshHost}
          <button
            class="flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-medium transition-colors {$activeTerminalTab === 'ssh'
              ? 'bg-emerald-600 text-white shadow-sm'
              : 'text-emerald-400 hover:text-[var(--text-primary)] hover:bg-emerald-950/50'}"
            on:click={() => selectTab('ssh')}
            title="Fjärrterminal över SSH ({knownSshHost})"
          >
            <Server size={11} />
            <span class="font-mono">{sshHostShort || knownSshHost.split('.')[0]}</span>
          </button>
        {/if}
      </div>

      <!-- Current directory badge -->
      <span
        class="font-mono text-[10px] truncate max-w-[220px] px-1.5 py-0.5 rounded bg-black/30 border border-[var(--border)] {$activeTerminalTab === 'ssh' ? 'text-emerald-400/90' : 'text-[var(--text-muted)]'}"
        title={currentPane.currentPath}
      >
        {currentPane.currentPath || ($activeTerminalTab === 'ssh' ? '~' : '/')}
      </span>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-1 shrink-0">
      <!-- Restart Process -->
      <button
        class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
        on:click={handleRestart}
        title="Starta om terminalsession"
      >
        <RotateCw size={12} />
      </button>

      <!-- Clear Terminal -->
      <button
        class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
        on:click={handleClear}
        title="Rensa terminalskärm"
      >
        <Trash2 size={12} />
      </button>

      <!-- Dock Toggle Button -->
      <button
        class="flex items-center gap-1 px-2 py-0.5 rounded border border-[var(--border)] text-[10px] text-[var(--text-secondary)] hover:bg-[var(--bg-hover)] ml-1"
        on:click={toggleTerminalDock}
        title="Växla dockningsposition (Underkant / Sida)"
      >
        {#if isSideDocked}
          <PanelBottom size={11} />
          <span>Under</span>
        {:else}
          <LayoutPanelLeft size={11} />
          <span>Sida</span>
        {/if}
      </button>

      <!-- Close Terminal -->
      <button
        class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] ml-1"
        on:click={() => isTerminalOpen.set(false)}
        title="Stäng terminal (Cmd+J)"
      >
        <X size={13} />
      </button>
    </div>
  </div>

  <!-- Terminal Emulators (kept active in DOM to prevent losing shell state) -->
  <div class="flex-1 min-h-0 relative w-full h-full overflow-hidden">
    <!-- Local Terminal -->
    <div class="absolute inset-0 w-full h-full {$activeTerminalTab === 'local' ? 'visible' : 'invisible pointer-events-none'}">
      <XtermTerminal
        bind:this={localTermRef}
        sessionId="local"
        sessionType="local"
        cwd={currentPane.isSSH ? '' : currentPane.currentPath}
        onDirectoryChange={(p) => handleDirectoryChange(p, false)}
      />
    </div>

    <!-- SSH Terminal (mounted if SSH connection is available) -->
    {#if knownSshHost && sshSessionId}
      <div class="absolute inset-0 w-full h-full {$activeTerminalTab === 'ssh' ? 'visible' : 'invisible pointer-events-none'}">
        <XtermTerminal
          bind:this={sshTermRef}
          sessionId={sshSessionId}
          sessionType="ssh"
          host={knownSshHost}
          cwd={currentPane.isSSH ? currentPane.currentPath : ''}
          onDirectoryChange={(p) => handleDirectoryChange(p, true)}
        />
      </div>
    {/if}
  </div>
</div>
