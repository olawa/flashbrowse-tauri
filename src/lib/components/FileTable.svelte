<script lang="ts">
  import { onMount } from 'svelte';
  import {
    leftPane,
    rightPane,
    activePaneId,
    navigatePane,
    sortPaneItems,
    goUp,
    clickMode,
    smartHoverPreview,
    refreshPane,
    activeHoveredItem,
    castToSecondaryInspector,
  } from '../stores/navigation';
  import { isKidsMode } from '../stores/theme';
  import { openInDefault, quickLook, renameItem } from '../invoke';
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
    ArrowUp,
    ArrowDown,
    Search,
    Rocket,
    X,
  } from 'lucide-svelte';

  export let paneId: 'left' | 'right' = 'left';
  export let onSelectPreview: (item: FileItem | null) => void;

  $: pane = paneId === 'left' ? $leftPane : $rightPane;
  $: isActive = $activePaneId === paneId;

  let filterText = '';
  $: if (pane.filterQuery !== undefined && filterText !== pane.filterQuery) {
    filterText = pane.filterQuery;
  }
  let contextMenuItem: FileItem | null = null;
  let contextMenuPos = { x: 0, y: 0 };
  let tableContainerEl: HTMLDivElement;

  // Hover preview state
  let hoverTimer: any = null;
  let hoveredPath: string | null = null;

  // Inline rename state (Finder style delayed click)
  let lastClickedPath: string | null = null;
  let lastClickTimestamp = 0;
  let renamingPath: string | null = null;
  let renameInputText = '';
  let renameInputEl: HTMLInputElement;

  // Trackpad pinch gesture accumulator
  let pinchDeltaAccumulator = 0;
  let lastPinchTriggerTime = 0;

  function globToRegex(glob: string): RegExp {
    const escaped = glob.replace(/[.+^${}()|[\]\\]/g, '\\$&');
    const regexStr = '^' + escaped.replace(/\*/g, '.*').replace(/\?/g, '.') + '$';
    return new RegExp(regexStr, 'i');
  }

  function matchFilter(name: string, query: string): boolean {
    const q = query.trim();
    if (!q) return true;

    // Support multiple wildcard / search patterns separated by space or comma: e.g. "*.png, *.jpg" or "*.rs *.toml"
    const tokens = q.split(/[\s,]+/).filter(Boolean);
    if (tokens.length > 1) {
      return tokens.some((token) => matchSinglePattern(name, token));
    }
    return matchSinglePattern(name, q);
  }

  function matchSinglePattern(name: string, pattern: string): boolean {
    if (pattern.includes('*') || pattern.includes('?')) {
      try {
        const rx = globToRegex(pattern);
        return rx.test(name);
      } catch {
        return name.toLowerCase().includes(pattern.toLowerCase());
      }
    }
    return name.toLowerCase().includes(pattern.toLowerCase());
  }

  $: filteredItems = pane.items.filter((item) => matchFilter(item.name, filterText));

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

  // MARK: - Single / Double Click & Finder Rename Handling
  function handleRowClick(item: FileItem, event: MouseEvent) {
    activePaneId.set(paneId);
    const store = paneId === 'left' ? leftPane : rightPane;
    const now = Date.now();
    const timeSinceLastClick = now - lastClickTimestamp;

    // Multi-select with Shift or Cmd/Ctrl
    if (event.metaKey || event.ctrlKey) {
      store.update((s) => {
        const next = new Set(s.selectedPaths);
        if (next.has(item.path)) next.delete(item.path);
        else next.add(item.path);
        return { ...s, selectedPaths: next };
      });
      lastClickedPath = item.path;
      lastClickTimestamp = now;
      onSelectPreview(item);
      return;
    }

    // Finder-style delayed click on already single-selected item to start rename!
    const isAlreadySingleSelected = pane.selectedPaths.has(item.path) && pane.selectedPaths.size === 1;
    if (isAlreadySingleSelected && lastClickedPath === item.path && timeSinceLastClick > 450 && timeSinceLastClick < 2500) {
      startInlineRename(item);
      lastClickedPath = null;
      return;
    }

    lastClickedPath = item.path;
    lastClickTimestamp = now;

    // Click Mode Handling
    if ($clickMode === 'folders-only') {
      if (item.is_dir) {
        navigatePane(paneId, item.path);
        return;
      } else {
        store.update((s) => ({ ...s, selectedPaths: new Set([item.path]) }));
        onSelectPreview(item);
      }
    } else if ($clickMode === 'always') {
      if (item.is_dir) {
        navigatePane(paneId, item.path);
      } else {
        openInDefault(item.path);
      }
    } else {
      // double-click mode
      store.update((s) => ({ ...s, selectedPaths: new Set([item.path]) }));
      onSelectPreview(item);
    }
  }

  function handleDoubleClick(item: FileItem) {
    if (item.is_dir) {
      navigatePane(paneId, item.path);
    } else {
      openInDefault(item.path);
    }
  }

  // MARK: - Smart Hover Live Preview
  function handleRowMouseEnter(item: FileItem) {
    hoveredPath = item.path;
    activeHoveredItem.set(item);
    if ($smartHoverPreview) {
      clearTimeout(hoverTimer);
      hoverTimer = setTimeout(() => {
        if (hoveredPath === item.path) {
          onSelectPreview(item);
        }
      }, 70);
    }
  }

  function handleRowMouseLeave() {
    hoveredPath = null;
    clearTimeout(hoverTimer);
  }

  // MARK: - Inline Rename
  function startInlineRename(item: FileItem) {
    renamingPath = item.path;
    renameInputText = item.name;
    setTimeout(() => {
      renameInputEl?.focus();
      renameInputEl?.select();
    }, 50);
  }

  async function commitInlineRename() {
    if (!renamingPath || !renameInputText.trim()) {
      renamingPath = null;
      return;
    }
    try {
      await renameItem(renamingPath, renameInputText.trim());
      await refreshPane(paneId);
    } catch (e: any) {
      alert(`Failed to rename: ${e}`);
    } finally {
      renamingPath = null;
    }
  }

  function cancelInlineRename() {
    renamingPath = null;
  }

  // Cast fling gesture state (Two-finger swipe UP)
  let castingRowPath: string | null = null;
  let lastCastTriggerTime = 0;

  async function handleCastItem(item: FileItem) {
    const now = Date.now();
    if (now - lastCastTriggerTime < 400) return;
    lastCastTriggerTime = now;
    castingRowPath = item.path;
    setTimeout(() => {
      if (castingRowPath === item.path) castingRowPath = null;
    }, 1200);
    await castToSecondaryInspector(item);
  }

  function handleRowWheel(item: FileItem, e: WheelEvent) {
    if (e.ctrlKey) return; // Keep ctrl+wheel for pinch in/out

    // Two-finger swipe UP detection: brisk negative deltaY
    if (e.deltaY < -35 && Math.abs(e.deltaY) > Math.abs(e.deltaX) * 2.0) {
      const now = Date.now();
      if (now - lastCastTriggerTime > 600) {
        e.preventDefault();
        handleCastItem(item);
      }
    }
  }

  // MARK: - Trackpad Pinch to Open / Up
  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey) {
      e.preventDefault();
      pinchDeltaAccumulator += e.deltaY;
      const now = Date.now();

      if (now - lastPinchTriggerTime > 400) {
        if (pinchDeltaAccumulator > 30) {
          // Pinch Out -> Open first selected folder
          lastPinchTriggerTime = now;
          pinchDeltaAccumulator = 0;
          const firstSelected = Array.from(pane.selectedPaths)[0];
          const item = pane.items.find((i) => i.path === firstSelected);
          if (item && item.is_dir) {
            navigatePane(paneId, item.path);
          }
        } else if (pinchDeltaAccumulator < -30) {
          // Pinch In -> Go up to parent folder
          lastPinchTriggerTime = now;
          pinchDeltaAccumulator = 0;
          goUp(paneId);
        }
      }
    }
  }

  // MARK: - Keyboard Handling (Space for QuickLook, Arrows, Enter, Cmd+Shift+Up for Cast)
  function handleKeyDown(e: KeyboardEvent) {
    if (renamingPath) return;

    // Shortcut for Cast: Cmd+Shift+Up or Cmd+Alt+Up
    if ((e.metaKey || e.ctrlKey) && (e.shiftKey || e.altKey) && e.key === 'ArrowUp') {
      e.preventDefault();
      const firstSelected = Array.from(pane.selectedPaths)[0];
      const item = pane.items.find((i) => i.path === firstSelected);
      if (item) {
        handleCastItem(item);
        return;
      }
    }

    if (e.key === ' ' && pane.selectedPaths.size > 0) {
      e.preventDefault();
      const firstSelected = Array.from(pane.selectedPaths)[0];
      if (firstSelected) {
        quickLook(firstSelected);
      }
    } else if (e.key === 'Enter') {
      const firstSelected = Array.from(pane.selectedPaths)[0];
      const item = pane.items.find((i) => i.path === firstSelected);
      if (item) {
        handleDoubleClick(item);
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectOffset(-1);
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectOffset(1);
    } else if (e.key === 'Backspace' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      goUp(paneId);
    }
  }

  function selectOffset(offset: number) {
    if (filteredItems.length === 0) return;
    const firstSelected = Array.from(pane.selectedPaths)[0];
    const currentIndex = filteredItems.findIndex((i) => i.path === firstSelected);
    let nextIndex = currentIndex === -1 ? 0 : currentIndex + offset;
    nextIndex = Math.max(0, Math.min(filteredItems.length - 1, nextIndex));

    const nextItem = filteredItems[nextIndex];
    if (nextItem) {
      const store = paneId === 'left' ? leftPane : rightPane;
      store.update((s) => ({ ...s, selectedPaths: new Set([nextItem.path]) }));
      onSelectPreview(nextItem);
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
  bind:this={tableContainerEl}
  tabindex="0"
  class="flex-1 flex flex-col h-full bg-[var(--bg-base)] overflow-hidden outline-none {isActive ? 'ring-1 ring-[var(--accent)]' : ''}"
  on:mousedown={() => activePaneId.set(paneId)}
  on:wheel|passive={handleWheel}
  on:keydown={handleKeyDown}
  role="region"
  aria-label="File table pane"
>
  <!-- Top Filter bar inside panel -->
  <div class="flex items-center gap-2 px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)]">
    <div class="relative flex-1 flex items-center">
      <Search size={12} class="absolute left-2 text-[var(--text-muted)] pointer-events-none" />
      <input
        type="text"
        bind:value={filterText}
        placeholder="Filter (*.png, test*, *.rs *.toml)..."
        class="w-full bg-[var(--bg-panel)] text-xs text-[var(--text-primary)] rounded pl-6 pr-6 py-1 border border-[var(--border)] focus:outline-none focus:border-[var(--accent)]"
        on:input={() => {
          const store = paneId === 'left' ? leftPane : rightPane;
          store.update((s) => ({ ...s, filterQuery: filterText }));
        }}
        on:keydown={(e) => {
          if (e.key === 'Escape') {
            filterText = '';
            const store = paneId === 'left' ? leftPane : rightPane;
            store.update((s) => ({ ...s, filterQuery: '' }));
            e.stopPropagation();
          }
        }}
      />
      {#if filterText}
        <button
          class="absolute right-1.5 p-0.5 rounded-full hover:bg-[var(--bg-hover)] text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors"
          on:click={() => {
            filterText = '';
            const store = paneId === 'left' ? leftPane : rightPane;
            store.update((s) => ({ ...s, filterQuery: '' }));
          }}
          title="Rensa filter (Esc)"
        >
          <X size={12} />
        </button>
      {/if}
    </div>
    <span class="text-[11px] text-[var(--text-muted)] font-mono whitespace-nowrap">
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
          {@const isHovered = hoveredPath === item.path}
          {@const isRenaming = renamingPath === item.path}
          {@const isCasting = castingRowPath === item.path}
          {@const isLargeFile = !item.is_dir && item.size_bytes >= 50_000_000}
          {@const proportion = isLargeFile ? Math.min(100, (item.size_bytes / 1_073_741_824) * 100) : 0}

          <div
            class="grid grid-cols-12 gap-2 px-3 py-1 items-center cursor-pointer transition-all duration-300 relative {isCasting ? '-translate-y-2.5 bg-amber-500/20 shadow-lg shadow-amber-500/20 text-amber-300 ring-1 ring-amber-400' : isSelected ? 'bg-[var(--accent-subtle)] text-[var(--accent)] font-medium' : isHovered ? 'bg-[var(--bg-hover)] text-[var(--text-primary)]' : 'text-[var(--text-primary)]'}"
            on:click={(e) => handleRowClick(item, e)}
            on:dblclick={() => handleDoubleClick(item)}
            on:mouseenter={() => handleRowMouseEnter(item)}
            on:mouseleave={handleRowMouseLeave}
            on:wheel={(e) => handleRowWheel(item, e)}
            on:contextmenu={(e) => handleContextMenu(item, e)}
            role="row"
            tabindex="-1"
          >
            <!-- Name Column -->
            <div class="col-span-6 flex items-center gap-2 min-w-0">
              <svelte:component this={getFileIcon(item)} size={14} class="{getIconColor(item)} flex-shrink-0" />
              
              {#if isRenaming}
                <input
                  bind:this={renameInputEl}
                  type="text"
                  bind:value={renameInputText}
                  on:keydown={(e) => {
                    if (e.key === 'Enter') commitInlineRename();
                    else if (e.key === 'Escape') cancelInlineRename();
                  }}
                  on:blur={commitInlineRename}
                  class="flex-1 bg-[var(--bg-panel)] text-xs text-[var(--text-primary)] px-1 py-0.5 rounded border border-[var(--accent)] focus:outline-none"
                />
              {:else}
                <span class="truncate font-sans {item.is_dir ? 'font-semibold' : ''}">{item.name}</span>
                {#if isCasting}
                  <span class="px-1.5 py-0.2 rounded-full bg-amber-500 text-black text-[9px] font-bold tracking-wide flex items-center gap-1 animate-bounce shrink-0 ml-1">
                    <Rocket size={10} /> Kastad!
                  </span>
                {/if}
              {/if}
            </div>

            <!-- Size Column with Visual Bar for >= 50 MB -->
            <div class="col-span-2 relative text-right font-mono text-[11px] flex items-center justify-end">
              {#if isLargeFile}
                <div
                  class="absolute right-0 h-4 rounded opacity-25 {item.size_bytes >= 1_000_000_000 ? 'bg-[var(--accent)]' : 'bg-cyan-400'}"
                  style="width: {proportion}%"
                ></div>
              {/if}
              <span class="relative z-10 text-[var(--text-secondary)]">{item.formatted_size}</span>
            </div>

            <!-- Modified Column -->
            <div class="col-span-2 text-[var(--text-muted)] text-[11px] truncate pl-2 font-mono">
              {item.formatted_modified}
            </div>

            <!-- Permissions Column -->
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
