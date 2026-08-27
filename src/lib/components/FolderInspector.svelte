<script lang="ts">
  import { onMount } from 'svelte';
  import { listDirectory, calculateDirSize, revealInOs, openInDefault } from '../invoke';
  import { activePaneId, navigatePane } from '../stores/navigation';
  import type { FileItem, DirectorySummary } from '../types';
  import {
    Folder,
    FolderOpen,
    File,
    FileText,
    PieChart,
    RefreshCw,
    Search,
    Copy,
    Check,
    ExternalLink,
    ArrowRight,
    Dna,
    Table,
    Code,
    Image as ImageIcon,
    Archive,
    SlidersHorizontal,
    X,
  } from 'lucide-svelte';

  export let item: FileItem;

  let children: FileItem[] = [];
  let isLoadingChildren = false;
  let dirSummary: DirectorySummary | null = null;
  let isCalculatingDu = false;
  let filterText = '';
  let selectedCategoryFilter: string | null = null;
  let copied = false;
  let errorMessage = '';

  $: if (item && item.is_dir) {
    loadChildren(item.path);
  }

  async function loadChildren(path: string) {
    isLoadingChildren = true;
    errorMessage = '';
    dirSummary = null;
    filterText = '';
    selectedCategoryFilter = null;
    try {
      const items = await listDirectory(path, false);
      // Sort: Folders first, then alphabetically
      children = items.sort((a, b) => {
        if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
        return a.name.localeCompare(b.name);
      });
    } catch (e: any) {
      errorMessage = String(e);
      children = [];
    } finally {
      isLoadingChildren = false;
    }
  }

  async function calculateFolderDu() {
    if (!item || !item.is_dir) return;
    isCalculatingDu = true;
    try {
      dirSummary = await calculateDirSize(item.path);
    } catch (e: any) {
      alert(`Kunde inte beräkna mappstorlek: ${e}`);
    } finally {
      isCalculatingDu = false;
    }
  }

  async function copyPath() {
    if (!item) return;
    try {
      await navigator.clipboard.writeText(item.path);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (err) {
      console.warn('Clipboard write failed:', err);
    }
  }

  function handleNavigateToChild(child: FileItem) {
    if (child.is_dir) {
      navigatePane($activePaneId, child.path);
    } else {
      openInDefault(child.path);
    }
  }

  function handleOpenFolderInPane() {
    if (item && item.is_dir) {
      navigatePane($activePaneId, item.path);
    }
  }

  // Category statistics breakdown
  $: folderCount = children.filter((c) => c.is_dir).length;
  $: fileCount = children.filter((c) => !c.is_dir).length;

  $: categoryStats = (() => {
    let bamCount = 0;
    let vcfCount = 0;
    let tableCount = 0;
    let codeCount = 0;
    let textCount = 0;
    let imageCount = 0;
    let archiveCount = 0;

    for (const c of children) {
      if (c.is_dir) continue;
      const ext = c.extension.toLowerCase();
      const name = c.name.toLowerCase();

      if (ext === 'bam' || ext === 'cram' || ext === 'sam' || name.endsWith('.bam') || name.endsWith('.cram')) {
        bamCount++;
      } else if (ext === 'vcf' || ext === 'bcf' || name.endsWith('.vcf.gz')) {
        vcfCount++;
      } else if (ext === 'tsv' || ext === 'csv' || ext === 'tab' || ext === 'xlsx' || ext === 'xls') {
        tableCount++;
      } else if (['rs', 'py', 'ts', 'js', 'sh', 'c', 'cpp', 'swift', 'r', 'json', 'toml', 'yaml', 'yml'].includes(ext)) {
        codeCount++;
      } else if (['txt', 'log', 'md', 'pdf', 'doc', 'docx'].includes(ext)) {
        textCount++;
      } else if (['png', 'jpg', 'jpeg', 'webp', 'svg', 'gif', 'tiff'].includes(ext)) {
        imageCount++;
      } else if (['zip', 'tar', 'gz', 'tgz', 'bz2', 'xz'].includes(ext)) {
        archiveCount++;
      }
    }

    const list = [];
    if (folderCount > 0) list.push({ key: 'folder', label: `${folderCount} Mappar`, icon: Folder, color: 'text-amber-400 bg-amber-950/40 border-amber-800/60' });
    if (bamCount > 0) list.push({ key: 'bam', label: `${bamCount} BAM/CRAM`, icon: Dna, color: 'text-emerald-400 bg-emerald-950/40 border-emerald-800/60' });
    if (vcfCount > 0) list.push({ key: 'vcf', label: `${vcfCount} VCF`, icon: Dna, color: 'text-purple-400 bg-purple-950/40 border-purple-800/60' });
    if (tableCount > 0) list.push({ key: 'table', label: `${tableCount} Tabeller`, icon: Table, color: 'text-blue-400 bg-blue-950/40 border-blue-800/60' });
    if (codeCount > 0) list.push({ key: 'code', label: `${codeCount} Kod/Script`, icon: Code, color: 'text-yellow-400 bg-yellow-950/40 border-yellow-800/60' });
    if (textCount > 0) list.push({ key: 'text', label: `${textCount} Dokument`, icon: FileText, color: 'text-slate-300 bg-slate-800/40 border-slate-700/60' });
    if (imageCount > 0) list.push({ key: 'image', label: `${imageCount} Bilder`, icon: ImageIcon, color: 'text-pink-400 bg-pink-950/40 border-pink-800/60' });
    if (archiveCount > 0) list.push({ key: 'archive', label: `${archiveCount} Arkiv`, icon: Archive, color: 'text-orange-400 bg-orange-950/40 border-orange-800/60' });
    return list;
  })();

  function matchesCategory(c: FileItem, cat: string): boolean {
    if (cat === 'folder') return c.is_dir;
    if (c.is_dir) return false;
    const ext = c.extension.toLowerCase();
    const name = c.name.toLowerCase();

    if (cat === 'bam') return ext === 'bam' || ext === 'cram' || ext === 'sam' || name.endsWith('.bam') || name.endsWith('.cram');
    if (cat === 'vcf') return ext === 'vcf' || ext === 'bcf' || name.endsWith('.vcf.gz');
    if (cat === 'table') return ext === 'tsv' || ext === 'csv' || ext === 'tab' || ext === 'xlsx' || ext === 'xls';
    if (cat === 'code') return ['rs', 'py', 'ts', 'js', 'sh', 'c', 'cpp', 'swift', 'r', 'json', 'toml', 'yaml', 'yml'].includes(ext);
    if (cat === 'text') return ['txt', 'log', 'md', 'pdf', 'doc', 'docx'].includes(ext);
    if (cat === 'image') return ['png', 'jpg', 'jpeg', 'webp', 'svg', 'gif', 'tiff'].includes(ext);
    if (cat === 'archive') return ['zip', 'tar', 'gz', 'tgz', 'bz2', 'xz'].includes(ext);
    return true;
  }

  $: filteredChildren = children.filter((c) => {
    if (selectedCategoryFilter && !matchesCategory(c, selectedCategoryFilter)) {
      return false;
    }
    if (!filterText.trim()) return true;
    return c.name.toLowerCase().includes(filterText.toLowerCase());
  });

  function getFileIcon(c: FileItem) {
    if (c.is_dir) return { icon: Folder, color: 'text-amber-400' };
    const ext = c.extension.toLowerCase();
    const name = c.name.toLowerCase();

    if (ext === 'bam' || ext === 'cram' || ext === 'sam' || name.endsWith('.bam') || name.endsWith('.cram')) {
      return { icon: Dna, color: 'text-emerald-400' };
    }
    if (ext === 'vcf' || ext === 'bcf' || name.endsWith('.vcf.gz')) {
      return { icon: Dna, color: 'text-purple-400' };
    }
    if (ext === 'tsv' || ext === 'csv' || ext === 'tab' || ext === 'xlsx') {
      return { icon: Table, color: 'text-blue-400' };
    }
    if (['rs', 'py', 'ts', 'js', 'sh', 'c', 'cpp', 'swift', 'r'].includes(ext)) {
      return { icon: Code, color: 'text-yellow-400' };
    }
    if (['png', 'jpg', 'jpeg', 'webp', 'svg'].includes(ext)) {
      return { icon: ImageIcon, color: 'text-pink-400' };
    }
    if (['zip', 'tar', 'gz', 'tgz'].includes(ext)) {
      return { icon: Archive, color: 'text-orange-400' };
    }
    return { icon: FileText, color: 'text-slate-400' };
  }
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden bg-[#0d0f14] text-slate-200">
  <!-- Folder Overview Header -->
  <div class="px-3 py-2 bg-[#151922] border-b border-[#252d3d] flex flex-col gap-2">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2 min-w-0">
        <div class="w-7 h-7 rounded-lg bg-[var(--accent)]/20 text-[var(--accent)] flex items-center justify-center shrink-0">
          <Folder size={16} class="fill-[var(--accent)]/30" />
        </div>
        <div class="truncate">
          <div class="flex items-center gap-1.5">
            <span class="font-bold text-xs text-white truncate" title={item.name}>{item.name}</span>
            <span class="px-1.5 py-0.2 rounded bg-amber-950 text-amber-400 text-[10px] font-mono border border-amber-800/80">
              MAPP
            </span>
          </div>
          <div class="text-[11px] text-slate-400 font-mono flex items-center gap-1.5 mt-0.5">
            <span>{children.length} objekt ({folderCount} mappar, {fileCount} filer)</span>
          </div>
        </div>
      </div>

      <button
        class="flex items-center gap-1 px-2 py-1 rounded bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-[11px] font-semibold transition-colors shadow-sm shrink-0"
        on:click={handleOpenFolderInPane}
        title="Navigera in i denna mapp i filtabellen"
      >
        <span>Öppna mapp</span>
        <ArrowRight size={11} />
      </button>
    </div>

    <!-- Category Breakdown Badges -->
    {#if categoryStats.length > 0}
      <div class="flex flex-wrap gap-1 pt-1 border-t border-[#252d3d]/60">
        {#each categoryStats as cat}
          <button
            class="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium border transition-colors {cat.color} {selectedCategoryFilter === cat.key ? 'ring-1 ring-white/60 brightness-125' : 'opacity-85 hover:opacity-100'}"
            on:click={() => {
              selectedCategoryFilter = selectedCategoryFilter === cat.key ? null : cat.key;
            }}
            title="Klicka för att filtrera innehållet på {cat.label}"
          >
            <svelte:component this={cat.icon} size={10} />
            <span>{cat.label}</span>
          </button>
        {/each}

        {#if selectedCategoryFilter}
          <button
            class="flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[10px] text-slate-400 hover:text-white bg-slate-800/50"
            on:click={() => (selectedCategoryFilter = null)}
          >
            <X size={10} />
            <span>Rensa filter</span>
          </button>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Quick Search Inside Folder -->
  <div class="px-3 py-1.5 border-b border-[#252d3d] bg-[#11141b] flex items-center gap-2">
    <div class="relative flex-1">
      <Search size={11} class="absolute left-2.5 top-2 text-slate-500" />
      <input
        type="text"
        bind:value={filterText}
        placeholder="Snabbkoll: sök i mappen (t.ex. *.bam, test)..."
        class="w-full bg-[#0e1015] border border-[#252d3d] rounded-md pl-7 pr-7 py-1 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-[var(--accent)]"
      />
      {#if filterText}
        <button
          class="absolute right-2 top-1.5 text-slate-500 hover:text-white"
          on:click={() => (filterText = '')}
        >
          <X size={12} />
        </button>
      {/if}
    </div>

    <button
      class="p-1 rounded hover:bg-[#1f2533] text-slate-400 hover:text-white"
      on:click={() => loadChildren(item.path)}
      title="Ladda om mappinnehåll"
    >
      <RefreshCw size={12} class={isLoadingChildren ? 'animate-spin' : ''} />
    </button>
  </div>

  <!-- Content Quick Peek List -->
  <div class="flex-1 overflow-auto p-2 text-xs select-text">
    {#if isLoadingChildren}
      <div class="h-40 flex items-center justify-center text-slate-400 gap-2">
        <RefreshCw size={14} class="animate-spin text-[var(--accent)]" />
        <span>Läser in mappinnehåll...</span>
      </div>
    {:else if errorMessage}
      <div class="p-3 rounded-lg bg-red-950/30 border border-red-800 text-red-400 space-y-1">
        <span class="font-bold block">Kunde inte läsa mapp:</span>
        <p class="text-[11px] font-mono">{errorMessage}</p>
      </div>
    {:else if filteredChildren.length === 0}
      <div class="h-32 flex flex-col items-center justify-center text-slate-500 text-center">
        <Folder size={24} class="opacity-20 mb-1" />
        <span class="text-[11px]">{children.length === 0 ? 'Mappen är tom' : 'Inga matchande filer'}</span>
      </div>
    {:else}
      <div class="border border-[#252d3d] rounded-lg bg-[#0e1015] overflow-hidden shadow-inner">
        <table class="w-full text-left font-mono text-[11px] border-collapse">
          <thead>
            <tr class="border-b border-[#252d3d] bg-[#161a24] text-slate-400 text-[10px]">
              <th class="p-1.5 pl-2.5">Innehåll ({filteredChildren.length})</th>
              <th class="p-1.5 text-right w-16">Storlek</th>
              <th class="p-1.5 text-right pr-2.5 w-24">Ändrad</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredChildren as child}
              {@const iconInfo = getFileIcon(child)}
              <tr
                class="border-b border-[#1f2533]/50 hover:bg-[var(--bg-hover)] cursor-pointer group transition-colors"
                on:dblclick={() => handleNavigateToChild(child)}
                title="{child.name} - Dubbelklicka för att {child.is_dir ? 'öppna mapp' : 'öppna fil'}"
              >
                <td class="p-1.5 pl-2.5 flex items-center gap-1.5 truncate max-w-[180px]">
                  <svelte:component this={iconInfo.icon} size={13} class="{iconInfo.color} shrink-0" />
                  <span class="truncate {child.is_dir ? 'font-semibold text-white group-hover:text-[var(--accent)]' : 'text-slate-300'}">
                    {child.name}
                  </span>
                </td>
                <td class="p-1.5 text-right text-slate-400 font-mono text-[10px]">
                  {child.is_dir ? '--' : child.formatted_size}
                </td>
                <td class="p-1.5 text-right pr-2.5 text-slate-500 font-mono text-[10px]">
                  {child.formatted_modified.split(' ')[0]}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <!-- Recursive Disk Usage du -h Section -->
    <div class="mt-3 pt-3 border-t border-[#252d3d]/80 space-y-2">
      {#if dirSummary}
        <div class="p-2.5 rounded-lg bg-emerald-950/30 border border-emerald-800 text-emerald-300 space-y-1">
          <div class="font-bold flex items-center justify-between text-xs">
            <span>Rekursiv diskstorlek:</span>
            <span class="text-sm text-emerald-200">{dirSummary.formatted_total_size}</span>
          </div>
          <div class="text-[10.5px] text-emerald-400/80 flex justify-between font-mono">
            <span>{dirSummary.total_files} filer</span>
            <span>{dirSummary.total_dirs} undermappar</span>
          </div>
        </div>
      {:else}
        <button
          class="w-full flex items-center justify-center gap-2 px-3 py-1.5 rounded-lg bg-[#191d26] hover:bg-[#222836] border border-[#262d3d] text-slate-300 hover:text-white font-medium text-xs shadow-sm transition-colors disabled:opacity-50"
          disabled={isCalculatingDu}
          on:click={calculateFolderDu}
        >
          <PieChart size={13} class={isCalculatingDu ? 'animate-spin text-[var(--accent)]' : 'text-[var(--accent)]'} />
          <span>{isCalculatingDu ? 'Beräknar storlek (du -h)...' : 'Beräkna rekursiv storlek (du -h)'}</span>
        </button>
      {/if}
    </div>
  </div>
</div>
