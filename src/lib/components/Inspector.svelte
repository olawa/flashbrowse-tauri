<script lang="ts">
  import { onMount } from 'svelte';
  import { emit } from '@tauri-apps/api/event';
  import { getPreview, sshGetPreview, calculateDirSize, revealInOs, openInDefault, toggleDetachedInspector } from '../invoke';
  import {
    isInspectorDetached,
    castToSecondaryInspector,
    inspectorScroll,
    isInspectorLocked,
    toggleInspectorLock,
  } from '../stores/navigation';
  import { renderMarkdown } from '../markdown';
  import BioInspector from './BioInspector.svelte';
  import ArchiveInspector from './ArchiveInspector.svelte';
  import FolderInspector from './FolderInspector.svelte';
  import CodeViewer from './CodeViewer.svelte';
  import SpreadsheetViewer from './SpreadsheetViewer.svelte';
  import NotebookViewer from './NotebookViewer.svelte';
  import AIInspector from './AIInspector.svelte';
  import NotesInspector from './NotesInspector.svelte';
  import MultiItemInspector from './MultiItemInspector.svelte';
  import { leftPane, rightPane, activePaneId, activeHoveredItem } from '../stores/navigation';
  import { isOllamaOnline } from '../stores/ollamaStore';
  import type { FileItem, PreviewContent, DirectorySummary } from '../types';
  import {
    FileText,
    Image as ImageIcon,
    Table,
    Binary,
    Copy,
    FolderOpen,
    ExternalLink,
    PieChart,
    Check,
    SquareArrowOutUpRight,
    Rocket,
    Globe,
    FileSpreadsheet,
    Volume2,
    Video,
    Sparkles,
    Lock,
    Unlock,
    NotebookPen,
    Download,
    Settings,
  } from 'lucide-svelte';
  import {
    downloadDirectory,
    isSavingFile,
    saveRemoteOrLocalItem,
    setDownloadDirectory,
    initDownloadStore,
  } from '../stores/downloadStore';

  export let item: FileItem | null = null;
  export let titlePrefix = 'Inspector';

  let preview: PreviewContent | null = null;
  let dirSummary: DirectorySummary | null = null;
  let isLoading = false;
  let isCalculatingDu = false;
  let copied = false;
  let castedAnimation = false;
  let inspectorBodyEl: HTMLElement;
  let htmlIframeEl: HTMLIFrameElement;
  let pdfIframeEl: HTMLIFrameElement;
  let codeViewerRef: any;
  let lastScrollPulse = 0;

  // View Mode Toggles
  let htmlViewMode: 'rendered' | 'source' = 'rendered';
  let pdfViewMode: 'pdf' | 'hex' = 'pdf';
  let mdViewMode: 'rendered' | 'source' = 'rendered';
  let svgViewMode: 'rendered' | 'source' = 'rendered';
  let showHexDump = false;
  let currentTab: 'preview' | 'notes' | 'ai' = 'preview';

  $: activePaneState = $activePaneId === 'left' ? $leftPane : $rightPane;
  $: selectedItems = activePaneState.items.filter((i) => activePaneState.selectedPaths.has(i.path));
  $: isMultiSelecting = selectedItems.length > 1 && !$isInspectorLocked && !$activeHoveredItem;

  function handleDeselectItem(path: string) {
    const store = $activePaneId === 'left' ? leftPane : rightPane;
    store.update((s) => {
      const next = new Set(s.selectedPaths);
      next.delete(path);
      return { ...s, selectedPaths: next };
    });
  }

  function handleClearSelection() {
    const store = $activePaneId === 'left' ? leftPane : rightPane;
    store.update((s) => ({ ...s, selectedPaths: new Set() }));
  }

  $: activeNotesDirectory = item
    ? (item.is_dir ? item.path : (item.path.substring(0, item.path.lastIndexOf('/')) || '/'))
    : ($activePaneId === 'left' ? $leftPane.currentPath : $rightPane.currentPath) || '/';

  // Remote Two-Finger Scroll listener
  $: if ($inspectorScroll.pulse && $inspectorScroll.pulse !== lastScrollPulse) {
    lastScrollPulse = $inspectorScroll.pulse;
    handleRemoteScroll($inspectorScroll.deltaY);
  }

  function handleRemoteScroll(deltaY: number) {
    // 1. If HTML or PDF iframe is visible, scroll it
    if (htmlViewMode === 'rendered' && htmlIframeEl?.contentWindow) {
      try {
        htmlIframeEl.contentWindow.scrollBy({ top: deltaY, left: 0, behavior: 'auto' });
        return;
      } catch {}
    }
    if (pdfViewMode === 'pdf' && pdfIframeEl?.contentWindow) {
      try {
        pdfIframeEl.contentWindow.scrollBy({ top: deltaY, left: 0, behavior: 'auto' });
        return;
      } catch {}
    }

    // 2. If CodeViewer is active, scroll its inner container
    if (codeViewerRef?.scrollByDelta) {
      codeViewerRef.scrollByDelta(deltaY);
      return;
    }

    // 3. Otherwise scroll the innermost scrollable container inside the inspector body
    if (inspectorBodyEl) {
      const scrollable =
        inspectorBodyEl.querySelector('.overflow-y-auto, .overflow-auto') ||
        inspectorBodyEl;
      scrollable.scrollBy({ top: deltaY, behavior: 'auto' });
    }
  }

  $: ext = item?.extension.toLowerCase() || '';
  $: isBam = !!item && (ext === 'bam' || ext === 'cram' || ext === 'sam' || item.name.endsWith('.bam') || item.name.endsWith('.cram'));
  $: isArchive = !!item && (ext === 'zip' || ext === 'tar' || ext === 'tgz' || item.name.endsWith('.tar.gz') || item.name.endsWith('.tar.bz2') || item.name.endsWith('.tar.xz'));

  $: if (item) {
    if (!isBam && !isArchive) {
      loadItemPreview(item);
    } else {
      preview = null;
    }
    try {
      emit('inspector-sync', { item, titlePrefix });
    } catch {}
  } else {
    preview = null;
    dirSummary = null;
  }

  async function handleDetach() {
    isInspectorDetached.set(true);
    await toggleDetachedInspector(item?.path);
  }

  async function handleCastToLarge() {
    if (!item) return;
    castedAnimation = true;
    setTimeout(() => (castedAnimation = false), 1500);
    await castToSecondaryInspector(item);
  }

  async function loadItemPreview(target: FileItem) {
    isLoading = true;
    dirSummary = null;
    try {
      if (target.is_dir) {
        preview = null;
      } else if (activePaneState.isSSH) {
        preview = await sshGetPreview(activePaneState.sshHost, target.path);
      } else {
        preview = await getPreview(target.path);
      }
    } catch (e: any) {
      preview = {
        kind: 'error',
        file_size_bytes: target.size_bytes,
        formatted_size: target.formatted_size,
        modified_str: target.formatted_modified,
        permissions_str: target.permissions,
        error_message: String(e),
      };
    } finally {
      isLoading = false;
    }
  }

  async function calculateFolderDu() {
    if (!item || !item.is_dir) return;
    isCalculatingDu = true;
    try {
      dirSummary = await calculateDirSize(item.path);
    } catch (e: any) {
      alert(`Failed to calculate size: ${e}`);
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

  let isSaveSuccess = false;
  let isDownloadConfigOpen = false;
  let customDownloadInput = '';

  onMount(() => {
    initDownloadStore();
  });

  async function handleSavePermanent() {
    if (!item) return;
    const res = await saveRemoteOrLocalItem(
      activePaneState.isSSH,
      activePaneState.sshHost,
      item.path
    );
    if (res.success) {
      isSaveSuccess = true;
      setTimeout(() => (isSaveSuccess = false), 2500);
    }
  }

  function truncateMiddle(str: string, maxLen = 32): string {
    if (!str || str.length <= maxLen) return str;
    const half = Math.floor((maxLen - 3) / 2);
    return str.slice(0, half) + '...' + str.slice(str.length - half);
  }
</script>

<div class="w-full h-full flex flex-col bg-[var(--bg-surface)] text-xs select-none relative overflow-hidden">
  <!-- Inspector Header -->
  <div class="flex items-start justify-between px-3 py-2 border-b border-[var(--border)] bg-[var(--bg-panel)] gap-2 shrink-0">
    <div class="flex flex-col min-w-0 flex-1">
      <div class="flex items-center gap-1.5">
        <span class="text-[10px] font-bold tracking-wider text-[var(--accent)] uppercase">{titlePrefix}</span>
        {#if $isInspectorLocked}
          <span class="px-1.5 py-0.2 rounded bg-amber-500/20 text-amber-400 font-mono text-[9px] font-bold tracking-wide flex items-center gap-1">
            <Lock size={9} /> LÅST
          </span>
        {/if}
        <span class="text-[var(--text-muted)]">•</span>
        <span class="text-[10px] text-[var(--text-secondary)] font-mono">{item?.extension ? item.extension.toUpperCase() : item?.is_dir ? 'MAPP' : ''}</span>
      </div>
      <span class="font-bold text-xs text-[var(--text-primary)] break-all leading-snug select-text mt-0.5" title={item?.path}>
        {item ? item.name : 'Ingen markering'}
      </span>
    </div>

    <div class="flex items-center gap-1 shrink-0">
      <!-- Tab Toggles: Preview | Anteckningar | AI -->
      <div class="flex items-center gap-0.5 bg-[var(--bg-surface)] p-0.5 rounded-lg border border-[var(--border)] mr-0.5">
        <button
          class="px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors {currentTab === 'preview' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'}"
          on:click={() => (currentTab = 'preview')}
          title="Visa standard filförhandsgranskning"
        >
          Förhandsgranska
        </button>

        <button
          class="px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors flex items-center gap-1 {currentTab === 'notes' ? 'bg-amber-600 text-white font-bold' : 'text-[var(--text-secondary)] hover:text-amber-400'}"
          on:click={() => (currentTab = 'notes')}
          title="Mappens anteckningar & lab notebook (NOTES.md)"
        >
          <NotebookPen size={11} />
          <span>Anteckningar</span>
        </button>

        <button
          class="px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors flex items-center gap-1 {currentTab === 'ai' ? 'bg-purple-600 text-white font-bold' : 'text-[var(--text-secondary)] hover:text-purple-400'}"
          on:click={() => (currentTab = 'ai')}
          title="Öppna Lokal AI-Assistent (Ollama)"
        >
          <Sparkles size={11} class={currentTab === 'ai' ? 'animate-spin' : ''} />
          <span>AI</span>
        </button>
      </div>

      {#if item}
        <!-- Save Permanent / Download to Mac -->
        <div class="relative flex items-center">
          <button
            class="flex items-center gap-1 px-2 py-0.5 rounded border text-[11px] font-semibold transition-all {isSaveSuccess ? 'bg-emerald-600 text-white border-emerald-500 shadow-sm' : 'bg-cyan-950/40 hover:bg-cyan-900/60 text-cyan-300 border-cyan-800/80 hover:text-white'}"
            on:click={handleSavePermanent}
            disabled={$isSavingFile}
            title="Spara permanent lokal kopia till {$downloadDirectory || '~/Downloads'}"
          >
            {#if $isSavingFile}
              <div class="w-2.5 h-2.5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
              <span>Sparar...</span>
            {:else if isSaveSuccess}
              <Check size={11} class="text-white" />
              <span>Sparad!</span>
            {:else}
              <Download size={11} class="text-cyan-400" />
              <span>Spara {activePaneState.isSSH ? 'till Mac' : 'kopia'}</span>
            {/if}
          </button>

          <button
            class="p-1 ml-0.5 rounded hover:bg-[var(--bg-hover)] text-slate-400 hover:text-white"
            on:click={() => {
              customDownloadInput = $downloadDirectory;
              isDownloadConfigOpen = !isDownloadConfigOpen;
            }}
            title="Ändra Downloads-mapp (Nu: {$downloadDirectory || '~/Downloads'})"
          >
            <Settings size={11} />
          </button>

          <!-- Download Directory Popover -->
          {#if isDownloadConfigOpen}
            <div
              class="absolute right-0 top-full mt-1 w-64 p-3 bg-[var(--bg-surface)] border border-[var(--border)] rounded-lg shadow-2xl z-50 text-xs text-[var(--text-primary)]"
              on:click|stopPropagation
            >
              <div class="font-bold text-[11px] mb-1 text-slate-300">Downloads-katalog:</div>
              <input
                type="text"
                bind:value={customDownloadInput}
                placeholder="t.ex. /Users/olwal516/Downloads"
                class="w-full bg-[var(--bg-panel)] text-xs text-[var(--text-primary)] px-2 py-1 rounded border border-[var(--border)] focus:border-[var(--accent)] focus:outline-none font-mono mb-2"
              />
              <div class="flex justify-end gap-1.5">
                <button
                  class="px-2 py-0.5 rounded border border-[var(--border)] text-[10.5px] hover:bg-[var(--bg-hover)]"
                  on:click={() => (isDownloadConfigOpen = false)}
                >
                  Stäng
                </button>
                <button
                  class="px-2 py-0.5 rounded bg-[var(--accent)] text-white text-[10.5px] font-semibold hover:bg-[var(--accent-hover)]"
                  on:click={() => {
                    setDownloadDirectory(customDownloadInput);
                    isDownloadConfigOpen = false;
                  }}
                >
                  Spara
                </button>
              </div>
            </div>
          {/if}
        </div>

        <!-- Lock Preview button -->
        <button
          class="p-1 rounded transition-colors {$isInspectorLocked ? 'bg-amber-500/20 text-amber-300 ring-1 ring-amber-500/50' : 'hover:bg-[var(--bg-hover)] text-[var(--text-secondary)]'}"
          on:click={toggleInspectorLock}
          title={$isInspectorLocked ? 'Inspektör låst på denna fil (Klicka eller tryck Cmd+L för att låsa upp)' : 'Lås inspektör på denna fil så den inte byts vid hovring (Cmd+L)'}
        >
          {#if $isInspectorLocked}
            <Lock size={12} class="text-amber-400" />
          {:else}
            <Unlock size={12} />
          {/if}
        </button>

        <!-- Cast to secondary window button -->
        <button
          class="p-1 rounded hover:bg-[var(--bg-hover)] text-amber-400 hover:text-amber-300 transition-transform {castedAnimation ? 'scale-125 text-emerald-400' : ''}"
          on:click={handleCastToLarge}
          title="Kasta uppåt till Stora Inspektörsfönstret (eller svep uppåt med 2 fingrar)"
        >
          <Rocket size={12} class={castedAnimation ? 'animate-bounce' : ''} />
        </button>

        <button
          class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)] hover:text-[var(--accent)]"
          on:click={handleDetach}
          title="Öppna/fokusera separat inspektörsfönster"
        >
          <SquareArrowOutUpRight size={12} />
        </button>

        <button
          class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)]"
          on:click={() => item && openInDefault(item.path)}
          title="Öppna i standardprogram"
        >
          <ExternalLink size={12} />
        </button>

        <button
          class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)]"
          on:click={() => item && revealInOs(item.path)}
          title="Visa i Finder"
        >
          <FolderOpen size={12} />
        </button>
      {/if}
    </div>
  </div>

  <!-- Inspector Body -->
  <div bind:this={inspectorBodyEl} class="flex-1 min-h-0 overflow-hidden flex flex-col bg-[var(--bg-base)]">
    {#if isMultiSelecting}
      <MultiItemInspector
        items={selectedItems}
        onDeselectItem={handleDeselectItem}
        onClearSelection={handleClearSelection}
      />
    {:else if currentTab === 'notes'}
      <NotesInspector dirPath={activeNotesDirectory} />
    {:else if currentTab === 'ai'}
      <AIInspector {item} {preview} />
    {:else if !item}
      <div class="flex-1 flex flex-col items-center justify-center p-6 text-center text-[var(--text-muted)]">
        <FileText size={32} class="opacity-20 mb-2" />
        <span>Välj en fil för att visa förhandsgranskning och metadata</span>
      </div>
    {:else if isBam}
      <BioInspector {item} />
    {:else if isArchive}
      <ArchiveInspector {item} />
    {:else if isLoading}
      <div class="flex-1 flex items-center justify-center text-[var(--text-muted)]">
        Läser in förhandsgranskning...
      </div>
    {:else if item.is_dir}
      <FolderInspector {item} />
    {:else if preview}
      <!-- 1. HTML REPORT PREVIEW (MultiQC, FastQC, Rmarkdown) -->
      {#if preview.kind === 'html' && preview.html_content}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-2.5 py-1 bg-[#161a24] border-b border-[#252d3d] text-[10.5px]">
            <div class="flex items-center gap-1">
              <button
                class="px-2 py-0.5 rounded font-medium transition-colors {htmlViewMode === 'rendered' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (htmlViewMode = 'rendered')}
              >
                🌐 Renderad
              </button>
              <button
                class="px-2 py-0.5 rounded font-medium transition-colors {htmlViewMode === 'source' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (htmlViewMode = 'source')}
              >
                📄 Källkod
              </button>
            </div>
            <button
              class="text-slate-400 hover:text-white flex items-center gap-1 text-[10px]"
              on:click={() => item && openInDefault(item.path)}
              title="Öppna i webbläsare"
            >
              <ExternalLink size={10} />
              <span>Webbläsare</span>
            </button>
          </div>

          {#if htmlViewMode === 'rendered'}
            <div class="flex-1 bg-white min-h-[300px]">
              <iframe
                bind:this={htmlIframeEl}
                srcdoc={preview.html_content}
                title={item.name}
                class="w-full h-full border-0 bg-white"
                sandbox="allow-scripts allow-same-origin allow-popups"
              ></iframe>
            </div>
          {:else}
            <div class="p-2 font-mono text-[11px] leading-relaxed text-[var(--text-primary)] overflow-x-auto select-text">
              <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
            </div>
          {/if}
        </div>

      <!-- 2. PDF DOCUMENT PREVIEW -->
      {:else if preview.kind === 'pdf' && preview.pdf_base64}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-2.5 py-1 bg-[#161a24] border-b border-[#252d3d] text-[10.5px]">
            <div class="flex items-center gap-1">
              <button
                class="px-2 py-0.5 rounded font-medium transition-colors {pdfViewMode === 'pdf' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (pdfViewMode = 'pdf')}
              >
                📄 PDF-visning
              </button>
              <button
                class="px-2 py-0.5 rounded font-medium transition-colors {pdfViewMode === 'hex' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (pdfViewMode = 'hex')}
              >
                🔢 Hex-dump
              </button>
            </div>
            <button
              class="text-slate-400 hover:text-white flex items-center gap-1 text-[10px]"
              on:click={() => item && openInDefault(item.path)}
              title="Öppna i Förhandsvisning"
            >
              <ExternalLink size={10} />
              <span>Preview.app</span>
            </button>
          </div>

          {#if pdfViewMode === 'pdf'}
            <div class="flex-1 bg-slate-900 min-h-[350px]">
              <iframe
                bind:this={pdfIframeEl}
                src="data:application/pdf;base64,{preview.pdf_base64}#toolbar=1"
                title={item.name}
                class="w-full h-full border-0 min-h-[350px]"
              ></iframe>
            </div>
          {:else if preview.hex_lines}
            <div class="p-2 font-mono text-[10px] text-purple-300 leading-tight select-text overflow-x-auto">
              {#each preview.hex_lines as line}
                <div>{line}</div>
              {/each}
            </div>
          {/if}
        </div>

      <!-- 3. MARKDOWN PREVIEW -->
      {:else if preview.kind === 'markdown' && preview.text_content}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-2.5 py-1 bg-[#161a24] border-b border-[#252d3d] text-[10.5px]">
            <div class="flex items-center gap-1">
              <button
                class="px-2 py-0.5 rounded font-medium transition-colors {mdViewMode === 'rendered' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (mdViewMode = 'rendered')}
              >
                📖 Formaterad
              </button>
              <button
                class="px-2 py-0.5 rounded font-medium transition-colors {mdViewMode === 'source' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (mdViewMode = 'source')}
              >
                📝 Råtext
              </button>
            </div>
          </div>

          {#if mdViewMode === 'rendered'}
            <div class="p-3 text-xs select-text overflow-auto space-y-2 leading-relaxed">
              {@html renderMarkdown(preview.text_content)}
            </div>
          {:else}
            <div class="p-2 font-mono text-[11px] leading-relaxed text-[var(--text-primary)] overflow-x-auto select-text">
              <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
            </div>
          {/if}
        </div>

      <!-- 4. VIDEO PREVIEW -->
      {:else if preview.kind === 'video' && preview.media_base64}
        <div class="p-3 flex flex-col items-center justify-center bg-black/50 min-h-[220px] gap-2">
          <video
            src="data:{preview.media_mime || 'video/mp4'};base64,{preview.media_base64}"
            controls
            class="max-h-72 w-full rounded shadow-md border border-[#252d3d]"
          >
            <track kind="captions" />
          </video>
          <span class="text-[10px] text-slate-400 font-mono">{item.name} ({preview.formatted_size})</span>
        </div>

      <!-- 5. AUDIO PREVIEW -->
      {:else if preview.kind === 'audio' && preview.media_base64}
        <div class="p-4 flex flex-col items-center justify-center bg-[#151922] min-h-[140px] gap-3 rounded-lg m-3 border border-[#252d3d]">
          <div class="w-10 h-10 rounded-full bg-emerald-500/20 text-emerald-400 flex items-center justify-center">
            <Volume2 size={20} />
          </div>
          <audio
            src="data:{preview.media_mime || 'audio/mpeg'};base64,{preview.media_base64}"
            controls
            class="w-full"
          ></audio>
          <span class="text-[10px] text-slate-400 font-mono">{item.name} ({preview.formatted_size})</span>
        </div>

      <!-- 6. SVG PREVIEW -->
      {:else if preview.kind === 'svg'}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-2.5 py-1 bg-[#161a24] border-b border-[#252d3d] text-[10.5px]">
            <div class="flex items-center gap-1">
              <button
                class="px-2 py-0.5 rounded font-medium transition-colors {svgViewMode === 'rendered' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (svgViewMode = 'rendered')}
              >
                🎨 Vektorbild
              </button>
              <button
                class="px-2 py-0.5 rounded font-medium transition-colors {svgViewMode === 'source' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (svgViewMode = 'source')}
              >
                📄 XML-kod
              </button>
            </div>
          </div>

          {#if svgViewMode === 'rendered' && preview.image_base64}
            <div class="p-4 flex items-center justify-center bg-black/40 min-h-[200px]">
              <img
                src="data:image/svg+xml;base64,{preview.image_base64}"
                alt={item.name}
                class="max-h-72 object-contain rounded shadow"
              />
            </div>
          {:else if preview.text_content}
            <div class="p-2 font-mono text-[11px] leading-relaxed text-[var(--text-primary)] overflow-x-auto select-text">
              <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
            </div>
          {/if}
        </div>

      <!-- 7. IMAGE PREVIEW -->
      {:else if preview.kind === 'image' && preview.image_base64}
        <div class="p-3 flex items-center justify-center bg-black/40 min-h-[180px]">
          <img
            src="data:{preview.image_mime || 'image/png'};base64,{preview.image_base64}"
            alt={item.name}
            class="max-h-64 object-contain rounded shadow"
          />
        </div>

      <!-- 8. JUPYTER NOTEBOOK PREVIEW -->
      {:else if preview.kind === 'notebook'}
        <NotebookViewer
          jsonContent={preview.text_content || ''}
          filename={item.name}
          formattedSize={preview.formatted_size}
        />

      <!-- 9. SPREADSHEET & DATA TABLE PREVIEW (Excel / CSV / TSV / ODS) -->
      {:else if preview.kind === 'table' && preview.table_headers && preview.table_rows}
        <SpreadsheetViewer
          headers={preview.table_headers}
          rows={preview.table_rows}
          sheetNames={preview.sheet_names || []}
          filename={item.name}
          formattedSize={preview.formatted_size}
        />

      <!-- 10. TEXT & CODE PREVIEW (with Syntax Highlighting) -->
      {:else if preview.kind === 'code' || preview.kind === 'text'}
        <CodeViewer
          bind:this={codeViewerRef}
          code={preview.text_content || ''}
          filename={item.name}
          language={preview.language || 'plaintext'}
          languageName={preview.language_name || 'Plain Text'}
          languageEmoji={preview.language_emoji || '📄'}
          formattedSize={preview.formatted_size}
        />

      <!-- 10. BINARY FILE PREVIEW -->
      {:else if preview.kind === 'binary' || preview.kind === 'hex'}
        <div class="flex-1 flex flex-col items-center justify-center p-6 text-center select-none overflow-y-auto">
          <div class="w-14 h-14 rounded-2xl bg-[#151922] border border-[#262f40] flex items-center justify-center mb-3 text-purple-400 shadow-sm">
            <Binary size={26} />
          </div>
          <div class="font-semibold text-sm text-slate-200">{item.name}</div>
          <div class="text-[11px] text-slate-400 font-mono mt-1">
            {preview.language_name || 'Binär fil'} • {item.formatted_size}
          </div>
          <p class="text-xs text-slate-400 max-w-xs mt-2 leading-relaxed">
            Binär fil som inte visas som rå text. Öppna filen i ett dedikerat program för att granska innehållet.
          </p>

          <div class="flex items-center gap-2 mt-4">
            <button
              class="px-3 py-1.5 rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-xs font-semibold flex items-center gap-1.5 shadow transition-colors"
              on:click={() => openInDefault(item.path)}
            >
              <ExternalLink size={13} />
              <span>Öppna i standardprogram</span>
            </button>
            <button
              class="px-3 py-1.5 rounded-lg bg-[#181d27] hover:bg-[#222836] text-slate-300 border border-[#262f40] text-xs font-medium flex items-center gap-1.5 transition-colors"
              on:click={() => revealInOs(item.path)}
            >
              <FolderOpen size={13} />
              <span>Visa i Finder</span>
            </button>
          </div>

          {#if preview.hex_lines && preview.hex_lines.length > 0}
            <div class="mt-6 w-full max-w-md">
              <button
                class="text-[10.5px] text-slate-500 hover:text-purple-300 underline font-mono transition-colors"
                on:click={() => (showHexDump = !showHexDump)}
              >
                {showHexDump ? '▲ Dölj Hex-dump' : '▼ Visa rå Hex-dump (första 512 bytes)'}
              </button>

              {#if showHexDump}
                <div class="mt-2 p-2 bg-[#0c0d10] border border-[#1e2330] rounded-lg font-mono text-[9.5px] text-purple-300 leading-tight select-text overflow-x-auto text-left max-h-48 overflow-y-auto">
                  {#each preview.hex_lines as line}
                    <div>{line}</div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        </div>

      <!-- ERROR / TOO LARGE -->
      {:else if preview.kind === 'error' || preview.kind === 'too_large'}
        <div class="p-4 text-amber-400 text-center">
          {preview.error_message || 'Kunde inte läsa förhandsgranskning'}
        </div>
      {/if}
    {/if}
  </div>

  <!-- Ultra-Compact Metadata Section (~95px) -->
  {#if item}
    <div class="p-2.5 border-t border-[var(--border)] bg-[var(--bg-surface)] text-[10px] space-y-1.5">
      <!-- Sökväg med truncate och Copy -->
      <div class="flex items-center justify-between gap-1 bg-[var(--bg-panel)] px-2 py-1 rounded border border-[var(--border)]">
        <span class="font-mono truncate text-[var(--text-secondary)]" title={item.path}>
          {truncateMiddle(item.path, 32)}
        </span>
        <button
          class="flex items-center gap-0.5 px-1.5 py-0.5 rounded bg-[var(--border)] hover:bg-[var(--accent)] hover:text-white text-[var(--text-primary)] transition-colors"
          on:click={copyPath}
          title="Kopiera fullständig sökväg"
        >
          {#if copied}
            <Check size={10} class="text-green-400" />
            <span class="text-[9px]">Kopierad</span>
          {:else}
            <Copy size={10} />
            <span class="text-[9px]">Kopiera</span>
          {/if}
        </button>
      </div>

      <!-- Kompakt filinfo -->
      <div class="grid grid-cols-2 gap-x-2 gap-y-0.5 text-[var(--text-muted)] font-mono">
        <div>Storlek: <span class="text-[var(--text-primary)] font-semibold">{item.formatted_size}</span></div>
        <div>Typ: <span class="text-[var(--text-primary)]">{item.extension.toUpperCase() || 'FIL'}</span></div>
        <div class="col-span-2 truncate">Ändrad: <span class="text-[var(--text-secondary)]">{item.formatted_modified}</span></div>
      </div>
    </div>
  {/if}
</div>
