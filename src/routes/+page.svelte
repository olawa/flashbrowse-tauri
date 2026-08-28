<script lang="ts">
  import { onMount } from 'svelte';
  import {
    initNavigation,
    leftPane,
    rightPane,
    isDualPane,
    isDualInspector,
    isInspectorDetached,
    activePaneId,
    activeHoveredItem,
  } from '$lib/stores/navigation';
  import {
    isTerminalOpen,
    terminalDockPosition,
    toggleTerminal,
    toggleTerminalDock,
  } from '$lib/stores/terminal';
  import { currentTheme, isKidsMode, setTheme } from '$lib/stores/theme';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import FileTable from '$lib/components/FileTable.svelte';
  import Inspector from '$lib/components/Inspector.svelte';
  import DetachedInspectorView from '$lib/components/DetachedInspectorView.svelte';
  import Terminal from '$lib/components/Terminal.svelte';
  import StashShelf from '$lib/components/StashShelf.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import { toggleStash } from '$lib/stores/stash';
  import type { FileItem } from '$lib/types';
  import {
    Lock,
    Unlock,
    PanelRightClose,
    Folder,
    FileText,
    Dna,
    Table,
    Copy,
    Check,
  } from 'lucide-svelte';

  let leftPreviewItem: FileItem | null = null;
  let rightPreviewItem: FileItem | null = null;
  let isPaletteOpen = false;
  let isDetachedWindowMode = false;

  // Kids Mode Pin Lock
  let isPinModalOpen = false;
  let pinInput = '';
  let pinError = '';

  onMount(async () => {
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      if (params.get('window') === 'inspector') {
        isDetachedWindowMode = true;
        return;
      }
    }

    await initNavigation();
    setTheme('pro-dark');

    // Default select first items for instant preview
    if ($leftPane.items.length > 0) {
      leftPreviewItem = $leftPane.items[0];
    }
    if ($rightPane.items.length > 0) {
      rightPreviewItem = $rightPane.items[0];
    }
  });

  $: if ($leftPane.items.length && !leftPreviewItem) {
    leftPreviewItem = $leftPane.items[0];
  }
  $: if ($rightPane.items.length && !rightPreviewItem) {
    rightPreviewItem = $rightPane.items[0];
  }

  function handleGlobalKeyDown(e: KeyboardEvent) {
    // Cmd+K / Ctrl+K: Command Palette
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isPaletteOpen = !isPaletteOpen;
    }
    // Cmd+J / Ctrl+J: Toggle Terminal
    else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'j') {
      if (e.altKey) {
        e.preventDefault();
        toggleTerminalDock();
      } else {
        e.preventDefault();
        toggleTerminal();
      }
    }
    // Cmd+Option+D: Toggle Dual Inspector
    else if ((e.metaKey || e.ctrlKey) && e.altKey && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      isDualInspector.update((v) => !v);
    }
    // Cmd+Option+S: Toggle Stash Shelf
    else if ((e.metaKey || e.ctrlKey) && e.altKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      toggleStash();
    }
  }

  function unlockKidsMode() {
    if (pinInput === '1234') {
      setTheme('pro-dark');
      isPinModalOpen = false;
      pinInput = '';
      pinError = '';
    } else {
      pinError = 'Wrong PIN (Default is 1234)';
    }
  }
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

