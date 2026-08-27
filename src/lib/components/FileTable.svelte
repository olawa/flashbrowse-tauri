<script lang="ts">
  import { leftPane, rightPane, activePaneId, navigatePane, sortPaneItems, goUp } from '../stores/navigation';
  import { isKidsMode } from '../stores/theme';
  import { openInDefault } from '../invoke';
  import ContextMenu from './ContextMenu.svelte';
  import type { FileItem } from '../types';
  import {
    Folder,
    FileText,
    FileCode,
    FileSpreadsheet,
    FileImage,
    FileArchive,
    File,
    ArrowUpDown,
    ArrowUp,
    ArrowDown,
    Search,
  } from 'lucide-svelte';

  export let paneId: 'left' | 'right' = 'left';
  export let onSelectPreview: (item: FileItem | null) => void;

  $: pane = paneId === 'left' ? $leftPane : $rightPane;
  $: isActive = $activePaneId === paneId;

  let filterText = '';
  let contextMenuItem: FileItem | null = null;
  let contextMenuPos = { x: 0, y: 0 };

  $: filteredItems = pane.items.filter((item) => {
    if (!filterText) return true;
    return item.name.toLowerCase().includes(filterText.toLowerCase());
  });

  function getFileIcon(item: FileItem) {
    if (item.is_dir) return Folder;
    const ext = item.extension.toLowerCase();
    if (['rs', 'py', 'js', 'ts', 'c', 'cpp', 'swift', 'sh', 'html', 'css', 'json'].includes(ext)) return FileCode;
    if (['csv', 'tsv', 'xlsx', 'tab'].includes(ext)) return FileSpreadsheet;
    if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext)) return FileImage;
    if (['zip', 'tar', 'gz', 'bz2', '7z'].includes(ext)) return FileArchive;
    if (['txt', 'md', 'doc', 'pdf'].includes(ext)) return FileText;
    return File;
  }

  function getIconColor(item: FileItem) {
    if (item.is_dir) return 'text-amber-400';
    const ext = item.extension.toLowerCase();
    if (['rs', 'py', 'js', 'ts', 'swift'].includes(ext)) return 'text-cyan-400';
    if (['csv', 'tsv'].includes(ext)) return 'text-emerald-400';
    if (['png', 'jpg', 'jpeg', 'svg'].includes(ext)) return 'text-pink-400';
    if (['zip', 'tar', 'gz'].includes(ext)) return 'text-red-400';
    return 'text-slate-400';
  }

  function handleRowClick(item: FileItem, event: MouseEvent) {
    activePaneId.set(paneId);
    const store = paneId === 'left' ? leftPane : rightPane;

    if (event.metaKey || event.ctrlKey) {
      store.update((s) => {
        const next = new Set(s.selectedPaths);
        if (next.has(item.path)) next.delete(item.path);
        else next.add(item.path);
        return { ...s, selectedPaths: next };
      });
    } else {
      store.update((s) => ({
        ...s,
        selectedPaths: new Set([item.path]),
      }));
    }

    onSelectPreview(item);
  }

  function handleDoubleClick(item: FileItem) {
    if (item.is_dir) {
      navigatePane(paneId, item.path);
    } else {
      openInDefault(item.path);
    }
  }

  function handleContextMenu(item: FileItem, event: MouseEvent) {
    event.preventDefault();
    contextMenuItem = item;
    contextMenuPos = { x: event.clientX, y: event.clientY };
  }

  function closeContextMenu() {
    contextMenuItem = null;
  }
</script>

<svelte:window on:click={closeContextMenu} />

<div
  class="flex-1 flex flex-col h-full bg-[var(--bg-base)] overflow-hidden {isActive ? 'ring-1 ring-[var(--accent)]' : ''}"
  on:mousedown={() => activePaneId.set(paneId)}
