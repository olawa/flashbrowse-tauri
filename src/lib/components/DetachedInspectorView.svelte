<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getPreview, calculateDirSize, revealInOs, openInDefault, toggleDetachedInspector } from '../invoke';
  import type { FileItem, PreviewContent, DirectorySummary } from '../types';
  import {
    FileText,
    Copy,
    FolderOpen,
    ExternalLink,
    PieChart,
    Check,
    RefreshCw,
    PanelRightClose,
  } from 'lucide-svelte';

  let currentItem: FileItem | null = null;
  let titlePrefix = 'Detached Inspector';
  let preview: PreviewContent | null = null;
  let dirSummary: DirectorySummary | null = null;
  let isLoading = false;
  let isCalculatingDu = false;
  let copied = false;
  let unlistenSync: (() => void) | null = null;

  onMount(async () => {
    // 1. Check if an initial path was provided in URL query
    const urlParams = new URLSearchParams(window.location.search);
    const p = urlParams.get('path');
    if (p) {
      const decoded = decodeURIComponent(p);
      const name = decoded.split('/').filter(Boolean).pop() || decoded;
      currentItem = {
        name,
        path: decoded,
        is_dir: false,
        is_symlink: false,
        size_bytes: 0,
        formatted_size: '--',
        modified_timestamp: 0,
        formatted_modified: '--',
        extension: name.split('.').pop() || '',
        is_hidden: false,
        permissions: '---------',
      };
      await loadPreview(decoded);
    }

    // 2. Listen to live sync events from the main window!
    try {
      unlistenSync = await listen<{ item: FileItem; titlePrefix: string }>('inspector-sync', async (event) => {
        if (event.payload && event.payload.item) {
          currentItem = event.payload.item;
          if (event.payload.titlePrefix) {
            titlePrefix = event.payload.titlePrefix;
          }
          await loadPreview(currentItem.path);
        }
      });
    } catch (e) {
      console.error('Failed to listen to inspector-sync:', e);
    }
  });

  onDestroy(() => {
    if (unlistenSync) {
      unlistenSync();
    }
  });

  async function loadPreview(path: string) {
    if (!path) return;
    isLoading = true;
    dirSummary = null;
    try {
      preview = await getPreview(path);
    } catch (e: any) {
      preview = {
        kind: 'error',
        file_size_bytes: 0,
        formatted_size: '--',
        modified_str: '--',
        permissions_str: '---------',
        error_message: String(e),
      };
    } finally {
      isLoading = false;
    }
  }

  async function calculateFolderDu() {
    if (!currentItem) return;
    isCalculatingDu = true;
    try {
      dirSummary = await calculateDirSize(currentItem.path);
    } catch (e: any) {
      alert(`Failed to calculate folder size: ${e}`);
    } finally {
      isCalculatingDu = false;
    }
  }

  async function copyPath() {
    if (!currentItem) return;
    await navigator.clipboard.writeText(currentItem.path);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  async function reattach() {
    await toggleDetachedInspector();
  }
</script>

<div class="flex flex-col h-screen w-screen bg-[#0d0e11] text-[#f1f5f9] font-sans select-none overflow-hidden">
  <!-- Top Bar -->
  <div class="flex items-center justify-between px-4 py-2.5 bg-[#14171d] border-b border-[#262d3d]">
    <div class="flex items-center gap-2 min-w-0">
      <span class="px-2 py-0.5 rounded bg-[#e85422]/20 text-[#e85422] text-[11px] font-bold tracking-wider uppercase">
        {titlePrefix}
      </span>
      <span class="font-bold text-sm text-white truncate">
        {currentItem ? currentItem.name : 'Ingen fil vald'}
      </span>
    </div>

    {#if currentItem}
      <div class="flex items-center gap-1.5">
        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs text-slate-300 hover:text-white"
          on:click={() => loadPreview(currentItem ? currentItem.path : '')}
          title="Ladda om förhandsvisning"
        >
          <RefreshCw size={12} class={isLoading ? 'animate-spin' : ''} />
          <span>Ladda om</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs text-slate-300 hover:text-white"
          on:click={() => openInDefault(currentItem ? currentItem.path : '')}
          title="Öppna i standardprogram"
        >
          <ExternalLink size={12} />
          <span>Öppna</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs text-slate-300 hover:text-white"
          on:click={() => revealInOs(currentItem ? currentItem.path : '')}
          title="Visa i Finder"
        >
          <FolderOpen size={12} />
          <span>Finder</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#e85422]/20 hover:bg-[#e85422] text-[#e85422] hover:text-white border border-[#e85422]/40 text-xs font-medium ml-1 transition-colors"
          on:click={reattach}
          title="Stäng detta fönster och visa i huvudfönstret igen"
        >
          <PanelRightClose size={12} />
          <span>Fäst tillbaka</span>
        </button>
      </div>
    {/if}
  </div>

  <!-- Content Body -->
  <div class="flex-1 overflow-auto bg-[#0a0b0e] p-4 font-mono text-xs select-text">
    {#if !currentItem}
      <div class="h-full flex flex-col items-center justify-center text-slate-500">
        <FileText size={40} class="opacity-20 mb-3" />
        <span>Väntar på filmarkering i Flashbrowse...</span>
      </div>
    {:else if isLoading}
      <div class="h-full flex items-center justify-center text-slate-500">
        Läser in filinnehåll...
      </div>
    {:else if preview}
      {#if preview.kind === 'directory'}
        <div class="max-w-md mx-auto mt-8 p-6 rounded-2xl bg-[#14171d] border border-[#262d3d] space-y-4 text-center">
          <div class="w-16 h-16 rounded-2xl bg-amber-500/10 text-amber-400 flex items-center justify-center mx-auto text-2xl">
            📁
          </div>
          <div>
            <h3 class="font-bold text-lg text-white">{currentItem.name}</h3>
            <p class="text-xs text-slate-400 mt-1 font-mono">{currentItem.path}</p>
          </div>

          {#if dirSummary}
            <div class="p-4 rounded-xl bg-emerald-950/30 border border-emerald-800 text-emerald-300 text-left space-y-2">
              <div class="flex justify-between font-bold text-sm">
                <span>Total storlek:</span>
                <span>{dirSummary.formatted_total_size}</span>
              </div>
              <div class="flex justify-between text-xs text-emerald-400/80">
                <span>Filer: {dirSummary.total_files}</span>
                <span>Mappar: {dirSummary.total_dirs}</span>
              </div>
            </div>
          {:else}
            <button
              class="w-full flex items-center justify-center gap-2 py-2.5 rounded-xl bg-[#e85422] hover:bg-[#ff6b35] text-white font-semibold transition-all shadow-lg disabled:opacity-50"
              disabled={isCalculatingDu}
              on:click={calculateFolderDu}
            >
              <PieChart size={15} />
              <span>{isCalculatingDu ? 'Beräknar storlek...' : 'Beräkna rekursiv mappstorlek (du -h)'}</span>
            </button>
          {/if}
        </div>
      {:else if preview.kind === 'image' && preview.image_base64}
        <div class="h-full flex items-center justify-center bg-black/50 p-6 rounded-xl border border-[#262d3d]">
          <img
            src="data:{preview.image_mime || 'image/png'};base64,{preview.image_base64}"
            alt={currentItem.name}
            class="max-h-[85vh] max-w-full object-contain rounded-lg shadow-2xl"
          />
        </div>
      {:else if preview.kind === 'table' && preview.table_headers && preview.table_rows}
        <div class="overflow-x-auto rounded-lg border border-[#262d3d] bg-[#14171d]">
          <table class="w-full text-left border-collapse text-xs">
            <thead>
              <tr class="border-b border-[#262d3d] bg-[#191d24]">
                {#each preview.table_headers as header}
                  <th class="p-2 font-bold text-[#e85422]">{header}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each preview.table_rows as row}
                <tr class="border-b border-[#262d3d]/50 hover:bg-white/5">
                  {#each row as cell}
                    <td class="p-2 text-slate-300">{cell}</td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else if preview.kind === 'code' || preview.kind === 'text'}
        <div class="p-4 rounded-xl bg-[#14171d] border border-[#262d3d] leading-relaxed text-slate-200">
          <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
        </div>
      {:else if preview.kind === 'hex' && preview.hex_lines}
        <div class="p-4 rounded-xl bg-[#14171d] border border-[#262d3d] text-purple-300 leading-tight">
          {#each preview.hex_lines as line}
            <div>{line}</div>
          {/each}
        </div>
      {:else if preview.kind === 'error'}
        <div class="p-8 text-center text-red-400">
          {preview.error_message || 'Kunde inte läsa fil'}
        </div>
      {/if}
    {/if}
  </div>

  <!-- Bottom Metadata Bar -->
  {#if currentItem}
    <div class="px-4 py-2 bg-[#14171d] border-t border-[#262d3d] flex items-center justify-between text-xs font-mono text-slate-400">
      <div class="flex items-center gap-3 truncate">
        <span class="truncate max-w-[500px]" title={currentItem.path}>{currentItem.path}</span>
        <button
          class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#262d3d] hover:bg-[#e85422] hover:text-white text-slate-200 transition-colors"
          on:click={copyPath}
        >
          {#if copied}
            <Check size={11} class="text-green-400" />
            <span>Kopierad</span>
          {:else}
            <Copy size={11} />
            <span>Kopiera sökväg</span>
          {/if}
        </button>
      </div>

      {#if preview}
        <div class="flex items-center gap-4 text-slate-300">
          <span>Storlek: <strong class="text-white">{preview.formatted_size}</strong></span>
          <span>Ändrad: <strong>{preview.modified_str}</strong></span>
          <span>Rättigheter: <strong>{preview.permissions_str}</strong></span>
        </div>
      {/if}
    </div>
  {/if}
</div>