{#if isDetachedWindowMode}
  <!-- Standalone Detached Inspector Window View -->
  <DetachedInspectorView />
{:else}
  <!-- Main Workstation Window -->
  <div class="flex h-screen w-screen bg-[var(--bg-base)] text-[var(--text-primary)] overflow-hidden font-sans select-none">
    <!-- 1. Left Sidebar -->
    <Sidebar />

    <!-- 2. Main Workstation Area -->
    <div class="flex-1 flex flex-col min-w-0 h-full overflow-hidden">
      <!-- Top Bar for Kids Mode or Detached status -->
      {#if $isKidsMode}
        <div class="flex items-center justify-between px-4 py-2 bg-pink-100 border-b border-pink-200 text-pink-900 text-xs font-semibold">
          <div class="flex items-center gap-2">
            <span>🎈 Barn-läge Aktivt (Skrivskyddat & Sandlåda)</span>
          </div>
          <button
            class="flex items-center gap-1 px-3 py-1 rounded-full bg-pink-600 hover:bg-pink-700 text-white shadow-sm transition-transform active:scale-95"
            on:click={() => (isPinModalOpen = true)}
          >
            <Lock size={12} />
            <span>Lås upp föräldraläge</span>
          </button>
        </div>
      {:else if $isInspectorDetached}
        <div class="flex items-center justify-between px-4 py-1.5 bg-[#14171d] border-b border-[#262d3d] text-xs text-slate-300">
          <div class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>Inspektor är löskopplad till eget fönster</span>
          </div>
          <button
            class="flex items-center gap-1 px-2.5 py-0.5 rounded bg-[#e85422]/20 hover:bg-[#e85422] text-[#e85422] hover:text-white border border-[#e85422]/40 text-[11px] font-medium transition-colors"
            on:click={() => isInspectorDetached.set(false)}
          >
            <PanelRightClose size={11} />
            <span>Fäst tillbaka i huvudfönstret</span>
          </button>
        </div>
      {/if}
      <!-- Global Top Active / Hover Path & Filename Bar -->
      <div class="px-3 py-1.5 bg-[var(--bg-surface)] border-b border-[var(--border)] flex items-center justify-between gap-3 text-xs select-text overflow-hidden shrink-0">
        {#if $activeHoveredItem || ($activePaneId === 'left' ? leftPreviewItem : rightPreviewItem)}
          {@const active = $activeHoveredItem || ($activePaneId === 'left' ? leftPreviewItem : rightPreviewItem)}
          {#if active}
            <div class="flex items-center gap-2 min-w-0 flex-1 overflow-hidden">
              <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 {active.is_dir ? 'bg-amber-500/20 text-amber-400' : 'bg-blue-500/20 text-blue-400'}">
                {#if active.is_dir}
                  <Folder size={12} />
                {:else if ['bam', 'cram', 'sam'].includes(active.extension.toLowerCase()) || active.name.endsWith('.bam')}
                  <Dna size={12} class="text-emerald-400" />
                {:else if ['vcf', 'bcf'].includes(active.extension.toLowerCase()) || active.name.endsWith('.vcf.gz')}
                  <Dna size={12} class="text-purple-400" />
                {:else if ['tsv', 'csv', 'tab', 'xlsx'].includes(active.extension.toLowerCase())}
                  <Table size={12} class="text-blue-400" />
                {:else}
                  <FileText size={12} />
                {/if}
              </div>

              <!-- Full filename in bold -->
              <span class="font-bold text-xs text-white select-text font-mono truncate hover:overflow-visible hover:whitespace-normal" title={active.name}>
                {active.name}
              </span>

              <!-- Type / Extension badge -->
              <span class="px-1.5 py-0.2 rounded bg-[#191d26] text-slate-300 text-[10px] font-mono border border-[#262d3d] shrink-0">
                {active.is_dir ? 'MAPP' : active.extension.toUpperCase() || 'FIL'}
              </span>

              <!-- Size & Modified -->
              <span class="text-[11px] text-slate-400 font-mono shrink-0">
                {active.is_dir ? '' : active.formatted_size} • {active.formatted_modified}
              </span>

              <!-- Full path in subtle font with copy button -->
              <span class="text-[10.5px] text-slate-500 font-mono truncate hidden md:inline select-text" title={active.path}>
                {active.path}
              </span>
            </div>

            <button
              class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#191d26] hover:bg-[#222836] border border-[#262d3d] text-[10.5px] text-slate-300 hover:text-white shrink-0 font-mono transition-colors"
              on:click={() => navigator.clipboard.writeText(active.path)}
              title="Kopiera fullständig sökväg"
            >
              <Copy size={10} />
              <span>Kopiera sökväg</span>
            </button>
          {/if}
        {:else}
          <div class="text-[11px] text-slate-500 font-mono">
            Hovra eller markera en fil för att visa fullständigt namn och sökväg
          </div>
        {/if}
      </div>

      <!-- Workstation Columns Container -->
      <div class="flex-1 flex min-h-0 overflow-hidden">
        <!-- LEFT WORKSTATION (Pane + Inspector) -->
        <div class="flex-1 flex min-w-0 h-full">
          <!-- Left File Browser -->
          <div class="flex-1 flex flex-col min-w-[280px] h-full border-r border-[var(--border)]">
            <Breadcrumb paneId="left" />
            <FileTable paneId="left" onSelectPreview={(item) => (leftPreviewItem = item)} />
          </div>

          <!-- Left Inspector -->
          {#if $isDualInspector || !$isDualPane}
            <Inspector item={leftPreviewItem} titlePrefix="Local" />
          {/if}
        </div>

        <!-- RIGHT WORKSTATION (Pane + Inspector) -->
        {#if $isDualPane}
          <div class="flex-1 flex min-w-0 h-full border-l border-[var(--border)]">
            <!-- Right File Browser -->
            <div class="flex-1 flex flex-col min-w-[280px] h-full border-r border-[var(--border)]">
              <Breadcrumb paneId="right" />
              <FileTable paneId="right" onSelectPreview={(item) => (rightPreviewItem = item)} />
            </div>

            <!-- Right Inspector -->
            {#if $isDualInspector}
              <Inspector
                item={rightPreviewItem}
                titlePrefix={$rightPane.isSSH ? `Remote (${$rightPane.sshHost.split('.')[0]})` : 'Right'}
              />
            {/if}
          </div>
        {/if}

        <!-- Vertical Full-Height Side Terminal Column -->
        {#if $isTerminalOpen && $terminalDockPosition === 'side'}
          <Terminal />
        {/if}
      </div>

      <!-- Stash Shelf Drawer (Staging) -->
      <StashShelf />

      <!-- Horizontal Bottom Terminal Drawer -->
      {#if $isTerminalOpen && $terminalDockPosition === 'bottom'}
        <Terminal />
      {/if}
    </div>
  </div>
{/if}

<!-- Command Palette Modal (Cmd+K) -->
<CommandPalette isOpen={isPaletteOpen} onClose={() => (isPaletteOpen = false)} />

<!-- Kids Mode PIN Unlock Modal -->
{#if isPinModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
    <div class="w-80 p-6 bg-[var(--bg-surface)] border border-[var(--border)] rounded-2xl shadow-2xl text-center space-y-4">
      <div class="w-12 h-12 rounded-full bg-pink-500/20 text-pink-500 flex items-center justify-center mx-auto">
        <Unlock size={24} />
      </div>
      <div>
        <h3 class="font-bold text-base text-[var(--text-primary)]">Lås upp föräldraläge</h3>
        <p class="text-xs text-[var(--text-secondary)] mt-1">Ange PIN-kod för att avsluta barn-läget (Standard: 1234)</p>
      </div>

      <input
        type="password"
        bind:value={pinInput}
        placeholder="PIN-kod"
        maxlength={6}
        class="w-full text-center text-xl tracking-widest font-mono p-2 rounded-lg bg-[var(--bg-panel)] border border-[var(--border)] text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
        on:keydown={(e) => e.key === 'Enter' && unlockKidsMode()}
      />

      {#if pinError}
        <span class="text-xs text-red-400 block">{pinError}</span>
      {/if}

      <div class="flex gap-2">
        <button
          class="flex-1 py-2 rounded-lg border border-[var(--border)] hover:bg-[var(--bg-hover)] text-xs"
          on:click={() => { isPinModalOpen = false; pinInput = ''; pinError = ''; }}
        >
          Avbryt
        </button>
        <button
          class="flex-1 py-2 rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-xs font-semibold"
          on:click={unlockKidsMode}
        >
          Lås upp
        </button>
      </div>
    </div>
  </div>
{/if}
