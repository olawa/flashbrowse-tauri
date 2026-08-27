<script lang="ts">
  import { onMount } from 'svelte';
  import { setTheme } from '../stores/theme';
  import { isDualInspector, isDualPane, showHiddenFiles, navigatePane, activePaneId } from '../stores/navigation';
  import { isTerminalOpen, toggleTerminal, toggleTerminalDock } from '../stores/terminal';
  import { getHomeDirectory } from '../invoke';
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
  } from 'lucide-svelte';

  export let isOpen = false;
  export let onClose: () => void;

  let query = '';
  let selectedIdx = 0;
  let inputEl: HTMLInputElement;

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
      { id: 'theme-pro', title: 'Theme: Pro Dark (Default)', category: 'Appearance', icon: Sparkles, action: () => setTheme('pro-dark') },
      { id: 'theme-cyber', title: 'Theme: Cyberpunk Neon', category: 'Appearance', icon: Palette, action: () => setTheme('cyberpunk') },
      { id: 'theme-retro', title: 'Theme: Retro Mac 1995', category: 'Appearance', icon: Monitor, action: () => setTheme('retro-mac') },
      { id: 'theme-kids', title: 'Theme: 🎈 Barn-läge (Kids Mode)', category: 'Appearance', icon: Baby, action: () => setTheme('kids-mode') },
      { id: 'toggle-terminal', title: 'Toggle Terminal (Cmd+J)', category: 'View', icon: Terminal, action: () => toggleTerminal() },
      { id: 'toggle-dock', title: 'Toggle Terminal Dock Position', category: 'View', icon: Terminal, action: () => toggleTerminalDock() },
      { id: 'toggle-inspector', title: 'Toggle Dual Inspector', category: 'View', icon: LayoutGrid, action: () => isDualInspector.update((v) => !v) },
      { id: 'toggle-dotfiles', title: 'Toggle Hidden Files', category: 'View', icon: Eye, action: () => showHiddenFiles.update((v) => !v) },
      { id: 'jump-home', title: 'Go to Home Directory', category: 'Navigation', icon: Home, action: () => navigatePane('left', home) },
      { id: 'jump-projects', title: 'Go to Projects (~/dev/projects)', category: 'Navigation', icon: FolderGit2, action: () => navigatePane('left', `${home}/dev/projects`) },
      { id: 'jump-marvin', title: 'Connect to Marvin SSH', category: 'Remote', icon: Server, action: () => navigatePane('right', '~') },
    ];
  });

  $: filtered = commands.filter((c) =>
    c.title.toLowerCase().includes(query.toLowerCase()) ||
    c.category.toLowerCase().includes(query.toLowerCase())
  );

  $: if (isOpen) {
    query = '';
    selectedIdx = 0;
    setTimeout(() => inputEl?.focus(), 50);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIdx = (selectedIdx + 1) % (filtered.length || 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIdx = (selectedIdx - 1 + filtered.length) % (filtered.length || 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filtered[selectedIdx]) {
        filtered[selectedIdx].action();
        onClose();
      }
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center pt-24 bg-black/60 backdrop-blur-sm select-none"
    on:click={onClose}
    on:keydown={handleKeyDown}
  >
    <div
      class="w-[520px] max-h-[420px] flex flex-col bg-[var(--bg-surface)] border border-[var(--border)] rounded-xl shadow-2xl overflow-hidden"
      on:click|stopPropagation
    >
      <!-- Search Input -->
      <div class="flex items-center gap-2 px-4 py-3 border-b border-[var(--border)] bg-[var(--bg-panel)]">
        <Search size={16} class="text-[var(--accent)]" />
        <input
          bind:this={inputEl}
          type="text"
          bind:value={query}
          placeholder="Type a command or search..."
          class="flex-1 bg-transparent text-sm text-[var(--text-primary)] focus:outline-none"
        />
        <kbd class="px-1.5 py-0.5 rounded bg-[var(--border)] text-[10px] text-[var(--text-muted)] font-mono">ESC</kbd>
      </div>

      <!-- Command List -->
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        {#each filtered as cmd, index}
          <button
            class="w-full flex items-center justify-between px-3 py-2 rounded-lg text-left transition-colors {index === selectedIdx ? 'bg-[var(--accent)] text-white' : 'hover:bg-[var(--bg-hover)] text-[var(--text-primary)]'}"
            on:click={() => { cmd.action(); onClose(); }}
          >
            <div class="flex items-center gap-2.5">
              <svelte:component this={cmd.icon} size={15} />
              <span class="text-xs font-medium">{cmd.title}</span>
            </div>
            <span class="text-[10px] opacity-60 uppercase font-mono">{cmd.category}</span>
          </button>
        {/each}

        {#if filtered.length === 0}
          <div class="p-6 text-center text-xs text-[var(--text-muted)]">
            No matching commands found
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
