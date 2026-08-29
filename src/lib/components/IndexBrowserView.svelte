<script lang="ts">
  import { onMount } from 'svelte';
  import {
    activeIndexMeta,
    indexRootPath,
    indexedGroups,
    selectedDirectories,
    isIndexScanning,
    indexSearchQuery,
    activeIndexGroups,
    activeIndexFilteredItems,
    openIndexScan,
    closeIndexView,
    selectAllIndexDirs,
    deselectAllIndexDirs,
    toggleIndexDir,
    refreshCurrentIndex,
    activeHighlightedParentDir,
  } from '../stores/indexStore';
  import {
    leftPane,
    rightPane,
    activePaneId,
    navigatePane,
    activeHoveredItem,
    triggerInspectorScroll,
    castToSecondaryInspector,
  } from '../stores/navigation';
  import { openInDefault, launchRsnap, revealInOs } from '../invoke';
  import type { FileItem, DirectoryIndexGroup } from '../types';
  import {
    Folder,
    FileText,
    Dna,
    Table,
    Code,
    Bookmark,
    Search,
    X,
    FolderTree,
    ChevronDown,
    ExternalLink,
    FolderOpen,
    Play,
    CheckSquare,
    Square,
    Loader2,
    Sparkles,
    Rocket,
    RefreshCw,
  } from 'lucide-svelte';

  export let onSelectPreview: (item: FileItem) => void = () => {};

  let hoveredPath: string | null = null;
  let isRootMenuOpen = false;

  $: totalFolders = $indexedGroups.length;
  $: selectedFolderCount = $selectedDirectories.size === 0 ? totalFolders : $selectedDirectories.size;
  $: isAllSelected = $selectedDirectories.size === totalFolders && totalFolders > 0;

  function handleFolderClick(group: DirectoryIndexGroup, e: MouseEvent) {
    toggleIndexDir(group.directory_path, e.metaKey || e.ctrlKey);
  }

  function handleFileClick(item: FileItem) {
    const parentDir = item.path.substring(0, item.path.lastIndexOf('/')) || '/';
    activeHighlightedParentDir.set(parentDir);
    onSelectPreview(item);
  }

  function handleFileDblClick(item: FileItem) {
    // Open in default app or navigate left pane to its folder
    const parentDir = item.path.substring(0, item.path.lastIndexOf('/')) || '/';
    navigatePane('left', parentDir);
    closeIndexView();
  }

  function handleFileMouseEnter(item: FileItem) {
    hoveredPath = item.path;
    const parentDir = item.path.substring(0, item.path.lastIndexOf('/')) || '/';
    activeHighlightedParentDir.set(parentDir);
    activeHoveredItem.set(item);
    onSelectPreview(item);
  }

  function handleFileMouseLeave() {
    hoveredPath = null;
  }

  function handleRowWheel(item: FileItem, e: WheelEvent) {
    if (e.ctrlKey) return;
    if (Math.abs(e.deltaY) > 0) {
      triggerInspectorScroll(e.deltaY);
    }
  }

  async function openAllInRsnap() {
    const bams = $activeIndexFilteredItems.map((i) => i.path);
    if (bams.length === 0) return;
    try {
      await launchRsnap(bams.slice(0, 10)); // launch up to 10 bams
    } catch (err) {
      console.warn('Failed to launch rsnap:', err);
    }
  }

  function getFileIcon(item: FileItem) {
    const ext = item.extension.toLowerCase();
    if (['bam', 'cram', 'sam', 'vcf', 'bcf', 'fastq', 'fq'].includes(ext)) return Dna;
    if (['csv', 'tsv', 'tab', 'xlsx'].includes(ext)) return Table;
    if (['rs', 'py', 'ts', 'js', 'sh', 'c', 'swift'].includes(ext)) return Code;
    if (['bed', 'gtf', 'gff'].includes(ext)) return Bookmark;
    return FileText;
  }
</script>

