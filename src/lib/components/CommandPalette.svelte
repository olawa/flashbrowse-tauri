<script lang="ts">
  import { onMount } from 'svelte';
  import { setTheme } from '../stores/theme';
  import { resetLayoutWidths } from '../stores/layoutStore';
  import { isDualInspector, isDualPane, showHiddenFiles, navigatePane, activePaneId, leftPane, rightPane } from '../stores/navigation';
  import { isTerminalOpen, toggleTerminal, toggleTerminalDock } from '../stores/terminal';
  import { isGenomicsHubOpen, addTracksToHub, isRsnapServerRunning } from '../stores/genomicsStore';
  import { getHomeDirectory, deepSearch, startRsnapServer, stopRsnapServer, launchRsnap, sendToIgv } from '../invoke';
  import type { SearchMatch } from '../types';
  import {
    Search,
    Sparkles,
    Palette,
    Monitor,
    Baby,
    Terminal,
    LayoutGrid,
    Eye,
    Server,
    Home,
    FolderGit2,
    Folder,
    FileText,
    FileCode,
    Dna,
    Archive,
    FileSpreadsheet,
    File,
    Loader2,
    Radio,
    Activity,
  } from 'lucide-svelte';

  export let isOpen = false;
  export let onClose: () => void;

  let query = '';
  let selectedIdx = 0;
  let inputEl: HTMLInputElement;
  let isSearchingFiles = false;
  let fileMatches: SearchMatch[] = [];
  let searchTimer: any = null;

  interface CommandItem {
    id: string;
    title: string;
    category: string;
    icon: any;
    action: () => void;
  }

  let commands: CommandItem[] = [];

  onMount(async () => {
    const home = await getHomeDirectory();
    commands = [
      { id: 'genomics-hub', title: 'Genomics: Öppna Genomics Track Hub', category: 'Genomics', icon: Sparkles, action: () => isGenomicsHubOpen.set(true) },
      { id: 'rsnap-viewer', title: 'rsnap: Öppna Desktop Viewer', category: 'Genomics', icon: Dna, action: () => launchRsnap([]) },
      { id: 'rsnap-server-start', title: 'rsnap: Starta rsnap Server (port 5555)', category: 'Genomics', icon: Server, action: async () => { await startRsnapServer(); isRsnapServerRunning.set(true); } },
      { id: 'rsnap-server-stop', title: 'rsnap: Stoppa rsnap Server', category: 'Genomics', icon: Server, action: async () => { await stopRsnapServer(); isRsnapServerRunning.set(false); } },
      { id: 'theme-swift-dark', title: 'Theme: 🍎 Swift macOS (Dark)', category: 'Utseende', icon: Sparkles, action: () => setTheme('swift-dark') },
      { id: 'theme-swift-light', title: 'Theme: 🍎 Swift macOS (Light)', category: 'Utseende', icon: Sparkles, action: () => setTheme('swift-light') },
      { id: 'theme-pro', title: 'Theme: Pro Dark (Midnight)', category: 'Utseende', icon: Sparkles, action: () => setTheme('pro-dark') },
      { id: 'theme-cyber', title: 'Theme: Cyberpunk Neon', category: 'Utseende', icon: Palette, action: () => setTheme('cyberpunk') },
      { id: 'theme-retro', title: 'Theme: Retro Mac 1995', category: 'Utseende', icon: Monitor, action: () => setTheme('retro-mac') },
      { id: 'theme-kids', title: 'Theme: 🎈 Barn-läge (Kids Mode)', category: 'Utseende', icon: Baby, action: () => setTheme('kids-mode') },
      { id: 'reset-layout', title: 'Layout: Återställ panelbredder till standard', category: 'Vy', icon: LayoutGrid, action: () => resetLayoutWidths() },
      { id: 'toggle-terminal', title: 'Toggle Terminal (Cmd+J)', category: 'Vy', icon: Terminal, action: () => toggleTerminal() },
      { id: 'toggle-dock', title: 'Toggle Terminal Dock Position', category: 'Vy', icon: Terminal, action: () => toggleTerminalDock() },
      { id: 'toggle-inspector', title: 'Toggle Dual Inspector', category: 'Vy', icon: LayoutGrid, action: () => isDualInspector.update((v) => !v) },
      { id: 'toggle-dotfiles', title: 'Toggle Hidden Files', category: 'Vy', icon: Eye, action: () => showHiddenFiles.update((v) => !v) },
      { id: 'jump-home', title: 'Gå till Hemkatalog (~)', category: 'Navigation', icon: Home, action: () => navigatePane($activePaneId, home) },
      { id: 'jump-projects', title: 'Gå till Utveckling (~/dev/projects)', category: 'Navigation', icon: FolderGit2, action: () => navigatePane($activePaneId, `${home}/dev/projects`) },
      { id: 'jump-marvin', title: 'Anslut till Marvin SSH', category: 'Fjärr', icon: Server, action: () => navigatePane('right', '~') },
    ];
  });

  $: filteredCommands = commands.filter((c) =>
    c.title.toLowerCase().includes(query.toLowerCase()) ||
    c.category.toLowerCase().includes(query.toLowerCase())
  );

  $: currentActivePath = (() => {
    const pane = $activePaneId === 'left' ? $leftPane : $rightPane;
    return pane.currentPath || '';
  })();

  // Debounced deep search when query changes
  $: {
    if (query.trim().length >= 2 && currentActivePath && !currentActivePath.startsWith('ssh:')) {
      clearTimeout(searchTimer);
      isSearchingFiles = true;
      searchTimer = setTimeout(async () => {
        try {
          fileMatches = await deepSearch(currentActivePath, query.trim(), 40);
        } catch (err) {
          console.warn('Deep search error:', err);
          fileMatches = [];
        } finally {
          isSearchingFiles = false;
        }
      }, 120);
    } else {
      fileMatches = [];
      isSearchingFiles = false;
    }
  }

  type CombinedItem =
    | { type: 'cmd'; item: CommandItem }
    | { type: 'file'; item: SearchMatch };

  $: allItems = [
    ...filteredCommands.map((c) => ({ type: 'cmd' as const, item: c })),
    ...fileMatches.map((f) => ({ type: 'file' as const, item: f })),
  ];

  $: if (isOpen) {
    query = '';
    selectedIdx = 0;
    fileMatches = [];
    setTimeout(() => inputEl?.focus(), 50);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIdx = (selectedIdx + 1) % (allItems.length || 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIdx = (selectedIdx - 1 + allItems.length) % (allItems.length || 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const current = allItems[selectedIdx];
      if (current) {
        if (current.type === 'cmd') {
          current.item.action();
        } else {
          handleFileSelect(current.item);
        }
        onClose();
      }
    }
  }

  function handleFileSelect(file: SearchMatch) {
    if (file.is_dir) {
      navigatePane($activePaneId, file.path);
    } else {
      const parentDir = file.path.substring(0, file.path.lastIndexOf('/')) || '/';
      navigatePane($activePaneId, parentDir).then(() => {
        const store = $activePaneId === 'left' ? leftPane : rightPane;
        store.update((s) => ({ ...s, selectedPaths: new Set([file.path]) }));
      });
    }
  }

  function getFileIcon(name: string, is_dir: boolean) {
    if (is_dir) return Folder;
    const lower = name.toLowerCase();
    if (lower.endsWith('.bam') || lower.endsWith('.cram') || lower.endsWith('.vcf.gz') || lower.endsWith('.vcf') || lower.endsWith('.fastq.gz')) return Dna;
    if (lower.endsWith('.tsv') || lower.endsWith('.csv') || lower.endsWith('.xlsx')) return FileSpreadsheet;
    if (lower.endsWith('.rs') || lower.endsWith('.py') || lower.endsWith('.ts') || lower.endsWith('.js') || lower.endsWith('.sh')) return FileCode;
    if (lower.endsWith('.zip') || lower.endsWith('.tar.gz') || lower.endsWith('.gz')) return Archive;
    return FileText;
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-black/70 backdrop-blur-sm select-none"
    on:click={onClose}
    on:keydown={handleKeyDown}
  >
    <div
      class="w-[580px] max-h-[500px] flex flex-col bg-[var(--bg-surface)] border border-[var(--border)] rounded-2xl shadow-2xl overflow-hidden"
      on:click|stopPropagation
    >
      <!-- Search Input -->
      <div class="flex items-center gap-2.5 px-4 py-3 border-b border-[var(--border)] bg-[var(--bg-panel)]">
        {#if isSearchingFiles}
          <Loader2 size={16} class="text-[var(--accent)] animate-spin" />
        {:else}
          <Search size={16} class="text-[var(--accent)]" />
        {/if}
        <input
          bind:this={inputEl}
          type="text"
          bind:value={query}
          placeholder="Sök kommando eller fil i undermappar (t.ex. *.bam, theme, config)..."
          class="flex-1 bg-transparent text-sm text-[var(--text-primary)] focus:outline-none placeholder:text-[var(--text-muted)]"
        />
        <kbd class="px-1.5 py-0.5 rounded bg-[var(--border)] text-[10px] text-[var(--text-muted)] font-mono">ESC</kbd>
      </div>

      <!-- Combined List -->
      <div class="flex-1 overflow-y-auto p-2 space-y-1 divide-y divide-[var(--border)]/30">
        <!-- Commands Section -->
        {#if filteredCommands.length > 0}
          <div class="pt-1 pb-1">
            {#if query.trim().length >= 2}
              <div class="px-3 py-1 text-[10px] font-bold uppercase tracking-wider text-[var(--text-muted)] font-mono">
                Kommandon
              </div>
            {/if}
            {#each filteredCommands as cmd, i}
              {@const globalIndex = i}
              <button
                class="w-full flex items-center justify-between px-3 py-2 rounded-lg text-left transition-colors {globalIndex === selectedIdx ? 'bg-[var(--accent)] text-white' : 'hover:bg-[var(--bg-hover)] text-[var(--text-primary)]'}"
                on:click={() => { cmd.action(); onClose(); }}
              >
                <div class="flex items-center gap-2.5 min-w-0">
                  <svelte:component this={cmd.icon} size={15} class="shrink-0" />
                  <span class="text-xs font-medium truncate">{cmd.title}</span>
                </div>
                <span class="text-[10px] opacity-60 uppercase font-mono shrink-0 ml-2">{cmd.category}</span>
              </button>
            {/each}
          </div>
        {/if}

        <!-- Deep Files Section -->
        {#if fileMatches.length > 0}
          <div class="pt-2 pb-1">
            <div class="px-3 py-1 text-[10px] font-bold uppercase tracking-wider text-cyan-400 font-mono flex items-center justify-between">
              <span>Filer i undermappar ({fileMatches.length})</span>
              <span class="text-[9px] opacity-70 lowercase">Spotlight / Fast Index</span>
            </div>
            {#each fileMatches as file, i}
              {@const globalIndex = filteredCommands.length + i}
              <button
                class="w-full flex items-center justify-between px-3 py-1.5 rounded-lg text-left transition-colors {globalIndex === selectedIdx ? 'bg-[var(--accent)] text-white' : 'hover:bg-[var(--bg-hover)] text-[var(--text-primary)]'}"
                on:click={() => { handleFileSelect(file); onClose(); }}
              >
                <div class="flex items-center gap-2.5 min-w-0 flex-1">
                  <svelte:component this={getFileIcon(file.name, file.is_dir)} size={14} class="shrink-0 text-cyan-400" />
                  <div class="flex flex-col min-w-0 flex-1">
                    <span class="text-xs font-medium truncate {file.is_dir ? 'font-semibold' : ''}">{file.name}</span>
                    <span class="text-[10px] font-mono opacity-60 truncate">{file.relative_path}</span>
                  </div>
                </div>
                <span class="text-[10px] font-mono opacity-75 shrink-0 ml-2">{file.formatted_size}</span>
              </button>
            {/each}
          </div>
        {/if}

        {#if allItems.length === 0 && !isSearchingFiles}
          <div class="p-8 text-center text-xs text-[var(--text-muted)]">
            Inga matchande kommandon eller filer hittades
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

