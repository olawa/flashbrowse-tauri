<script lang="ts">
  import { tick } from 'svelte';
  import {
    goBack,
    goForward,
    goUp,
    navigatePane,
    leftPane,
    rightPane,
    isDualInspector,
    activePaneId,
    connectToSshHost,
  } from '../stores/navigation';
  import {
    ChevronRight,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    LayoutGrid,
    Server,
    HardDrive,
    Edit3,
    Check,
    Laptop,
    Clipboard,
    Copy,
    Terminal as TerminalIcon,
  } from 'lucide-svelte';
  import { isTerminalOpen, toggleTerminal, openTerminalAt } from '../stores/terminal';

  export let paneId: 'left' | 'right' = 'left';

  $: pane = paneId === 'left' ? $leftPane : $rightPane;
  $: pathSegments = pane.currentPath.split('/').filter(Boolean);

  let isEditingPath = false;
  let editPathText = '';
  let pathInputEl: HTMLInputElement;
  let isServerMenuOpen = false;

  let savedServers: Array<{ name: string; host: string }> = [
    { name: 'Marvin (HPC)', host: 'marvin.cgu.igp.uu.se' },
  ];

  function loadSavedServers() {
    try {
      const stored = localStorage.getItem('flashbrowse_ssh_servers');
      if (stored) {
        savedServers = JSON.parse(stored);
      }
    } catch {}
  }

  let isContextMenuOpen = false;
  let contextMenuX = 0;
  let contextMenuY = 0;

  function cleanPathString(input: string): string {
    let str = input.trim();
    if ((str.startsWith('"') && str.endsWith('"')) || (str.startsWith("'") && str.endsWith("'"))) {
      str = str.slice(1, -1);
    }
    if (str.startsWith('file://')) {
      str = str.replace('file://', '');
      try {
        str = decodeURIComponent(str);
      } catch {}
    }
    return str.trim();
  }

  function startEditing(prefillFromClipboard = false) {
    loadSavedServers();
    if (prefillFromClipboard) {
      navigator.clipboard.readText().then((text) => {
        const cleaned = cleanPathString(text);
        if (cleaned.startsWith('/') || cleaned.startsWith('~')) {
          editPathText = cleaned;
        } else {
          editPathText = pane.currentPath;
        }
        isEditingPath = true;
        tick().then(() => {
          pathInputEl?.focus();
          pathInputEl?.select();
        });
      }).catch(() => {
        editPathText = pane.currentPath;
        isEditingPath = true;
        tick().then(() => {
          pathInputEl?.focus();
          pathInputEl?.select();
        });
      });
      return;
    }

    editPathText = pane.currentPath;
    isEditingPath = true;
    tick().then(() => {
      pathInputEl?.focus();
      pathInputEl?.select();
    });
  }

  function commitPath() {
    if (!isEditingPath) return;
    isEditingPath = false;
    const cleaned = cleanPathString(editPathText);
    if (cleaned && cleaned !== pane.currentPath) {
      navigatePane(paneId, cleaned);
    }
  }

  async function pastePathAndGo() {
    try {
      const text = await navigator.clipboard.readText();
      const cleaned = cleanPathString(text);
      if (cleaned) {
        navigatePane(paneId, cleaned);
      }
    } catch (err) {
      console.error('Kunde inte läsa från urklipp:', err);
    }
  }

  async function pasteIntoInput() {
    try {
      const text = await navigator.clipboard.readText();
      const cleaned = cleanPathString(text);
      if (cleaned) {
        editPathText = cleaned;
        pathInputEl?.focus();
      }
    } catch (err) {
      console.error('Kunde inte klistra in:', err);
    }
  }

  let copiedPath = false;

  async function copyCurrentPath() {
    try {
      await navigator.clipboard.writeText(pane.currentPath);
      copiedPath = true;
      setTimeout(() => (copiedPath = false), 2000);
    } catch (err) {
      console.error('Kunde inte kopiera sökväg:', err);
    }
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    contextMenuX = e.clientX;
    contextMenuY = e.clientY;
    isContextMenuOpen = true;
  }

  function cancelEditing() {
    isEditingPath = false;
  }

  function navigateToSegment(index: number) {
    let target = '/' + pathSegments.slice(0, index + 1).join('/');
    if (pane.isSSH && pane.currentPath.startsWith('~')) {
      target = '~/' + pathSegments.slice(0, index + 1).join('/');
    }
    navigatePane(paneId, target);
  }

  function switchToLocal() {
    const store = paneId === 'left' ? leftPane : rightPane;
    store.update((s) => ({ ...s, isSSH: false }));
    isServerMenuOpen = false;
    navigatePane(paneId, '/');
  }

  function switchToSSH(host: string) {
    // This menu belongs to one pane, so the host goes in that pane.
    connectToSshHost(host, '~', paneId);
    isServerMenuOpen = false;
  }
