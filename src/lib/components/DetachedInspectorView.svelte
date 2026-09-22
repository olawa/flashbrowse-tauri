<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    getPreview,
    sshGetPreview,
    calculateDirSize,
    revealInOs,
    openInDefault,
    toggleDetachedInspector,
    getInspectorInitialPath,
  } from '../invoke';
  import { renderMarkdown } from '../markdown';
  import ImagePreview from './ImagePreview.svelte';
  import BioInspector from './BioInspector.svelte';
  import ArchiveInspector from './ArchiveInspector.svelte';
  import FolderInspector from './FolderInspector.svelte';
  import CodeViewer from './CodeViewer.svelte';
  import SpreadsheetViewer from './SpreadsheetViewer.svelte';
  import NotebookViewer from './NotebookViewer.svelte';
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
  let unlistenCastItem: (() => void) | null = null;
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
    // 1. Check if an initial path was provided in URL query or from backend
    const urlParams = new URLSearchParams(window.location.search);
    const p = urlParams.get('path');
    if (p) {
      setItemFromPath(decodeURIComponent(p));
    } else {
      try {
        const initPath = await getInspectorInitialPath();
        if (initPath) {
          setItemFromPath(initPath);
        }
      } catch (e) {
        console.warn('Could not get initial path:', e);
      }
    }

    // 2. Listen ONLY to active cast events from the main window (never update on incidental hover!)
    try {
      unlistenCastSync = await listen<string>('inspector-cast-path', async (event) => {
        if (event.payload) {
          setItemFromPath(event.payload, true);
        }
      });

      unlistenCastItem = await listen<FileItem>('inspector-cast-item', async (event) => {
        if (event.payload) {
          currentItem = event.payload;
          castAlert = true;
          setTimeout(() => (castAlert = false), 2500);
          await loadPreview(currentItem.path);
        }
      });
    } catch (e) {
      console.error('Failed to listen to inspector cast events:', e);
    }
  });

  onDestroy(() => {
    if (unlistenCastSync) unlistenCastSync();
    if (unlistenCastItem) unlistenCastItem();
  });

  async function loadPreview(path: string) {
    if (!path) return;
    isLoading = true;
    dirSummary = null;
    try {
      if (path.startsWith('ssh://')) {
        const clean = path.replace(/^ssh:\/\//, '');
        const slashIdx = clean.indexOf('/');
        const host = slashIdx > 0 ? clean.slice(0, slashIdx) : clean;
        const remotePath = slashIdx > 0 ? clean.slice(slashIdx) : '/';
        preview = await sshGetPreview(host, remotePath);
      } else {
        preview = await getPreview(path);
      }
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

<div class="flex flex-col h-screen w-screen bg-[var(--bg-base)] text-[#f1f5f9] font-sans select-none overflow-hidden relative">
  <!-- Cast Toast Banner -->
  {#if castAlert}
    <div class="absolute top-12 left-1/2 -translate-x-1/2 z-50 px-4 py-1.5 rounded-full bg-emerald-600 text-[var(--text-primary)] text-xs font-bold shadow-2xl flex items-center gap-2 animate-bounce border border-emerald-400">
      <Rocket size={14} />
      <span>Fil kastad hit från filtabellen!</span>
    </div>
  {/if}

  <!-- Top Bar -->
  <div class="flex items-center justify-between px-4 py-2.5 bg-[var(--bg-surface)] border-b border-[var(--border)] shrink-0">
    <div class="flex items-center gap-2.5 min-w-0">
      <div class="flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-[var(--accent)]/20 text-[var(--accent)] border border-[var(--accent)]/40 text-[11px] font-bold tracking-wider uppercase">
        <Sparkles size={11} />
        <span>Stora Inspektorn</span>
      </div>
      <span class="font-bold text-sm text-[var(--text-primary)] truncate max-w-lg select-text" title={currentItem ? currentItem.path : ''}>
        {currentItem ? currentItem.name : 'Väntar på fil...'}
      </span>
      {#if currentItem?.extension}
        <span class="px-1.5 py-0.2 rounded bg-slate-800 text-[var(--text-primary)] text-[10px] font-mono border border-slate-700">
          {currentItem.extension.toUpperCase()}
        </span>
      {/if}
      <span class="px-2 py-0.5 rounded-full bg-emerald-950/60 border border-emerald-700/50 text-emerald-300 text-[10px] font-medium flex items-center gap-1" title="Detta fönster är statiskt och ändras inte vid hovring i fillistan - bara när du aktivt kastar en ny fil hit">
        <span>📌 Låst till kastad fil</span>
      </span>
    </div>

    {#if currentItem}
      <div class="flex items-center gap-2">
        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[var(--bg-panel)] hover:bg-[var(--bg-active)] border border-[var(--border)] text-xs text-[var(--text-primary)] hover:text-[var(--text-primary)] transition-colors"
          on:click={() => loadPreview(currentItem ? currentItem.path : '')}
          title="Ladda om filinnehåll"
        >
          <RefreshCw size={12} class={isLoading ? 'animate-spin' : ''} />
          <span>Ladda om</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[var(--bg-panel)] hover:bg-[var(--bg-active)] border border-[var(--border)] text-xs text-[var(--text-primary)] hover:text-[var(--text-primary)] transition-colors"
          on:click={() => openInDefault(currentItem ? currentItem.path : '')}
          title="Öppna i standardprogram"
        >
          <ExternalLink size={12} />
          <span>Öppna</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[var(--bg-panel)] hover:bg-[var(--bg-active)] border border-[var(--border)] text-xs text-[var(--text-primary)] hover:text-[var(--text-primary)] transition-colors"
          on:click={() => revealInOs(currentItem ? currentItem.path : '')}
          title="Visa i Finder"
        >
          <FolderOpen size={12} />
          <span>Finder</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[var(--accent)]/20 hover:bg-[var(--accent)] text-[var(--accent)] hover:text-white border border-[var(--accent)]/40 text-xs font-medium ml-1 transition-colors"
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
  <div class="flex-1 overflow-auto bg-[var(--bg-base)] flex flex-col font-mono text-xs select-text">
    {#if !currentItem}
      <div class="h-full flex flex-col items-center justify-center text-[var(--text-muted)]">
        <FileText size={40} class="opacity-20 mb-3" />
        <span>Väntar på att filer markeras eller sveps uppåt i Flashbrowse...</span>
      </div>
    {:else if isBam}
      <BioInspector item={currentItem} />
    {:else if isArchive}
      <ArchiveInspector item={currentItem} />
    {:else if isLoading}
      <div class="h-full flex items-center justify-center text-[var(--text-muted)]">
        Läser in filinnehåll...
      </div>
    {:else if currentItem.is_dir || (preview && preview.kind === 'directory')}
      <FolderInspector item={currentItem} />
    {:else if preview}
      <!-- 1. HTML REPORT PREVIEW (MultiQC / FastQC) -->
      {#if preview.kind === 'html' && preview.html_content}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-4 py-1.5 bg-[var(--bg-panel)] border-b border-[var(--border)] text-xs">
            <div class="flex items-center gap-1.5">
              <button
                class="px-3 py-1 rounded font-medium transition-colors {htmlViewMode === 'rendered' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-white'}"
                on:click={() => (htmlViewMode = 'rendered')}
              >
                🌐 Renderad rapport
              </button>
              <button
                class="px-3 py-1 rounded font-medium transition-colors {htmlViewMode === 'source' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-white'}"
                on:click={() => (htmlViewMode = 'source')}
              >
                📄 Källkod
              </button>
            </div>
            <button
              class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center gap-1 text-xs"
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
            <div class="p-4 font-mono text-xs leading-relaxed text-[var(--text-primary)] overflow-auto select-text">
              <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
            </div>
          {/if}
        </div>

      <!-- 2. PDF DOCUMENT PREVIEW -->
      {:else if preview.kind === 'pdf' && preview.pdf_base64}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-4 py-1.5 bg-[var(--bg-panel)] border-b border-[var(--border)] text-xs">
            <div class="flex items-center gap-1.5">
              <button
                class="px-3 py-1 rounded font-medium transition-colors {pdfViewMode === 'pdf' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-white'}"
                on:click={() => (pdfViewMode = 'pdf')}
              >
                📄 PDF-visning
              </button>
              <button
                class="px-3 py-1 rounded font-medium transition-colors {pdfViewMode === 'hex' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-white'}"
                on:click={() => (pdfViewMode = 'hex')}
              >
                🔢 Hex-dump
              </button>
            </div>
            <button
              class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] flex items-center gap-1 text-xs"
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
          <div class="flex items-center justify-between px-4 py-1.5 bg-[var(--bg-panel)] border-b border-[var(--border)] text-xs">
            <div class="flex items-center gap-1.5">
              <button
                class="px-3 py-1 rounded font-medium transition-colors {mdViewMode === 'rendered' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-white'}"
                on:click={() => (mdViewMode = 'rendered')}
              >
                📖 Formaterad
              </button>
              <button
                class="px-3 py-1 rounded font-medium transition-colors {mdViewMode === 'source' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-white'}"
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
            <div class="p-4 font-mono text-xs leading-relaxed text-[var(--text-primary)] overflow-auto select-text">
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
            class="max-h-[75vh] max-w-4xl rounded-xl shadow-2xl border border-[var(--border)]"
          >
            <track kind="captions" />
          </video>
        </div>

      <!-- 5. AUDIO PREVIEW -->
      {:else if preview.kind === 'audio' && preview.media_base64}
        <div class="flex-1 flex flex-col items-center justify-center p-12 bg-[var(--bg-surface)]">
          <div class="w-16 h-16 rounded-full bg-emerald-500/20 text-emerald-400 flex items-center justify-center mb-4">
            <Volume2 size={32} />
          </div>
          <h3 class="font-bold text-lg text-[var(--text-primary)] mb-2">{currentItem.name}</h3>
          <audio
            src="data:{preview.media_mime || 'audio/mpeg'};base64,{preview.media_base64}"
            controls
            class="w-96 mb-2"
          ></audio>
          <span class="text-xs text-[var(--text-secondary)]">{preview.formatted_size}</span>
        </div>

      <!-- 6. SVG PREVIEW -->
      {:else if preview.kind === 'svg'}
        <div class="flex-1 flex flex-col h-full overflow-hidden">
          <div class="flex items-center justify-between px-4 py-1.5 bg-[var(--bg-panel)] border-b border-[var(--border)] text-xs">
            <div class="flex items-center gap-1.5">
              <button
                class="px-3 py-1 rounded font-medium transition-colors {svgViewMode === 'rendered' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-white'}"
                on:click={() => (svgViewMode = 'rendered')}
              >
                🎨 Vektorbild
              </button>
              <button
                class="px-3 py-1 rounded font-medium transition-colors {svgViewMode === 'source' ? 'bg-[var(--accent)] text-white font-bold' : 'text-[var(--text-secondary)] hover:text-white'}"
                on:click={() => (svgViewMode = 'source')}
              >
                📄 XML-kod
              </button>
            </div>
          </div>

          {#if svgViewMode === 'rendered' && preview.image_base64}
            <div class="flex-1 flex items-center justify-center p-8 bg-[var(--bg-active)]">
              <img
                src="data:image/svg+xml;base64,{preview.image_base64}"
                alt={currentItem.name}
                class="max-h-[80vh] max-w-full object-contain rounded shadow-2xl"
              />
            </div>
          {:else if preview.text_content}
            <div class="p-4 font-mono text-xs leading-relaxed text-[var(--text-primary)] overflow-auto select-text">
              <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
            </div>
          {/if}
        </div>

      <!-- 7. IMAGE PREVIEW -->
      {:else if preview.kind === 'image' && preview.image_base64}
        <div class="flex-1 flex flex-col min-h-0">
          <ImagePreview
            src="data:{preview.image_mime || 'image/png'};base64,{preview.image_base64}"
            alt={currentItem.name}
          />
        </div>

      <!-- 8. JUPYTER NOTEBOOK PREVIEW -->
      {:else if preview.kind === 'notebook'}
        <NotebookViewer
          jsonContent={preview.text_content || ''}
          filename={currentItem.name}
          formattedSize={preview.formatted_size}
        />

      <!-- 9. SPREADSHEET & DATA TABLE PREVIEW (Excel / CSV / TSV / ODS) -->
      {:else if preview.kind === 'table' && preview.table_headers && preview.table_rows}
        <SpreadsheetViewer
          headers={preview.table_headers}
          rows={preview.table_rows}
          sheetNames={preview.sheet_names || []}
          filename={currentItem.name}
          formattedSize={preview.formatted_size}
        />

      <!-- 10. TEXT & CODE PREVIEW (with Syntax Highlighting) -->
      {:else if preview.kind === 'code' || preview.kind === 'text'}
        <CodeViewer
          code={preview.text_content || ''}
          filename={currentItem.name}
          language={preview.language || 'plaintext'}
          languageName={preview.language_name || 'Plain Text'}
          languageEmoji={preview.language_emoji || '📄'}
          formattedSize={preview.formatted_size}
        />

      <!-- 10. BINARY HEX PREVIEW -->
      {:else if preview.kind === 'hex' && preview.hex_lines}
        <div class="p-4 rounded-xl bg-[var(--bg-surface)] border border-[var(--border)] m-4 text-purple-300 leading-tight">
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
    <div class="px-4 py-2 bg-[var(--bg-surface)] border-t border-[var(--border)] flex items-center justify-between text-xs font-mono text-[var(--text-secondary)] shrink-0">
      <div class="flex items-center gap-3 truncate">
        <span class="truncate max-w-[500px]" title={currentItem.path}>{currentItem.path}</span>
        <button
          class="flex items-center gap-1 px-2 py-0.5 rounded bg-[var(--bg-active)] hover:bg-[var(--accent)] hover:text-white text-white transition-colors"
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
