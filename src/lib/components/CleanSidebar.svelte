<script lang="ts">
  import { onMount } from 'svelte';
  import { getHomeDirectory } from '../invoke';
  import { navigatePane, activePaneId, leftPane, setLayoutMode } from '../stores/navigation';
  import { openIndexScan } from '../stores/indexStore';
  import { stashItems, toggleStash } from '../stores/stash';
  import type { FileTypeIndexMeta } from '../types';
  import DiskBar from './DiskBar.svelte';
  import {
    Clock,
    Home,
    Monitor,
    Download,
    FileText,
    Table,
    Image as ImageIcon,
    Archive,
    Code,
    Layers,
    LayoutTemplate,
  } from 'lucide-svelte';

  let homeDir = '';

  onMount(async () => {
    try {
      homeDir = await getHomeDirectory();
    } catch (e) {
      console.warn('Kunde inte hämta hemkatalogen:', e);
    }
  });

  $: currentPath = $leftPane.currentPath;

  function jumpTo(path: string) {
    navigatePane($activePaneId, path);
  }

  /**
   * The content groups a general file browser needs - not the BAM/VCF/FASTQ
   * ones the pro sidebar offers. Scanning one runs the same index walk the pro
   * mode uses; the count only exists once that has run, so none is invented
   * here.
   */
  const contentGroups: { id: string; label: string; extensions: string[]; icon: any; color: string }[] = [
    {
      id: 'clean-docs',
      label: 'Dokument',
      extensions: ['pdf', 'doc', 'docx', 'odt', 'rtf', 'txt', 'md', 'pages'],
      icon: FileText,
      color: 'text-blue-400',
    },
    {
      id: 'clean-sheets',
      label: 'Kalkylblad',
      extensions: ['xlsx', 'xls', 'ods', 'csv', 'tsv', 'numbers'],
      icon: Table,
      color: 'text-emerald-400',
    },
    {
      id: 'clean-images',
      label: 'Bilder',
      extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'heic', 'svg', 'tiff'],
      icon: ImageIcon,
      color: 'text-pink-400',
    },
    {
      id: 'clean-archives',
      label: 'Arkiv',
      extensions: ['zip', 'tar', 'gz', 'tgz', 'bz2', 'xz', '7z', 'rar', 'dmg', 'iso'],
      icon: Archive,
      color: 'text-amber-400',
    },
    {
      id: 'clean-code',
      label: 'Kod',
      extensions: ['py', 'rs', 'js', 'ts', 'svelte', 'sh', 'c', 'cpp', 'h', 'swift', 'go', 'java', 'r'],
      icon: Code,
      color: 'text-purple-400',
    },
  ];

  function scanGroup(group: (typeof contentGroups)[number]) {
    const root = currentPath || homeDir;
    if (!root) return;
    const meta: FileTypeIndexMeta = {
      id: group.id,
      name: group.label,
      extensions: group.extensions,
      badge: group.label.slice(0, 3).toUpperCase(),
      iconName: group.label,
      colorClass: group.color,
    };
    openIndexScan(meta, root);
  }
</script>

<div class="h-full flex flex-col bg-[var(--bg-surface)] border-r border-[var(--border)] select-none">
  <div class="px-3 py-3 border-b border-[var(--border)] flex items-center justify-between">
    <span class="font-bold text-sm text-[var(--text-primary)]">Flashbrowse</span>
    <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[var(--accent-subtle)] text-[var(--accent)]">
      Clean
    </span>
  </div>

  <div class="flex-1 overflow-y-auto p-2 space-y-4">
    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Snabbt</span>
      <div class="mt-1 space-y-0.5">
        <button
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
          on:click={() => homeDir && jumpTo(`${homeDir}/Desktop`)}
        >
          <Clock size={14} class="text-[var(--accent)]" />
          <span>Senaste</span>
        </button>
        <button
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
          on:click={() => homeDir && jumpTo(homeDir)}
        >
          <Home size={14} class="text-[var(--accent)]" />
          <span>Hem</span>
        </button>
        <button
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
          on:click={() => homeDir && jumpTo(`${homeDir}/Desktop`)}
        >
          <Monitor size={14} class="text-blue-400" />
          <span>Skrivbord</span>
        </button>
        <button
          class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
          on:click={() => homeDir && jumpTo(`${homeDir}/Downloads`)}
        >
          <Download size={14} class="text-green-400" />
          <span>Hämtade filer</span>
        </button>
      </div>
    </div>

    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Innehåll</span>
      <div class="mt-1 space-y-0.5">
        {#each contentGroups as group (group.id)}
          <button
            class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
            on:click={() => scanGroup(group)}
            title="Leta upp alla {group.label.toLowerCase()} under den här mappen"
          >
            <svelte:component this={group.icon} size={14} class={group.color} />
            <span class="truncate">{group.label}</span>
          </button>
        {/each}
      </div>
    </div>

    <div>
      <span class="px-2 text-[10px] font-semibold text-[var(--text-muted)] tracking-wider uppercase">Samlingsfack</span>
      <div class="mt-1">
        <button
          class="w-full flex items-center justify-between px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left {$stashItems.length >
          0
            ? 'text-[var(--accent)] font-semibold'
            : 'text-[var(--text-secondary)]'}"
          on:click={toggleStash}
        >
          <span class="flex items-center gap-2">
            <Layers size={14} class={$stashItems.length > 0 ? 'text-[var(--accent)]' : ''} />
            <span>{$stashItems.length > 0 ? `${$stashItems.length} filer` : 'Tomt'}</span>
          </span>
        </button>
      </div>
    </div>
  </div>

  <div class="p-2 border-t border-[var(--border)] space-y-2">
    <DiskBar path={currentPath} />
    <button
      class="w-full flex items-center gap-2 px-2 py-1.5 rounded hover:bg-[var(--bg-hover)] text-left text-[var(--text-muted)] hover:text-[var(--text-primary)] text-[11px]"
      on:click={() => setLayoutMode('pro')}
      title="Tillbaka till pro-läget: två paneler, terminal och bioinformatikverktygen"
    >
      <LayoutTemplate size={13} />
      <span>Byt till pro-läge</span>
    </button>
  </div>
</div>
