<script lang="ts">
  import { onMount } from 'svelte';
  import { getHomeDirectory } from '../invoke';
  import { navigatePane, activePaneId, showHiddenFiles, leftPane, rightPane } from '../stores/navigation';
  import { currentTheme, setTheme, isKidsMode } from '../stores/theme';
  import DiskBar from './DiskBar.svelte';
  import {
    Home,
    FolderGit2,
    Download,
    FileText,
    Monitor,
    Server,
    Palette,
    Eye,
    EyeOff,
    Sparkles,
    Baby,
    Lock,
  } from 'lucide-svelte';
  import type { ThemeName } from '../types';

  let homeDir = '';
  let devProjectsDir = '';

  onMount(async () => {
    try {
      homeDir = await getHomeDirectory();
      devProjectsDir = `${homeDir}/dev/projects`;
    } catch {}
  });

  function jumpTo(path: string, isSSH = false, host = '') {
    const paneId = $activePaneId;
    if (isSSH) {
      const store = paneId === 'left' ? leftPane : rightPane;
      store.update((s) => ({ ...s, isSSH: true, sshHost: host }));
    } else {
      const store = paneId === 'left' ? leftPane : rightPane;
      store.update((s) => ({ ...s, isSSH: false }));
    }
    navigatePane(paneId, path);
  }

  const themes: Array<{ id: ThemeName; label: string; icon: any }> = [
    { id: 'pro-dark', label: 'Pro Dark (Default)', icon: Sparkles },
    { id: 'cyberpunk', label: 'Cyberpunk Neon', icon: Palette },
    { id: 'retro-mac', label: 'Retro Mac 1995', icon: Monitor },
    { id: 'kids-mode', label: '🎈 Barn-läge (Kids)', icon: Baby },
  ];
</script>

<aside class="w-52 h-full flex flex-col border-r border-[var(--border)] bg-[var(--bg-surface)] text-xs select-none">
  <!-- Header / App Logo -->
  <div class="px-3 py-3 border-b border-[var(--border)] flex items-center gap-2">
    <div class="w-6 h-6 rounded bg-[var(--accent)] flex items-center justify-center font-black text-white text-xs shadow-sm">
      ⚡
    </div>
    <div class="flex flex-col">
      <span class="font-bold text-[var(--text-primary)] text-sm leading-tight">Flashbrowse</span>
      <span class="text-[10px] text-[var(--text-secondary)] font-mono">v0.1 Tauri Edition</span>
    </div>
  </div>

  <!-- Navigation Lists -->
  <div class="flex-1 overflow-y-auto p-2 space-y-4">
    <!-- Favorites -->
    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Favorites</span>
      <div class="mt-1 space-y-0.5">
        {#if homeDir}
          <button
            class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-primary)]"
            on:click={() => jumpTo(homeDir)}
          >
            <Home size={14} class="text-[var(--accent)]" />
            <span>Home</span>
          </button>

          <button
            class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-primary)]"
            on:click={() => jumpTo(`${homeDir}/Desktop`)}
          >
            <Monitor size={14} class="text-blue-400" />
            <span>Desktop</span>
          </button>

          <button
            class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-primary)]"
            on:click={() => jumpTo(`${homeDir}/Downloads`)}
          >
            <Download size={14} class="text-green-400" />
            <span>Downloads</span>
          </button>

          <button
            class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-primary)]"
            on:click={() => jumpTo(`${homeDir}/Documents`)}
          >
            <FileText size={14} class="text-yellow-400" />
            <span>Documents</span>
          </button>
        {/if}

        <button
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-primary)]"
          on:click={() => jumpTo(devProjectsDir || '/')}
        >
          <FolderGit2 size={14} class="text-purple-400" />
          <span>Projects</span>
        </button>
      </div>
    </div>

    <!-- Remote SSH Servers -->
    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Remote (SSH)</span>
      <div class="mt-1 space-y-0.5">
        <button
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-green-400"
          on:click={() => jumpTo('~', true, 'marvin.cgu.igp.uu.se')}
        >
          <Server size={14} />
          <span>Marvin (HPC)</span>
        </button>
      </div>
    </div>

    <!-- Themes & Skins -->
    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Theme & Skins</span>
      <div class="mt-1 space-y-0.5">
        {#each themes as t}
          <button
            class="w-full flex items-center justify-between px-2 py-1.5 rounded text-left transition-colors {$currentTheme === t.id ? 'bg-[var(--accent-subtle)] text-[var(--accent)] font-semibold' : 'text-[var(--text-primary)] hover:bg-[var(--bg-hover)]'}"
            on:click={() => setTheme(t.id)}
          >
            <div class="flex items-center gap-2">
              <svelte:component this={t.icon} size={14} />
              <span>{t.label}</span>
            </div>
            {#if $currentTheme === t.id}
              <span class="w-1.5 h-1.5 rounded-full bg-[var(--accent)]"></span>
            {/if}
          </button>
        {/each}
      </div>
    </div>

    <!-- Options -->
    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Options</span>
      <div class="mt-1">
        <button
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-secondary)]"
          on:click={() => showHiddenFiles.update((v) => !v)}
        >
          {#if $showHiddenFiles}
            <EyeOff size={14} class="text-[var(--accent)]" />
            <span>Hide Dotfiles</span>
          {:else}
            <Eye size={14} />
            <span>Show Dotfiles</span>
          {/if}
        </button>
      </div>
    </div>
  </div>

  <!-- Disk space progressbar -->
  <DiskBar path={homeDir || '/'} />
</aside>