{#if $activeIndexMeta}
  <div class="flex-1 flex flex-col h-full min-h-0 bg-[var(--bg-base)] text-[var(--text-primary)] select-none overflow-hidden font-sans">
    <!-- Top Hub Header -->
    <div class="px-4 py-2.5 bg-[var(--bg-surface)] border-b border-[var(--border)] flex items-center justify-between gap-3 shrink-0">
      <div class="flex items-center gap-3 min-w-0">
        <!-- Index Category Badge -->
        <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-[var(--bg-panel)] border border-[var(--border)] {$activeIndexMeta.colorClass} font-bold text-xs">
          <span>{$activeIndexMeta.name}</span>
          <span class="px-1 py-0.2 rounded bg-black/40 text-[10px] font-mono">{$activeIndexMeta.badge}</span>
        </div>

        <!-- Root Directory Selector Dropdown -->
        <div class="relative">
          <button
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--bg-hover)] border border-[var(--border)] text-xs text-[var(--text-secondary)] hover:text-white transition-colors"
            on:click={() => (isRootMenuOpen = !isRootMenuOpen)}
          >
            <FolderTree size={12} class="text-[var(--accent)]" />
            <span class="font-mono text-[11px] truncate max-w-[200px]" title={$indexRootPath}>
              {$indexRootPath.split('/').pop() || '~'}
            </span>
            <ChevronDown size={11} />
          </button>

          {#if isRootMenuOpen}
            <div class="absolute left-0 top-full mt-1 w-64 bg-[var(--bg-surface)] border border-[var(--border)] rounded-lg shadow-2xl z-50 py-1 text-xs">
              <button
                class="w-full text-left px-3 py-1.5 hover:bg-[var(--bg-hover)] flex items-center gap-2"
                on:click={async () => {
                  isRootMenuOpen = false;
                  const home = await (await import('../invoke')).getHomeDirectory();
                  $activeIndexMeta && openIndexScan($activeIndexMeta, home);
                }}
              >
                <span>🏠 Hemkatalog (~)</span>
              </button>
              <button
                class="w-full text-left px-3 py-1.5 hover:bg-[var(--bg-hover)] flex items-center gap-2"
                on:click={() => {
                  isRootMenuOpen = false;
                  $activeIndexMeta && openIndexScan($activeIndexMeta, $leftPane.currentPath);
                }}
              >
                <span>👈 Vänster panel ({$leftPane.currentPath.split('/').pop() || '/'})</span>
              </button>
              <button
                class="w-full text-left px-3 py-1.5 hover:bg-[var(--bg-hover)] flex items-center gap-2"
                on:click={() => {
                  isRootMenuOpen = false;
                  $activeIndexMeta && openIndexScan($activeIndexMeta, $rightPane.currentPath);
                }}
              >
                <span>👉 Höger panel ({$rightPane.currentPath.split('/').pop() || '/'})</span>
              </button>
            </div>
          {/if}
        </div>

        <!-- Files & Folders Count -->
        <span class="text-xs text-[var(--text-muted)] font-mono hidden md:inline">
          • {$activeIndexFilteredItems.length} filer i {selectedFolderCount} av {totalFolders} mappar
        </span>

        {#if $isIndexScanning}
          <div class="flex items-center gap-1 text-amber-400 text-xs font-mono animate-pulse">
            <Loader2 size={12} class="animate-spin" />
            <span>Skannar...</span>
          </div>
        {/if}
      </div>

      <!-- Actions & Search -->
      <div class="flex items-center gap-2 shrink-0">
        <!-- Search filter inside index -->
        <div class="relative flex items-center">
          <Search size={11} class="absolute left-2 text-slate-500" />
          <input
            type="text"
            placeholder="Filtrera index..."
            bind:value={$indexSearchQuery}
            class="pl-6 pr-2 py-1 bg-[var(--bg-panel)] text-xs text-[var(--text-primary)] rounded-md border border-[var(--border)] focus:outline-none focus:border-[var(--accent)] w-36 sm:w-48 font-mono"
          />
        </div>

        <!-- BAM rsnap action button -->
        {#if $activeIndexMeta.id === 'bam' && $activeIndexFilteredItems.length > 0}
          <button
            class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-emerald-600/20 hover:bg-emerald-600/30 text-emerald-300 border border-emerald-500/40 text-xs font-bold transition-colors"
            on:click={openAllInRsnap}
            title="Öppna markerade BAM-filer i rsnap viewer"
          >
            <Play size={11} />
            <span class="hidden sm:inline">rsnap</span>
          </button>
        {/if}

        <!-- Refresh Index Button -->
        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--bg-hover)] border border-[var(--border)] text-xs text-slate-300 hover:text-white transition-colors"
          on:click={refreshCurrentIndex}
          disabled={$isIndexScanning}
          title="Läs om och uppdatera indexet från disk"
        >
          <RefreshCw size={11} class={$isIndexScanning ? 'animate-spin text-amber-400' : ''} />
          <span class="hidden sm:inline">Uppdatera</span>
        </button>

        <!-- Close Index Button -->
        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--bg-hover)] border border-[var(--border)] text-xs text-slate-300 hover:text-white transition-colors"
          on:click={closeIndexView}
          title="Stäng index och återgå till vanlig fillista (Esc)"
        >
          <X size={12} />
          <span class="hidden sm:inline">Stäng index</span>
        </button>
      </div>
    </div>

    <!-- Main Split Columns Area: Folders | Files -->
    <div class="flex-1 flex min-h-0 overflow-hidden">
      <!-- 1. LEFT COLUMN: Directory Groups List -->
      <div class="w-64 lg:w-72 h-full flex flex-col border-r border-[var(--border)] bg-[var(--bg-surface)] shrink-0">
        <!-- Folder Selection Bar -->
        <div class="px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-panel)] flex items-center justify-between text-[11px]">
          <span class="font-bold text-[var(--text-secondary)]">Mappar ({totalFolders})</span>
          <div class="flex items-center gap-2">
            <button
              class="text-[var(--accent)] hover:underline text-[10.5px]"
              on:click={selectAllIndexDirs}
            >
              Alla
            </button>
            <span class="text-slate-600">•</span>
            <button
              class="text-slate-400 hover:underline text-[10.5px]"
              on:click={deselectAllIndexDirs}
            >
              Rensa
            </button>
          </div>
        </div>

        <!-- Folders List -->
        <div class="flex-1 overflow-y-auto divide-y divide-[var(--border)]/30">
          {#each $indexedGroups as group}
            {@const isSelected = $selectedDirectories.size === 0 || $selectedDirectories.has(group.directory_path)}
            {@const isParentOfActive = $activeHighlightedParentDir === group.directory_path}
            <div
              class="px-3 py-2 flex items-start gap-2 cursor-pointer transition-all {isParentOfActive ? 'bg-[var(--accent)]/15 border-l-4 border-l-[var(--accent)] ring-1 ring-[var(--accent)]/30 text-white font-medium shadow-sm' : isSelected ? 'bg-[var(--accent-subtle)] text-[var(--text-primary)]' : 'opacity-60 hover:opacity-90'}"
              on:click={(e) => handleFolderClick(group, e)}
              role="button"
              tabindex="-1"
            >
              <div class="mt-0.5 text-[var(--accent)]">
                {#if isSelected}
                  <CheckSquare size={13} />
                {:else}
                  <Square size={13} />
                {/if}
              </div>

              <div class="flex-1 min-w-0">
                <div class="flex items-center justify-between gap-1">
                  <span class="font-semibold text-xs truncate {isParentOfActive ? 'text-white' : ''}" title={group.directory_name}>
                    {group.directory_name}
                  </span>
                  <div class="flex items-center gap-1 shrink-0">
                    {#if isParentOfActive}
                      <span class="px-1.5 py-0.2 rounded bg-[var(--accent)] text-white text-[9px] font-bold tracking-wide">
                        Aktiv
                      </span>
                    {/if}
                    <span class="px-1.5 py-0.2 rounded-full bg-[var(--bg-panel)] text-[10px] font-mono text-[var(--text-secondary)] shrink-0">
                      {group.items.length}
                    </span>
                  </div>
                </div>
                <div class="text-[10px] font-mono text-[var(--text-muted)] truncate" title={group.relative_path}>
                  {group.relative_path}
                </div>
              </div>
            </div>
          {/each}

          {#if totalFolders === 0 && !$isIndexScanning}
            <div class="p-6 text-center text-xs text-[var(--text-muted)]">
              Inga mappar med matchande filer hittades i {$indexRootPath.split('/').pop() || '~'}.
            </div>
          {/if}
        </div>
      </div>

      <!-- 2. RIGHT COLUMN: Files Table -->
      <div class="flex-1 flex flex-col min-w-0 h-full overflow-hidden">
        <!-- Table Header -->
        <div class="grid grid-cols-12 gap-2 px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-panel)] text-[var(--text-muted)] font-sans font-semibold text-[11px] shrink-0 sticky top-0">
          <div class="col-span-8">Namn</div>
          <div class="col-span-2 text-right">Storlek</div>
          <div class="col-span-2 text-right pr-1">Ändrad</div>
        </div>

        <!-- Files List -->
        <div class="flex-1 overflow-y-auto divide-y divide-[var(--border)]/40 font-mono text-xs">
          {#each $activeIndexFilteredItems as item}
            {@const isHovered = hoveredPath === item.path}
            <div
              class="grid grid-cols-12 gap-2 px-3 py-1.5 items-center cursor-pointer transition-colors {isHovered ? 'bg-[var(--bg-hover)] text-white' : 'text-[var(--text-primary)]'}"
              on:click={() => handleFileClick(item)}
              on:dblclick={() => handleFileDblClick(item)}
              on:mouseenter={() => handleFileMouseEnter(item)}
              on:mouseleave={handleFileMouseLeave}
              on:wheel={(e) => handleRowWheel(item, e)}
              role="row"
              tabindex="-1"
            >
              <!-- Name Column with directory path -->
              <div class="col-span-8 flex items-center gap-2 min-w-0">
                <svelte:component this={getFileIcon(item)} size={14} class="{$activeIndexMeta.colorClass} shrink-0" />
                <div class="flex flex-col min-w-0 flex-1">
                  <span class="truncate font-sans font-medium">{item.name}</span>
                  <span class="text-[10px] text-[var(--text-muted)] font-mono truncate" title={item.path}>
                    {item.path.replace($indexRootPath, '.')}
                  </span>
                </div>
              </div>

              <!-- Size -->
              <div class="col-span-2 text-right text-[var(--text-secondary)] font-mono text-[11px]">
                {item.formatted_size}
              </div>

              <!-- Modified -->
              <div class="col-span-2 text-right pr-1 text-[var(--text-muted)] font-mono text-[11px] truncate">
                {item.formatted_modified}
              </div>
            </div>
          {/each}

          {#if $activeIndexFilteredItems.length === 0 && !$isIndexScanning}
            <div class="p-12 text-center text-[var(--text-muted)] flex flex-col items-center justify-center space-y-2">
              <Folder size={32} class="opacity-20" />
              <span>Inga matchande filer i valda mappar.</span>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
