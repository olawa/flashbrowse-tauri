<script lang="ts">
  import { onMount } from 'svelte';
  import { getPreview, calculateDirSize, revealInOs, openInDefault } from '$lib/invoke';
  import type { PreviewContent, DirectorySummary } from '$lib/types';
  import {
    FileText,
    Copy,
    FolderOpen,
    ExternalLink,
    PieChart,
    Check,
    RefreshCw,
  } from 'lucide-svelte';

  let filePath = '';
  let fileName = '';
  let preview: PreviewContent | null = null;
  let dirSummary: DirectorySummary | null = null;
  let isLoading = false;
  let isCalculatingDu = false;
  let copied = false;
  let viewMode: 'visual' | 'hex' | 'raw' = 'visual';

  onMount(() => {
    const urlParams = new URLSearchParams(window.location.search);
    const p = urlParams.get('path');
    if (p) {
      filePath = decodeURIComponent(p);
      fileName = filePath.split('/').filter(Boolean).pop() || filePath;
      loadPreview(filePath);
    }
  });

  async function loadPreview(targetPath: string) {
    isLoading = true;
    try {
      preview = await getPreview(targetPath);
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

  async function copyPath() {
    if (!filePath) return;
    await navigator.clipboard.writeText(filePath);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<div class="flex flex-col h-screen w-screen bg-[#0d0e11] text-[#f1f5f9] font-sans select-none overflow-hidden">
  <!-- Top Bar -->
  <div class="flex items-center justify-between px-4 py-2.5 bg-[#14171d] border-b border-[#262d3d]">
    <div class="flex items-center gap-2 min-w-0">
      <span class="px-2 py-0.5 rounded bg-[#e85422]/20 text-[#e85422] text-[11px] font-bold tracking-wider uppercase">
        Detached Inspector
      </span>
      <span class="font-bold text-sm text-white truncate">{fileName || 'Ingen fil vald'}</span>
    </div>

    {#if filePath}
      <div class="flex items-center gap-2">
        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs"
          on:click={() => loadPreview(filePath)}
        >
          <RefreshCw size={12} class={isLoading ? 'animate-spin' : ''} />
          <span>Ladda om</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs"
          on:click={() => openInDefault(filePath)}
        >
          <ExternalLink size={12} />
          <span>Öppna</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs"
          on:click={() => revealInOs(filePath)}
        >
          <FolderOpen size={12} />
          <span>Visa i Finder</span>
        </button>
      </div>
    {/if}
  </div>

  <!-- Content Area -->
  <div class="flex-1 overflow-auto bg-[#0a0b0e] p-4 font-mono text-xs select-text">
    {#if !filePath}
      <div class="h-full flex items-center justify-center text-slate-500">
        Ingen sökväg angiven
      </div>
    {:else if isLoading}
      <div class="h-full flex items-center justify-center text-slate-500">
        Läser in filinnehåll...
      </div>
    {:else if preview}
      {#if preview.kind === 'image' && preview.image_base64}
        <div class="h-full flex items-center justify-center bg-black/50 p-6 rounded-xl border border-[#262d3d]">
          <img
            src="data:{preview.image_mime || 'image/png'};base64,{preview.image_base64}"
            alt={fileName}
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
  <div class="px-4 py-2 bg-[#14171d] border-t border-[#262d3d] flex items-center justify-between text-xs font-mono text-slate-400">
    <div class="flex items-center gap-3 truncate">
      <span class="truncate max-w-[500px]" title={filePath}>{filePath}</span>
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
</div>
