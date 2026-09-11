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
  } from 'lucide-svelte';

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

  async function copyCurrentPath() {
    try {
      await navigator.clipboard.writeText(pane.currentPath);
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
    const target = '/' + pathSegments.slice(0, index + 1).join('/');
    navigatePane(paneId, target);
  }

  function switchToLocal() {
    const store = paneId === 'left' ? leftPane : rightPane;
    store.update((s) => ({ ...s, isSSH: false }));
    isServerMenuOpen = false;
    navigatePane(paneId, '/');
  }

  function switchToSSH(host: string) {
    const store = paneId === 'left' ? leftPane : rightPane;
    store.update((s) => ({ ...s, isSSH: true, sshHost: host }));
    isServerMenuOpen = false;
    navigatePane(paneId, '~');
  }
</script>

<svelte:window
  on:click={() => {
    isServerMenuOpen = false;
    isContextMenuOpen = false;
  }}
  on:keydown={(e) => {
    if (paneId === $activePaneId && !isEditingPath) {
      if ((e.metaKey || e.ctrlKey) && (e.key === 'l' || e.key === 'L')) {
        e.preventDefault();
        startEditing();
      } else if ((e.metaKey || e.ctrlKey) && e.shiftKey && (e.key === 'g' || e.key === 'G')) {
        e.preventDefault();
        pastePathAndGo();
      }
    }
  }}
/>

<div class="flex items-center gap-1.5 px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] text-xs text-[var(--text-secondary)] select-none relative z-20">
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
    <div
      class="flex items-center gap-1 overflow-x-auto whitespace-nowrap flex-1 py-0.5 cursor-pointer rounded px-1 hover:bg-[var(--bg-hover)]/40 transition-colors min-w-0"
      on:click={(e) => {
        const target = e.target as HTMLElement;
        if (target === e.currentTarget || target.tagName === 'DIV' || target.classList.contains('breadcrumb-space')) {
          startEditing();
        }
      }}
      on:contextmenu={handleContextMenu}
      title="Klicka för att skriva eller klistra in sökväg (Högerklicka för meny)"
      role="button"
      tabindex="-1"
    >
      <!-- Host / Connection Badge with Switcher -->
      <div class="relative shrink-0">
        {#if pane.isSSH}
          <button
            class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-green-950/60 text-green-400 font-semibold border border-green-800/60 hover:bg-green-900/60 transition-colors"
            on:click={(e) => {
              e.stopPropagation();
              loadSavedServers();
              isServerMenuOpen = !isServerMenuOpen;
            }}
            title="Klicka för att växla mellan servrar eller lokal disk"
          >
            <Server size={12} />
            <span class="max-w-[140px] truncate">{pane.sshHost}</span>
          </button>
        {:else}
          <button
            class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-[#1e2330] hover:bg-[#252c3d] text-slate-300 font-medium border border-[#2d354a] transition-colors"
            on:click={(e) => {
              e.stopPropagation();
              loadSavedServers();
              isServerMenuOpen = !isServerMenuOpen;
            }}
            title="Klicka för att ansluta panelen till en SSH-server"
          >
            <Laptop size={12} class="text-blue-400" />
            <span>Lokal (Mac)</span>
          </button>
        {/if}

        <!-- Server Switcher Dropdown Menu -->
        {#if isServerMenuOpen}
          <div
            class="absolute top-full left-0 mt-1 w-56 py-1 bg-[var(--bg-surface)] border border-[var(--border)] rounded-md shadow-2xl z-50 text-xs text-[var(--text-primary)]"
            on:click={(e) => e.stopPropagation()}
            role="menu"
            tabindex="-1"
          >
            <div class="px-2 py-1 text-[10px] font-semibold text-[var(--text-muted)] uppercase tracking-wider border-b border-[var(--border)]">
              Växla anslutning i denna panel
            </div>

            <button
              class="w-full flex items-center justify-between px-2.5 py-1.5 hover:bg-[var(--bg-hover)] text-left {!pane.isSSH ? 'text-[var(--accent)] font-semibold bg-[var(--accent-subtle)]' : ''}"
              on:click={switchToLocal}
            >
              <div class="flex items-center gap-2">
                <Laptop size={13} class="text-blue-400" />
                <span>Lokal disk (Macintosh)</span>
              </div>
              {#if !pane.isSSH}
                <Check size={12} />
              {/if}
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

      <ChevronRight size={12} class="opacity-40 shrink-0" />

      <!-- Root button -->
      <button
        class="flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] text-[var(--text-primary)] shrink-0"
        on:click={(e) => {
          e.stopPropagation();
          navigatePane(paneId, pane.isSSH ? '~' : '/');
        }}
      >
        <HardDrive size={11} />
        <span>{pane.isSSH ? '~' : 'Root'}</span>
      </button>

      {#if pathSegments.length > 0}
        <ChevronRight size={12} class="opacity-40 shrink-0" />
      {/if}

      <!-- Dynamic Path Segments -->
      {#each pathSegments as segment, index}
        <button
          class="px-1.5 py-0.5 rounded hover:bg-[var(--bg-hover)] transition-colors shrink-0 {index === pathSegments.length - 1 ? 'font-semibold text-[var(--accent)]' : 'text-[var(--text-primary)]'}"
          on:click={(e) => {
            e.stopPropagation();
            navigateToSegment(index);
          }}
        >
          {segment}
        </button>
        {#if index < pathSegments.length - 1}
          <ChevronRight size={12} class="opacity-40 shrink-0" />
        {/if}
      {/each}

      <!-- Edit icon button on right edge of breadcrumb -->
      <button
        class="ml-1 p-0.5 rounded text-slate-400 hover:text-white hover:bg-[var(--bg-hover)] shrink-0 opacity-70 hover:opacity-100 transition-opacity"
        on:click={(e) => {
          e.stopPropagation();
          startEditing();
        }}
        title="Redigera sökväg direkt (eller klicka på listen)"
      >
        <Edit3 size={11} />
      </button>
    </div>
  {/if}

  <!-- Dual Inspector toggle & Action -->
  <div class="flex items-center gap-1.5 ml-auto shrink-0">
    <button
      class="p-1 rounded text-slate-400 hover:text-white hover:bg-[var(--bg-hover)]"
      on:click={pastePathAndGo}
      title="Klistra in sökväg från urklipp och gå dit direkt (⌘⇧G)"
    >
      <Clipboard size={12} />
    </button>

    <button
      class="p-1 rounded text-slate-400 hover:text-white hover:bg-[var(--bg-hover)]"
      on:click={() => startEditing()}
      title="Redigera sökväg manuellt (⌘L)"
    >
      <Edit3 size={12} />
    </button>

    <button
      class="px-2 py-0.5 rounded border border-[var(--border)] text-[11px] hover:bg-[var(--bg-hover)] flex items-center gap-1 {$isDualInspector ? 'bg-[var(--accent-subtle)] text-[var(--accent)] border-[var(--accent)]' : ''}"
      on:click={() => isDualInspector.update((v) => !v)}
      title="Växla Dual Inspector"
    >
      <LayoutGrid size={11} />
      <span>Dual Inspector</span>
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
