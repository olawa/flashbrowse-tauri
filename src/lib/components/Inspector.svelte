<script lang="ts">
  import { onMount } from 'svelte';
  import { emit } from '@tauri-apps/api/event';
  import { getPreview, calculateDirSize, revealInOs, openInDefault, toggleDetachedInspector } from '../invoke';
  import { isInspectorDetached } from '../stores/navigation';
  import BioInspector from './BioInspector.svelte';
  import ArchiveInspector from './ArchiveInspector.svelte';
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
  } from 'lucide-svelte';

  export let item: FileItem | null = null;
  export let titlePrefix = 'Inspector';

  let preview: PreviewContent | null = null;
  let dirSummary: DirectorySummary | null = null;
  let isLoading = false;
  let isCalculatingDu = false;
  let copied = false;

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

  async function loadItemPreview(target: FileItem) {
    isLoading = true;
    dirSummary = null;
    try {
      if (target.is_dir) {
        preview = null;
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

  function truncateMiddle(str: string, maxLen = 35) {
    if (str.length <= maxLen) return str;
    const half = Math.floor((maxLen - 3) / 2);
    return str.substring(0, half) + '...' + str.substring(str.length - half);
  }
</script>

<div class="w-80 h-full flex flex-col border-l border-[var(--border)] bg-[var(--bg-surface)] text-xs select-none">
  <!-- Inspector Header -->
  <div class="flex items-center justify-between px-3 py-2 border-b border-[var(--border)] bg-[var(--bg-panel)]">
    <div class="flex items-center gap-1.5 truncate">
      <span class="text-[10px] font-bold tracking-wider text-[var(--accent)] uppercase">{titlePrefix}</span>
      <span class="text-[var(--text-muted)]">•</span>
      <span class="font-semibold text-[var(--text-primary)] truncate">{item ? item.name : 'No selection'}</span>
    </div>

    {#if item}
      <div class="flex items-center gap-1">
        <button
          class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)] hover:text-[var(--accent)]"
          on:click={handleDetach}
          title="Koppla loss inspektor till eget fönster (Detach Window)"
        >
          <SquareArrowOutUpRight size={12} />
        </button>
        <button
          class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)]"
          on:click={() => item && openInDefault(item.path)}
          title="Open in default app"
        >
          <ExternalLink size={12} />
        </button>
        <button
          class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)]"
          on:click={() => item && revealInOs(item.path)}
          title="Reveal in Finder / Explorer"
        >
          <FolderOpen size={12} />
        </button>
      </div>
    {/if}
  </div>

  <!-- Inspector Body -->
  <div class="flex-1 overflow-y-auto overflow-x-hidden flex flex-col bg-[var(--bg-base)]">
    {#if !item}
      <div class="flex-1 flex flex-col items-center justify-center p-6 text-center text-[var(--text-muted)]">
        <FileText size={32} class="opacity-20 mb-2" />
        <span>Select an item to view preview and metadata</span>
      </div>
    {:else if isBam}
      <BioInspector {item} />
    {:else if isArchive}
      <ArchiveInspector {item} />
    {:else if isLoading}
      <div class="flex-1 flex items-center justify-center text-[var(--text-muted)]">
        Loading preview...
      </div>
    {:else if item.is_dir}
      <!-- Directory View -->
      <div class="p-4 space-y-4">
        <div class="p-3 rounded-lg bg-[var(--bg-panel)] border border-[var(--border)] space-y-2">
          <div class="flex items-center justify-between text-[var(--text-secondary)]">
            <span>Folder:</span>
            <span class="font-semibold text-[var(--text-primary)]">{item.name}</span>
          </div>
          <div class="flex items-center justify-between text-[var(--text-secondary)]">
            <span>Modified:</span>
            <span>{item.formatted_modified}</span>
          </div>
          <div class="flex items-center justify-between text-[var(--text-secondary)]">
            <span>Permissions:</span>
            <span class="font-mono text-[11px]">{item.permissions}</span>
          </div>
        </div>

        {#if dirSummary}
          <div class="p-3 rounded-lg bg-emerald-950/30 border border-emerald-800 text-emerald-300 space-y-1">
            <div class="font-bold flex items-center justify-between">
              <span>Recursive Size:</span>
              <span>{dirSummary.formatted_total_size}</span>
            </div>
            <div class="text-[11px] text-emerald-400/80 flex justify-between">
              <span>Files: {dirSummary.total_files}</span>
              <span>Subfolders: {dirSummary.total_dirs}</span>
            </div>
          </div>
        {:else}
          <button
            class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white font-medium shadow-sm disabled:opacity-50"
            disabled={isCalculatingDu}
            on:click={calculateFolderDu}
          >
            <PieChart size={14} />
            <span>{isCalculatingDu ? 'Calculating Size...' : 'Calculate Folder Size (du -h)'}</span>
          </button>
        {/if}
      </div>
    {:else if preview}
      <!-- File Preview Types -->
      {#if preview.kind === 'image' && preview.image_base64}
        <div class="p-3 flex items-center justify-center bg-black/40 min-h-[180px]">
          <img
            src="data:{preview.image_mime || 'image/png'};base64,{preview.image_base64}"
            alt={item.name}
            class="max-h-64 object-contain rounded shadow"
          />
        </div>
      {:else if preview.kind === 'table' && preview.table_headers && preview.table_rows}
        <div class="overflow-x-auto p-2">
          <table class="w-full text-left border-collapse text-[10px] font-mono">
            <thead>
              <tr class="border-b border-[var(--border)] bg-[var(--bg-panel)]">
                {#each preview.table_headers as header}
                  <th class="p-1 font-semibold text-[var(--accent)] truncate max-w-[100px]">{header}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each preview.table_rows as row}
                <tr class="border-b border-[var(--border)]/40 hover:bg-[var(--bg-hover)]">
                  {#each row as cell}
                    <td class="p-1 truncate max-w-[120px]">{cell}</td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else if preview.kind === 'code' || preview.kind === 'text'}
        <div class="p-2 font-mono text-[11px] leading-relaxed text-[var(--text-primary)] overflow-x-auto select-text">
          <pre class="m-0 whitespace-pre-wrap break-words">{preview.text_content}</pre>
        </div>
      {:else if preview.kind === 'hex' && preview.hex_lines}
        <div class="p-2 font-mono text-[10px] text-purple-300 leading-tight select-text overflow-x-auto">
          {#each preview.hex_lines as line}
            <div>{line}</div>
          {/each}
        </div>
      {:else if preview.kind === 'error'}
        <div class="p-4 text-red-400 text-center">
          {preview.error_message || 'Could not load preview'}
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
          title="Copy full path"
        >
          {#if copied}
            <Check size={10} class="text-green-400" />
            <span class="text-[9px]">Copied</span>
          {:else}
            <Copy size={10} />
            <span class="text-[9px]">Copy</span>
          {/if}
        </button>
      </div>

      <!-- Kompakt filinfo -->
      <div class="grid grid-cols-2 gap-x-2 gap-y-0.5 text-[var(--text-muted)] font-mono">
        <div>Size: <span class="text-[var(--text-primary)] font-semibold">{item.formatted_size}</span></div>
        <div>Type: <span class="text-[var(--text-primary)]">{item.extension.toUpperCase() || 'FILE'}</span></div>
        <div class="col-span-2 truncate">Date: <span class="text-[var(--text-secondary)]">{item.formatted_modified}</span></div>
      </div>
    </div>
  {/if}
</div>
