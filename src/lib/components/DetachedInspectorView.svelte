<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getPreview, calculateDirSize, revealInOs, openInDefault, toggleDetachedInspector } from '../invoke';
  import { renderMarkdown } from '../markdown';
  import BioInspector from './BioInspector.svelte';
  import ArchiveInspector from './ArchiveInspector.svelte';
  import FolderInspector from './FolderInspector.svelte';
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
    Volume2,
    Video,
    Rocket,
    Sparkles,
  } from 'lucide-svelte';

  let currentItem: FileItem | null = null;
  let titlePrefix = 'Detached Inspector';
  let preview: PreviewContent | null = null;
  let dirSummary: DirectorySummary | null = null;
  let isLoading = false;
  let isCalculatingDu = false;
  let copied = false;
  let unlistenSync: (() => void) | null = null;
  let unlistenPathSync: (() => void) | null = null;
  let unlistenCastSync: (() => void) | null = null;
  let castAlert = false;

  // View Mode Toggles
  let htmlViewMode: 'rendered' | 'source' = 'rendered';
  let pdfViewMode: 'pdf' | 'hex' = 'pdf';
  let mdViewMode: 'rendered' | 'source' = 'rendered';
  let svgViewMode: 'rendered' | 'source' = 'rendered';

  $: ext = currentItem?.extension.toLowerCase() || '';
  $: isBam = !!currentItem && (ext === 'bam' || ext === 'cram' || ext === 'sam' || currentItem.name.endsWith('.bam') || currentItem.name.endsWith('.cram'));
  $: isArchive = !!currentItem && (ext === 'zip' || ext === 'tar' || ext === 'tgz' || currentItem.name.endsWith('.tar.gz') || currentItem.name.endsWith('.tar.bz2') || currentItem.name.endsWith('.tar.xz'));

  function setItemFromPath(decodedPath: string, wasCast = false) {
    const name = decodedPath.split('/').filter(Boolean).pop() || decodedPath;
    currentItem = {
      name,
      path: decodedPath,
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
    if (wasCast) {
      castAlert = true;
      setTimeout(() => (castAlert = false), 2500);
    }
    loadPreview(decodedPath);
  }

  onMount(async () => {
    // 1. Check if an initial path was provided in URL query
    const urlParams = new URLSearchParams(window.location.search);
    const p = urlParams.get('path');
    if (p) {
      setItemFromPath(decodeURIComponent(p));
    }

    // 2. Listen to live sync & cast events from the main window!
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

      unlistenPathSync = await listen<string>('inspector-sync-path', async (event) => {
        if (event.payload) {
          setItemFromPath(event.payload);
        }
      });

      unlistenCastSync = await listen<string>('inspector-cast-path', async (event) => {
        if (event.payload) {
          setItemFromPath(event.payload, true);
        }
      });
    } catch (e) {
      console.error('Failed to listen to inspector sync events:', e);
    }
  });

  onDestroy(() => {
    if (unlistenSync) unlistenSync();
    if (unlistenPathSync) unlistenPathSync();
    if (unlistenCastSync) unlistenCastSync();
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

  async function copyPath() {
    if (!currentItem) return;
    try {
      await navigator.clipboard.writeText(currentItem.path);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (err) {
      console.warn('Clipboard write failed:', err);
    }
  }

  async function reattach() {
    await toggleDetachedInspector();
  }
</script>

<div class="flex flex-col h-screen w-screen bg-[#0d0e11] text-[#f1f5f9] font-sans select-none overflow-hidden relative">
  <!-- Cast Toast Banner -->
  {#if castAlert}
    <div class="absolute top-12 left-1/2 -translate-x-1/2 z-50 px-4 py-1.5 rounded-full bg-emerald-600 text-white text-xs font-bold shadow-2xl flex items-center gap-2 animate-bounce border border-emerald-400">
      <Rocket size={14} />
      <span>Fil kastad hit från filtabellen!</span>
    </div>
  {/if}

  <!-- Top Bar -->
  <div class="flex items-center justify-between px-4 py-2.5 bg-[#14171d] border-b border-[#262d3d] shrink-0">
    <div class="flex items-center gap-2.5 min-w-0">
      <div class="flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-[#e85422]/20 text-[#e85422] border border-[#e85422]/40 text-[11px] font-bold tracking-wider uppercase">
        <Sparkles size={11} />
        <span>Stora Inspektorn</span>
      </div>
      <span class="font-bold text-sm text-white truncate max-w-lg select-text" title={currentItem ? currentItem.path : ''}>
        {currentItem ? currentItem.name : 'Väntar på fil...'}
      </span>
      {#if currentItem?.extension}
        <span class="px-1.5 py-0.2 rounded bg-slate-800 text-slate-300 text-[10px] font-mono border border-slate-700">
          {currentItem.extension.toUpperCase()}
        </span>
      {/if}
    </div>

    {#if currentItem}
      <div class="flex items-center gap-2">
        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs text-slate-300 hover:text-white transition-colors"
          on:click={() => loadPreview(currentItem ? currentItem.path : '')}
          title="Ladda om filinnehåll"
        >
          <RefreshCw size={12} class={isLoading ? 'animate-spin' : ''} />
          <span>Ladda om</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs text-slate-300 hover:text-white transition-colors"
          on:click={() => openInDefault(currentItem ? currentItem.path : '')}
          title="Öppna i standardprogram"
        >
          <ExternalLink size={12} />
          <span>Öppna</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#191d24] hover:bg-[#222834] border border-[#262d3d] text-xs text-slate-300 hover:text-white transition-colors"
          on:click={() => revealInOs(currentItem ? currentItem.path : '')}
          title="Visa i Finder"
        >
          <FolderOpen size={12} />
          <span>Finder</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#e85422]/20 hover:bg-[#e85422] text-[#e85422] hover:text-white border border-[#e85422]/40 text-xs font-medium ml-1 transition-colors"
          on:click={reattach}
          title="Stäng detta fönster"
        >
          <PanelRightClose size={12} />
          <span>Stäng fönster</span>
        </button>
      </div>
    {/if}
  </div>

  <!-- Content Body -->
  <div class="flex-1 overflow-auto bg-[#0a0b0e] flex flex-col font-mono text-xs select-text">
    {#if !currentItem}
      <div class="h-full flex flex-col items-center justify-center text-slate-500">
        <FileText size={40} class="opacity-20 mb-3" />
        <span>Väntar på att filer markeras eller sveps uppåt i Flashbrowse...</span>
      </div>
    {:else if isBam}
      <BioInspector item={currentItem} />
    {:else if isArchive}
      <ArchiveInspector item={currentItem} />
    {:else if isLoading}
      <div class="h-full flex items-center justify-center text-slate-500">
        Läser in filinnehåll...
      </div>
    {:else if currentItem.is_dir || (preview && preview.kind === 'directory')}
      <FolderInspector item={currentItem} />
    {:else if preview}
      <!-- 1. HTML REPORT PREVIEW (MultiQC / FastQC) -->
      {#if preview.kind === 'html' && preview.html_content}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-4 py-1.5 bg-[#161a24] border-b border-[#252d3d] text-xs">
            <div class="flex items-center gap-1.5">
              <button
                class="px-3 py-1 rounded font-medium transition-colors {htmlViewMode === 'rendered' ? 'bg-[#e85422] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (htmlViewMode = 'rendered')}
              >
                🌐 Renderad rapport
              </button>
              <button
                class="px-3 py-1 rounded font-medium transition-colors {htmlViewMode === 'source' ? 'bg-[#e85422] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (htmlViewMode = 'source')}
              >
                📄 Källkod
              </button>
            </div>
            <button
              class="text-slate-400 hover:text-white flex items-center gap-1 text-xs"
              on:click={() => currentItem && openInDefault(currentItem.path)}
            >
              <ExternalLink size={12} />
              <span>Öppna i webbläsare</span>
            </button>
          </div>

          {#if htmlViewMode === 'rendered'}
            <div class="flex-1 bg-white min-h-[400px]">
              <iframe
                srcdoc={preview.html_content}
                title={currentItem.name}
                class="w-full h-full border-0 bg-white"
                sandbox="allow-scripts allow-same-origin allow-popups"
              ></iframe>
            </div>
          {:else}
            <div class="p-4 font-mono text-xs leading-relaxed text-slate-200 overflow-auto select-text">
              <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
            </div>
          {/if}
        </div>

      <!-- 2. PDF DOCUMENT PREVIEW -->
      {:else if preview.kind === 'pdf' && preview.pdf_base64}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-4 py-1.5 bg-[#161a24] border-b border-[#252d3d] text-xs">
            <div class="flex items-center gap-1.5">
              <button
                class="px-3 py-1 rounded font-medium transition-colors {pdfViewMode === 'pdf' ? 'bg-[#e85422] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (pdfViewMode = 'pdf')}
              >
                📄 PDF-visning
              </button>
              <button
                class="px-3 py-1 rounded font-medium transition-colors {pdfViewMode === 'hex' ? 'bg-[#e85422] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (pdfViewMode = 'hex')}
              >
                🔢 Hex-dump
              </button>
            </div>
            <button
              class="text-slate-400 hover:text-white flex items-center gap-1 text-xs"
              on:click={() => currentItem && openInDefault(currentItem.path)}
            >
              <ExternalLink size={12} />
              <span>Öppna i PDF-läsare</span>
            </button>
          </div>

          {#if pdfViewMode === 'pdf'}
            <div class="flex-1 bg-slate-900 min-h-[400px]">
              <iframe
                src="data:application/pdf;base64,{preview.pdf_base64}#toolbar=1"
                title={currentItem.name}
                class="w-full h-full border-0 min-h-[400px]"
              ></iframe>
            </div>
          {:else if preview.hex_lines}
            <div class="p-4 font-mono text-xs text-purple-300 leading-tight select-text overflow-auto">
              {#each preview.hex_lines as line}
                <div>{line}</div>
              {/each}
            </div>
          {/if}
        </div>

      <!-- 3. MARKDOWN PREVIEW -->
      {:else if preview.kind === 'markdown' && preview.text_content}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-4 py-1.5 bg-[#161a24] border-b border-[#252d3d] text-xs">
            <div class="flex items-center gap-1.5">
              <button
                class="px-3 py-1 rounded font-medium transition-colors {mdViewMode === 'rendered' ? 'bg-[#e85422] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (mdViewMode = 'rendered')}
              >
                📖 Formaterad
              </button>
              <button
                class="px-3 py-1 rounded font-medium transition-colors {mdViewMode === 'source' ? 'bg-[#e85422] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (mdViewMode = 'source')}
              >
                📝 Råtext
              </button>
            </div>
          </div>

          {#if mdViewMode === 'rendered'}
            <div class="p-6 text-sm select-text overflow-auto space-y-3 leading-relaxed max-w-4xl mx-auto w-full">
              {@html renderMarkdown(preview.text_content)}
            </div>
          {:else}
            <div class="p-4 font-mono text-xs leading-relaxed text-slate-200 overflow-auto select-text">
              <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
            </div>
          {/if}
        </div>

      <!-- 4. VIDEO PREVIEW -->
      {:else if preview.kind === 'video' && preview.media_base64}
        <div class="flex-1 flex flex-col items-center justify-center p-8 bg-black/60">
          <video
            src="data:{preview.media_mime || 'video/mp4'};base64,{preview.media_base64}"
            controls
            class="max-h-[75vh] max-w-4xl rounded-xl shadow-2xl border border-[#252d3d]"
          >
            <track kind="captions" />
          </video>
        </div>

      <!-- 5. AUDIO PREVIEW -->
      {:else if preview.kind === 'audio' && preview.media_base64}
        <div class="flex-1 flex flex-col items-center justify-center p-12 bg-[#12151c]">
          <div class="w-16 h-16 rounded-full bg-emerald-500/20 text-emerald-400 flex items-center justify-center mb-4">
            <Volume2 size={32} />
          </div>
          <h3 class="font-bold text-lg text-white mb-2">{currentItem.name}</h3>
          <audio
            src="data:{preview.media_mime || 'audio/mpeg'};base64,{preview.media_base64}"
            controls
            class="w-96 mb-2"
          ></audio>
          <span class="text-xs text-slate-400">{preview.formatted_size}</span>
        </div>

      <!-- 6. SVG PREVIEW -->
      {:else if preview.kind === 'svg'}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-4 py-1.5 bg-[#161a24] border-b border-[#252d3d] text-xs">
            <div class="flex items-center gap-1.5">
              <button
                class="px-3 py-1 rounded font-medium transition-colors {svgViewMode === 'rendered' ? 'bg-[#e85422] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (svgViewMode = 'rendered')}
              >
                🎨 Vektorbild
              </button>
              <button
                class="px-3 py-1 rounded font-medium transition-colors {svgViewMode === 'source' ? 'bg-[#e85422] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (svgViewMode = 'source')}
              >
                📄 XML-kod
              </button>
            </div>
          </div>

          {#if svgViewMode === 'rendered' && preview.image_base64}
            <div class="flex-1 flex items-center justify-center p-8 bg-black/40">
              <img
                src="data:image/svg+xml;base64,{preview.image_base64}"
                alt={currentItem.name}
                class="max-h-[80vh] max-w-full object-contain rounded shadow-2xl"
              />
            </div>
          {:else if preview.text_content}
            <div class="p-4 font-mono text-xs leading-relaxed text-slate-200 overflow-auto select-text">
              <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
            </div>
          {/if}
        </div>

      <!-- 7. IMAGE PREVIEW -->
      {:else if preview.kind === 'image' && preview.image_base64}
        <div class="flex-1 flex items-center justify-center bg-black/50 p-6">
          <img
            src="data:{preview.image_mime || 'image/png'};base64,{preview.image_base64}"
            alt={currentItem.name}
            class="max-h-[85vh] max-w-full object-contain rounded-lg shadow-2xl"
          />
        </div>

      <!-- 8. CSV / TSV TABLE PREVIEW -->
      {:else if preview.kind === 'table' && preview.table_headers && preview.table_rows}
        <div class="flex-1 overflow-auto p-4">
          <div class="rounded-lg border border-[#262d3d] bg-[#14171d] overflow-hidden">
            <table class="w-full text-left border-collapse text-xs">
              <thead>
                <tr class="border-b border-[#262d3d] bg-[#191d24]">
                  {#each preview.table_headers as header}
                    <th class="p-2.5 font-bold text-[#e85422]">{header}</th>
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
        </div>

      <!-- 9. TEXT & CODE PREVIEW -->
      {:else if preview.kind === 'code' || preview.kind === 'text'}
        <div class="p-4 rounded-xl bg-[#14171d] border border-[#262d3d] m-4 leading-relaxed text-slate-200">
          <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
        </div>

      <!-- 10. BINARY HEX PREVIEW -->
      {:else if preview.kind === 'hex' && preview.hex_lines}
        <div class="p-4 rounded-xl bg-[#14171d] border border-[#262d3d] m-4 text-purple-300 leading-tight">
          {#each preview.hex_lines as line}
            <div>{line}</div>
          {/each}
        </div>

      <!-- ERROR / TOO LARGE -->
      {:else if preview.kind === 'error' || preview.kind === 'too_large'}
        <div class="p-8 text-center text-amber-400">
          {preview.error_message || 'Kunde inte läsa fil'}
        </div>
      {/if}
    {/if}
  </div>

  <!-- Bottom Metadata Bar -->
  {#if currentItem}
    <div class="px-4 py-2 bg-[#14171d] border-t border-[#262d3d] flex items-center justify-between text-xs font-mono text-slate-400 shrink-0">
      <div class="flex items-center gap-3 truncate">
        <span class="truncate max-w-[500px]" title={currentItem.path}>{currentItem.path}</span>
        <button
          class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#262d3d] hover:bg-[#e85422] hover:text-white text-slate-200 transition-colors"
          on:click={copyPath}
        >
          {#if copied}
            <Check size={12} class="text-green-400" />
            <span>Kopierad</span>
          {:else}
            <Copy size={12} />
            <span>Kopiera</span>
          {/if}
        </button>
      </div>

      <div class="flex items-center gap-4 shrink-0">
        <span>{currentItem.formatted_size}</span>
        <span>{currentItem.formatted_modified}</span>
      </div>
    </div>
  {/if}
</div>