</script>

<svelte:window
  on:click={() => {
    isServerMenuOpen = false;
    isContextMenuOpen = false;
  }}
  on:keydown={(e) => {
    if (paneId === $activePaneId && !isEditingPath) {
      if ((e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey && (e.key === 'l' || e.key === 'L')) {
        e.preventDefault();
        startEditing();
      } else if ((e.metaKey || e.ctrlKey) && e.shiftKey && (e.key === 'g' || e.key === 'G')) {
        e.preventDefault();
        pastePathAndGo();
      }
    }
  }}
/>

<div
  class="flex items-center gap-1.5 px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] text-xs text-[var(--text-secondary)] select-none relative z-20"
  class:pane-remote={pane.isSSH}
>
  <!-- History Controls -->
  <div class="flex items-center gap-0.5 mr-1 shrink-0">
    <button
      class="p-1 rounded hover:bg-[var(--bg-hover)] disabled:opacity-30 disabled:hover:bg-transparent"
      disabled={pane.historyIndex <= 0}
      on:click={() => goBack(paneId)}
      title="Bakåt"
    >
      <ArrowLeft size={13} />
    </button>

    <button
      class="p-1 rounded hover:bg-[var(--bg-hover)] disabled:opacity-30 disabled:hover:bg-transparent"
      disabled={pane.historyIndex >= pane.history.length - 1}
      on:click={() => goForward(paneId)}
      title="Framåt"
    >
      <ArrowRight size={13} />
    </button>

    <button
      class="p-1 rounded hover:bg-[var(--bg-hover)]"
      on:click={() => goUp(paneId)}
      title="Upp en nivå (⌘↑)"
    >
      <ArrowUp size={13} />
    </button>
  </div>

  <!-- Editable / Breadcrumb Path Bar -->
  {#if isEditingPath}
    <div class="flex-1 flex items-center gap-1 min-w-0">
      <input
        bind:this={pathInputEl}
        type="text"
        bind:value={editPathText}
        on:keydown={(e) => {
          if (e.key === 'Enter') commitPath();
          else if (e.key === 'Escape') cancelEditing();
        }}
        on:blur={commitPath}
        placeholder={pane.isSSH ? '/home/user/... eller ~' : '/Users/...'}
        class="flex-1 bg-[var(--bg-panel)] text-xs text-[var(--text-primary)] px-2 py-0.5 rounded border border-[var(--accent)] font-mono focus:outline-none shadow-inner"
      />
      <button
        class="p-1 rounded bg-[var(--bg-panel)] hover:bg-[var(--bg-hover)] text-[var(--text-secondary)] hover:text-white border border-[var(--border)]"
        on:mousedown|preventDefault
        on:click={pasteIntoInput}
        title="Klistra in från urklipp"
      >
        <Clipboard size={12} />
      </button>
      <button
        class="p-1 rounded bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)]"
        on:mousedown|preventDefault
        on:click={commitPath}
        title="Gå till sökväg (Enter)"
      >
        <Check size={12} />
      </button>
    </div>
  {:else}
    <!-- SSH Server Switcher / Badge (Only when remote SSH) -->
    {#if pane.isSSH}
      <div class="relative shrink-0">
        <button
          class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-green-950/60 text-green-400 font-semibold border border-green-800/60 hover:bg-green-900/60 transition-colors text-[11px]"
          on:click={(e) => {
            e.stopPropagation();
            loadSavedServers();
            isServerMenuOpen = !isServerMenuOpen;
          }}
          title="Ansluten till {pane.sshHost} (klicka för att växla)"
        >
          <Server size={11} />
          <span class="max-w-[100px] truncate">{pane.sshHost.split('.')[0]}</span>
        </button>

        {#if isServerMenuOpen}
          <div
            class="absolute top-full left-0 mt-1 w-56 py-1 bg-[var(--bg-surface)] border border-[var(--border)] rounded-md shadow-2xl z-50 text-xs text-[var(--text-primary)]"
            on:click={(e) => e.stopPropagation()}
            role="menu"
            tabindex="-1"
          >
            <div class="px-2 py-1 text-[10px] font-semibold text-[var(--text-muted)] uppercase tracking-wider border-b border-[var(--border)]">
              Växla anslutning
            </div>

            <button
              class="w-full flex items-center justify-between px-2.5 py-1.5 hover:bg-[var(--bg-hover)] text-left"
              on:click={switchToLocal}
            >
              <div class="flex items-center gap-2">
                <Laptop size={13} class="text-blue-400" />
                <span>Lokal disk (Macintosh)</span>
              </div>
            </button>

            <div class="my-1 border-t border-[var(--border)]"></div>
            <div class="px-2 py-0.5 text-[9.5px] text-[var(--text-muted)]">SSH-servrar:</div>

            {#each savedServers as srv}
              {@const isCur = pane.isSSH && pane.sshHost === srv.host}
              <button
                class="w-full flex items-center justify-between px-2.5 py-1.5 hover:bg-[var(--bg-hover)] text-left {isCur ? 'text-green-400 font-semibold bg-green-950/30' : 'text-slate-300'}"
                on:click={() => switchToSSH(srv.host)}
              >
                <div class="flex items-center gap-2 truncate">
                  <Server size={13} class="text-green-400 shrink-0" />
                  <span class="truncate">{srv.name}</span>
                </div>
                {#if isCur}
                  <Check size={12} class="shrink-0" />
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Clickable POSIX Path Bar (/data/analysis/folder) -->
    <div
      class="flex-1 flex items-center min-w-0 font-mono text-[11.5px] bg-[var(--bg-panel)]/80 hover:bg-[var(--bg-panel)] border border-[var(--border)] rounded px-2 py-1 transition-colors cursor-text overflow-x-auto whitespace-nowrap scrollbar-none shadow-inner"
      on:click={(e) => {
        const target = e.target as HTMLElement;
        if (target === e.currentTarget || target.classList.contains('path-divider')) {
          startEditing();
        }
      }}
      on:contextmenu={handleContextMenu}
      title="Klicka för att redigera eller klistra in sökväg (⌘L)"
      role="textbox"
      tabindex="-1"
    >
      <!-- Root slash or tilde -->
      <button
        class="px-1 py-0.2 rounded hover:bg-[var(--bg-hover)] text-slate-400 hover:text-white transition-colors font-mono font-bold shrink-0"
        on:click={(e) => {
          e.stopPropagation();
          navigatePane(paneId, pane.isSSH ? '~' : '/');
        }}
        title="Gå till {pane.isSSH ? '~' : '/'}"
      >
        {pane.isSSH && pane.currentPath.startsWith('~') ? '~' : '/'}
      </button>

      {#each pathSegments as segment, index}
        <span class="path-divider text-slate-600 font-mono select-none px-0.5 shrink-0">/</span>

        {@const isLast = index === pathSegments.length - 1}
        <button
          class="px-1 py-0.2 rounded transition-colors shrink-0 {isLast ? 'font-bold text-white bg-[var(--accent)]/20 text-[var(--accent)]' : 'text-slate-300 hover:text-white hover:bg-[var(--bg-hover)]'}"
          on:click={(e) => {
            e.stopPropagation();
            navigateToSegment(index);
          }}
          title="Gå till {pathSegments.slice(0, index + 1).join('/')}"
        >
          {segment}
        </button>
      {/each}
    </div>
  {/if}

  <!-- Compact Right Utility Icons (Copy & Terminal) -->
  <div class="flex items-center gap-0.5 shrink-0 ml-1">
    <button
      class="p-1 rounded text-slate-400 hover:text-white hover:bg-[var(--bg-hover)] transition-colors {copiedPath ? 'text-green-400 bg-green-950/40' : ''}"
      on:click={copyCurrentPath}
      title={copiedPath ? 'Sökväg kopierad!' : 'Kopiera fullständig sökväg'}
    >
      {#if copiedPath}
        <Check size={12} class="text-green-400" />
      {:else}
        <Copy size={12} />
      {/if}
    </button>

    <button
      class="p-1 rounded text-slate-400 hover:text-white hover:bg-[var(--bg-hover)]"
      on:click={pastePathAndGo}
      title="Klistra in sökväg från urklipp och gå dit direkt (⌘⇧G)"
    >
      <Clipboard size={12} />
    </button>

    <button
      class="p-1 rounded text-slate-400 hover:text-white hover:bg-[var(--bg-hover)] transition-colors {$isTerminalOpen && $activePaneId === paneId ? 'text-amber-400 bg-amber-500/20' : ''}"
      on:click={() => {
        activePaneId.set(paneId);
        toggleTerminal();
      }}
      title="Öppna/Stäng Terminal här (⌘J)"
    >
      <TerminalIcon size={12} />
    </button>
  </div>
</div>

{#if isContextMenuOpen}
  <div
    class="fixed z-50 w-52 py-1 bg-[var(--bg-surface)] border border-[var(--border)] rounded-md shadow-2xl text-xs text-[var(--text-primary)] select-none backdrop-blur-md"
    style="top: {contextMenuY}px; left: {contextMenuX}px;"
    on:click|stopPropagation
    role="menu"
    tabindex="-1"
  >
    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left transition-colors"
      on:click={() => {
        isContextMenuOpen = false;
        pastePathAndGo();
      }}
    >
      <div class="flex items-center gap-2">
        <Clipboard size={13} class="text-emerald-400" />
        <span>Klistra in sökväg</span>
      </div>
      <kbd class="text-[9px] font-mono opacity-70">⌘⇧G</kbd>
    </button>

    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left transition-colors"
      on:click={() => {
        isContextMenuOpen = false;
        copyCurrentPath();
      }}
    >
      <div class="flex items-center gap-2">
        <Copy size={13} class="text-blue-400" />
        <span>Kopiera sökväg</span>
      </div>
    </button>

    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left transition-colors"
      on:click={() => {
        isContextMenuOpen = false;
        activePaneId.set(paneId);
        openTerminalAt(pane.currentPath);
      }}
    >
      <div class="flex items-center gap-2">
        <TerminalIcon size={13} class="text-amber-400" />
        <span>Öppna i Terminal</span>
      </div>
      <kbd class="text-[9px] font-mono opacity-70">⌘J</kbd>
    </button>

    <div class="h-px bg-[var(--border)] my-1"></div>

    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left transition-colors"
      on:click={() => {
        isContextMenuOpen = false;
        startEditing();
      }}
    >
      <div class="flex items-center gap-2">
        <Edit3 size={13} class="text-slate-400" />
        <span>Redigera sökväg</span>
      </div>
      <kbd class="text-[9px] font-mono opacity-70">⌘L</kbd>
    </button>
  </div>
{/if}
