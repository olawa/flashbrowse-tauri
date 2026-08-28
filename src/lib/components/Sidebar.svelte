<script lang="ts">
  import { onMount } from 'svelte';
  import { getHomeDirectory } from '../invoke';
  import {
    navigatePane,
    activePaneId,
    showHiddenFiles,
    leftPane,
    rightPane,
    clickMode,
    smartHoverPreview,
  } from '../stores/navigation';
  import { currentTheme, setTheme, isKidsMode } from '../stores/theme';
  import DiskBar from './DiskBar.svelte';
  import { stashItems, toggleStash } from '../stores/stash';
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
    MousePointerClick,
    Dna,
    Table,
    Code,
    Bookmark,
  Layers,
  Zap,
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

  function applyIndexFilter(filterQuery: string) {
    const paneId = $activePaneId;
    const store = paneId === 'left' ? leftPane : rightPane;
    store.update((s) => {
      if (s.filterQuery === filterQuery) {
        return { ...s, filterQuery: '' };
      }
      return { ...s, filterQuery };
    });
  }

  const indexCategories = [
    { label: 'BAM & CRAM', filter: '*.bam, *.cram, *.sam', badge: 'BAM', icon: Dna, color: 'text-emerald-400' },
    { label: 'VCF & BCF', filter: '*.vcf, *.vcf.gz, *.bcf', badge: 'VCF', icon: Dna, color: 'text-purple-400' },
    { label: 'FASTQ Reads', filter: '*.fastq, *.fq, *.fastq.gz, *.fq.gz', badge: 'FASTQ', icon: Dna, color: 'text-cyan-400' },
    { label: 'Tabeller & Sheets', filter: '*.csv, *.tsv, *.tab, *.xlsx', badge: 'TABLE', icon: Table, color: 'text-blue-400' },
    { label: 'Annotationer', filter: '*.bed, *.gtf, *.gff, *.bigwig', badge: 'BED', icon: Bookmark, color: 'text-pink-400' },
    { label: 'Källkod & Skript', filter: '*.rs, *.py, *.ts, *.js, *.sh, *.c, *.swift', badge: 'CODE', icon: Code, color: 'text-yellow-400' },
    { label: 'Dokument & Text', filter: '*.md, *.pdf, *.txt, *.doc', badge: 'DOC', icon: FileText, color: 'text-slate-300' },
  ];

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

    <!-- Index Hub (Filtypsindexering) -->
    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Filtypsindex (Hub)</span>
      <div class="mt-1 space-y-0.5">
        {#each indexCategories as cat}
          {@const isActive = ($activePaneId === 'left' ? $leftPane.filterQuery : $rightPane.filterQuery) === cat.filter}
          <button
            class="w-full flex items-center justify-between px-2 py-1.5 rounded text-left transition-colors group {isActive ? 'bg-[var(--accent-subtle)] text-[var(--accent)] font-semibold border border-[var(--accent)]/40' : 'hover:bg-[var(--bg-hover)] text-[var(--text-primary)]'}"
            on:click={() => applyIndexFilter(cat.filter)}
            title="Filtrera mappen på {cat.label} ({cat.filter})"
          >
            <div class="flex items-center gap-2 truncate">
              <svelte:component this={cat.icon} size={14} class="{cat.color} shrink-0" />
              <span class="truncate {isActive ? 'text-[var(--accent)]' : 'group-hover:text-white'}">{cat.label}</span>
            </div>
            <span class="text-[9.5px] font-mono px-1 py-0.2 rounded {isActive ? 'bg-[var(--accent)] text-white font-bold' : 'bg-[#191d26] text-slate-400 group-hover:text-slate-200 border border-[#262d3d]'}">
              {cat.badge}
            </span>
          </button>
        {/each}
      </div>
    </div>

    <!-- Stash Shelf -->
    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Staging</span>
      <div class="mt-1 space-y-0.5">
        <button
          class="w-full flex items-center justify-between px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left {$stashItems.length > 0 ? 'text-[var(--accent)] font-semibold' : 'text-[var(--text-secondary)]'}"
          on:click={toggleStash}
        >
          <div class="flex items-center gap-2">
            <Layers size={14} class={$stashItems.length > 0 ? 'text-[var(--accent)]' : ''} />
            <span>Samlingsfack (Stash)</span>
          </div>
          <span class="text-[10px] px-1.5 py-0.2 rounded-full {$stashItems.length > 0 ? 'bg-[var(--accent)] text-white font-bold' : 'bg-[var(--border)] text-slate-400 font-mono'}">
            {$stashItems.length}
          </span>
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

    <!-- Options & Behavior -->
    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Behavior</span>
      <div class="mt-1 space-y-0.5">
        <button
          class="w-full flex items-center justify-between px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-secondary)]"
          on:click={() => clickMode.update((m) => (m === 'folders-only' ? 'double-click' : 'folders-only'))}
        >
          <div class="flex items-center gap-2">
            <MousePointerClick size={14} class={$clickMode === 'folders-only' ? 'text-[var(--accent)]' : ''} />
            <span>{$clickMode === 'folders-only' ? 'Enkelklick Mappar' : 'Dubbelklick Mappar'}</span>
          </div>
          <span class="text-[9px] px-1 rounded bg-[var(--border)] font-mono">
            {$clickMode === 'folders-only' ? '1x' : '2x'}
          </span>
        </button>

        <button
          class="w-full flex items-center justify-between px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-secondary)]"
          on:click={() => smartHoverPreview.update((v) => !v)}
        >
          <div class="flex items-center gap-2">
            <Zap size={14} class={$smartHoverPreview ? 'text-amber-400' : ''} />
            <span>Hover Preview</span>
          </div>
          <span class="text-[9px] px-1 rounded bg-[var(--border)] font-mono">
            {$smartHoverPreview ? 'ON' : 'OFF'}
          </span>
        </button>

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