>
  <!-- Top Filter bar inside panel -->
  <div class="flex items-center gap-2 px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)]">
    <div class="relative flex-1">
      <Search size={12} class="absolute left-2 top-2 text-[var(--text-muted)]" />
      <input
        type="text"
        bind:value={filterText}
        placeholder="Filter {pane.items.length} items..."
        class="w-full bg-[var(--bg-panel)] text-xs text-[var(--text-primary)] rounded pl-6 pr-2 py-1 border border-[var(--border)] focus:outline-none focus:border-[var(--accent)]"
      />
    </div>
    <span class="text-[11px] text-[var(--text-muted)] font-mono">
      {filteredItems.length} / {pane.items.length}
    </span>
  </div>

  {#if pane.errorMessage}
    <div class="p-4 m-3 rounded bg-red-900/20 border border-red-800 text-red-400 text-xs">
      {pane.errorMessage}
    </div>
  {:else if pane.isLoading}
    <div class="flex-1 flex items-center justify-center text-xs text-[var(--text-muted)]">
      Loading directory...
    </div>
  {:else if $isKidsMode}
    <!-- Kids Mode Card Grid -->
    <div class="flex-1 overflow-y-auto p-4 grid grid-cols-3 sm:grid-cols-4 gap-4">
      {#each filteredItems as item}
        <button
          class="flex flex-col items-center justify-center p-4 rounded-2xl bg-white border-2 border-[var(--border)] hover:scale-105 hover:shadow-lg transition-all text-center {pane.selectedPaths.has(item.path) ? 'ring-4 ring-[var(--accent)] bg-[var(--accent-subtle)]' : ''}"
          on:click={(e) => handleRowClick(item, e)}
          on:dblclick={() => handleDoubleClick(item)}
        >
          <div class="w-16 h-16 rounded-xl flex items-center justify-center {item.is_dir ? 'bg-amber-100' : 'bg-blue-100'} mb-2">
            <svelte:component this={getFileIcon(item)} size={32} class={getIconColor(item)} />
          </div>
          <span class="font-bold text-sm text-[var(--text-primary)] truncate max-w-[120px]">{item.name}</span>
          <span class="text-[10px] text-[var(--text-muted)]">{item.formatted_size}</span>
        </button>
      {/each}
    </div>
  {:else}
    <!-- Pro Table View -->
    <div class="flex-1 overflow-y-auto flex flex-col text-xs font-mono select-none">
      <!-- Table Header -->
      <div class="grid grid-cols-12 gap-2 px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] text-[var(--text-muted)] font-sans font-semibold text-[11px] sticky top-0 z-10">
        <button
          class="col-span-6 flex items-center gap-1 text-left hover:text-[var(--text-primary)]"
          on:click={() => sortPaneItems(paneId, 'name')}
        >
          <span>Name</span>
          {#if pane.sortBy === 'name'}
            {#if pane.sortAsc}<ArrowUp size={11} />{:else}<ArrowDown size={11} />{/if}
          {/if}
        </button>

        <button
          class="col-span-2 flex items-center gap-1 justify-end hover:text-[var(--text-primary)]"
          on:click={() => sortPaneItems(paneId, 'size')}
        >
          <span>Size</span>
          {#if pane.sortBy === 'size'}
            {#if pane.sortAsc}<ArrowUp size={11} />{:else}<ArrowDown size={11} />{/if}
          {/if}
        </button>

        <button
          class="col-span-2 flex items-center gap-1 hover:text-[var(--text-primary)] pl-2"
          on:click={() => sortPaneItems(paneId, 'modified')}
        >
          <span>Modified</span>
          {#if pane.sortBy === 'modified'}
            {#if pane.sortAsc}<ArrowUp size={11} />{:else}<ArrowDown size={11} />{/if}
          {/if}
        </button>

        <div class="col-span-2 text-right">
          <span>Mode</span>
        </div>
      </div>

      <!-- Table Rows -->
      <div class="divide-y divide-[var(--border)]/40">
        {#each filteredItems as item}
          {@const isSelected = pane.selectedPaths.has(item.path)}
          <div
            class="grid grid-cols-12 gap-2 px-3 py-1 items-center cursor-pointer transition-colors {isSelected ? 'bg-[var(--accent-subtle)] text-[var(--accent)] font-medium' : 'hover:bg-[var(--bg-hover)] text-[var(--text-primary)]'}"
            on:click={(e) => handleRowClick(item, e)}
            on:dblclick={() => handleDoubleClick(item)}
            on:contextmenu={(e) => handleContextMenu(item, e)}
          >
            <div class="col-span-6 flex items-center gap-2 min-w-0">
              <svelte:component this={getFileIcon(item)} size={14} class="{getIconColor(item)} flex-shrink-0" />
              <span class="truncate font-sans {item.is_dir ? 'font-semibold' : ''}">{item.name}</span>
            </div>

            <div class="col-span-2 text-right text-[var(--text-secondary)] font-mono text-[11px]">
              {item.formatted_size}
            </div>

            <div class="col-span-2 text-[var(--text-muted)] text-[11px] truncate pl-2 font-mono">
              {item.formatted_modified}
            </div>

            <div class="col-span-2 text-right text-[var(--text-muted)] text-[10px] font-mono">
              {item.permissions}
            </div>
          </div>
        {/each}

        {#if filteredItems.length === 0}
          <div class="p-8 text-center text-[var(--text-muted)]">
            Empty directory
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

{#if contextMenuItem}
  <ContextMenu
    item={contextMenuItem}
    {paneId}
    x={contextMenuPos.x}
    y={contextMenuPos.y}
    onClose={closeContextMenu}
  />
{/if}
