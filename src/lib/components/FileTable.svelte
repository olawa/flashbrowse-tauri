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
    isInspectorLocked,
    castToSecondaryInspector,
    triggerInspectorScroll,
    reloadPane,
  } from '../stores/navigation';
  import { isKidsMode } from '../stores/theme';
  import { openInDefault, quickLook, renameItem, trashItems } from '../invoke';
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
  let tableContainerEl: HTMLElement;

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

  // Marquee / Rubberband drag-selection state
  let isMarqueeDragging = false;
  let marqueeStart = { x: 0, y: 0 };
  let marqueeCurrent = { x: 0, y: 0 };
  let rowElements: Map<string, HTMLElement> = new Map();

  function registerRow(node: HTMLElement, path: string) {
    rowElements.set(path, node);
    return {
      update(newPath: string) {
        if (newPath !== path) {
          rowElements.delete(path);
          path = newPath;
          rowElements.set(path, node);
        }
      },
      destroy() {
        rowElements.delete(path);
      },
    };
  }

  $: marqueeRect = {
    left: Math.min(marqueeStart.x, marqueeCurrent.x),
    top: Math.min(marqueeStart.y, marqueeCurrent.y),
    width: Math.abs(marqueeCurrent.x - marqueeStart.x),
    height: Math.abs(marqueeCurrent.y - marqueeStart.y),
  };

  function handleContainerMouseDown(e: MouseEvent) {
    if (e.button !== 0) return; // only left mouse button
    const target = e.target as HTMLElement;
    if (target.closest('input, button, [role="button"], a')) return;

    activePaneId.set(paneId);
    const store = paneId === 'left' ? leftPane : rightPane;

    const rowEl = target.closest('[data-row-path]') as HTMLElement;
    if (!rowEl) {
      // Clicked on empty space: deselect all unless Shift/Cmd is held
      if (!e.metaKey && !e.ctrlKey && !e.shiftKey) {
        store.update((s) => ({ ...s, selectedPaths: new Set() }));
        onSelectPreview(null);
      }
    }

    const containerRect = tableContainerEl?.getBoundingClientRect();
    if (!containerRect) return;

    marqueeStart = {
      x: e.clientX - containerRect.left + (tableContainerEl?.scrollLeft || 0),
      y: e.clientY - containerRect.top + (tableContainerEl?.scrollTop || 0),
    };
    marqueeCurrent = { ...marqueeStart };

    function onMouseMove(moveEvent: MouseEvent) {
      const curX = moveEvent.clientX - containerRect.left + (tableContainerEl?.scrollLeft || 0);
      const curY = moveEvent.clientY - containerRect.top + (tableContainerEl?.scrollTop || 0);
      const dist = Math.hypot(curX - marqueeStart.x, curY - marqueeStart.y);

      if (dist > 4) {
        isMarqueeDragging = true;
        marqueeCurrent = { x: curX, y: curY };

        const selLeft = Math.min(marqueeStart.x, curX);
        const selRight = Math.max(marqueeStart.x, curX);
        const selTop = Math.min(marqueeStart.y, curY);
        const selBottom = Math.max(marqueeStart.y, curY);

        const newSelected = new Set(
          moveEvent.metaKey || moveEvent.ctrlKey || moveEvent.shiftKey ? pane.selectedPaths : []
        );

        rowElements.forEach((el, path) => {
          const r = el.getBoundingClientRect();
          const rTop = r.top - containerRect.top + tableContainerEl.scrollTop;
          const rBottom = r.bottom - containerRect.top + tableContainerEl.scrollTop;
          const rLeft = r.left - containerRect.left + tableContainerEl.scrollLeft;
          const rRight = r.right - containerRect.left + tableContainerEl.scrollLeft;

          const overlaps = selLeft < rRight && selRight > rLeft && selTop < rBottom && selBottom > rTop;
          if (overlaps) {
            newSelected.add(path);
          }
        });

        store.update((s) => ({ ...s, selectedPaths: newSelected }));
      }
    }

    function onMouseUp() {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
      setTimeout(() => {
        isMarqueeDragging = false;
      }, 50);
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }


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
    if (['bam', 'cram', 'sam'].includes(ext)) return 'text-emerald-400';
    if (['vcf', 'bcf'].includes(ext)) return 'text-purple-400';
    if (['fastq', 'fq'].includes(ext)) return 'text-cyan-400';
    if (['csv', 'tsv', 'xlsx', 'tab'].includes(ext)) return 'text-blue-400';
    if (['rs', 'py', 'js', 'ts', 'c', 'cpp', 'swift'].includes(ext)) return 'text-yellow-400';
    if (['png', 'jpg', 'jpeg', 'webp', 'svg'].includes(ext)) return 'text-pink-400';
    if (['zip', 'tar', 'gz'].includes(ext)) return 'text-red-400';
    return 'text-slate-400';
  }

  // MARK: - Single / Double Click & Shift / Cmd Multi-Selection
  function handleRowClick(item: FileItem, event: MouseEvent) {
    activePaneId.set(paneId);
    const store = paneId === 'left' ? leftPane : rightPane;
    const now = Date.now();
    const timeSinceLastClick = now - lastClickTimestamp;

    // 1. Shift + Click: Range Selection from anchor
    if (event.shiftKey) {
      const anchorPath = lastClickedPath || (filteredItems.length > 0 ? filteredItems[0].path : null);
      let anchorIdx = anchorPath ? filteredItems.findIndex((i) => i.path === anchorPath) : 0;
      let targetIdx = filteredItems.findIndex((i) => i.path === item.path);

      if (anchorIdx === -1) anchorIdx = 0;
      if (targetIdx === -1) targetIdx = 0;

      const minIdx = Math.min(anchorIdx, targetIdx);
      const maxIdx = Math.max(anchorIdx, targetIdx);

      const rangePaths = new Set(
        event.metaKey || event.ctrlKey ? pane.selectedPaths : []
      );
      for (let i = minIdx; i <= maxIdx; i++) {
        rangePaths.add(filteredItems[i].path);
      }

      store.update((s) => ({ ...s, selectedPaths: rangePaths }));
      lastClickTimestamp = now;
      onSelectPreview(item);
      return;
    }

    // 2. Cmd + Click / Ctrl + Click: Toggle individual item
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

  // MARK: - Smart Hover Live Preview with Lock and Cmd Support
  function handleRowMouseEnter(item: FileItem, e?: MouseEvent) {
    hoveredPath = item.path;
    clearTimeout(hoverTimer);

    // If Inspector is locked or user is holding Cmd / Ctrl, do NOT change preview!
    if ($isInspectorLocked || e?.metaKey || e?.ctrlKey) {
      return;
    }

    if ($smartHoverPreview) {
      hoverTimer = setTimeout(() => {
        if (hoveredPath === item.path && !$isInspectorLocked) {
          activeHoveredItem.set(item);
          onSelectPreview(item);
        }
      }, 150);
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

  // Remote Inspector Scroll & Cast Gesture
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

    // 1. If Alt is held with swipe UP, trigger cast
    if (e.altKey && e.deltaY < -20) {
      e.preventDefault();
      handleCastItem(item);
      return;
    }

    // 2. Cmd + scroll on row drives remote inspector scrolling!
    // Without Cmd, standard scrolling moves the file list up/down naturally.
    if (e.metaKey && Math.abs(e.deltaY) > 0) {
      e.preventDefault();
      triggerInspectorScroll(e.deltaY);
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

  // MARK: - Keyboard Handling (Space for QuickLook, Arrows, Enter, Cmd+Backspace for Trash, Cmd+A for Select All, Cmd+Up for GoUp)
  async function handleKeyDown(e: KeyboardEvent) {
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

    // Cmd + Backspace or Delete -> Trash selected items!
    if ((e.metaKey || e.ctrlKey) && (e.key === 'Backspace' || e.key === 'Delete')) {
      e.preventDefault();
      const paths = Array.from(pane.selectedPaths);
      if (paths.length > 0) {
        try {
          await trashItems(paths);
          reloadPane(paneId);
        } catch (err) {
          console.error('Failed to trash items:', err);
        }
      }
      return;
    }

    // Cmd + A -> Select all filtered items
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'a') {
      e.preventDefault();
      const store = paneId === 'left' ? leftPane : rightPane;
      store.update((s) => ({
        ...s,
        selectedPaths: new Set(filteredItems.map((i) => i.path)),
      }));
      return;
    }

    // Esc -> Clear selection
    if (e.key === 'Escape' && !pane.filterQuery) {
      const store = paneId === 'left' ? leftPane : rightPane;
      store.update((s) => ({ ...s, selectedPaths: new Set() }));
      return;
    }

    // Cmd + ArrowUp -> Go up to enclosing directory
    if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowUp') {
      e.preventDefault();
      goUp(paneId);
      return;
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
  tabindex="0"
  class="flex-1 flex flex-col h-full bg-[var(--bg-base)] overflow-hidden outline-none {isActive ? 'ring-1 ring-[var(--accent)]' : ''}"
  on:mousedown={() => activePaneId.set(paneId)}
  on:wheel|passive={handleWheel}
  on:keydown={handleKeyDown}
  role="region"
  aria-label="File table for {paneId} pane"
>
  <!-- Search / Quick Filter Bar -->
  <div class="px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] flex items-center justify-between gap-2 shrink-0">
    <div class="relative flex-1 flex items-center">
      <Search size={13} class="text-[var(--text-muted)] absolute left-2 pointer-events-none" />
      <input
        type="text"
        bind:value={filterText}
        on:input={() => {
          const store = paneId === 'left' ? leftPane : rightPane;
          store.update((s) => ({ ...s, filterQuery: filterText }));
        }}
        placeholder="Filter... (e.g. *.png, test, rs)"
        class="w-full bg-[var(--bg-panel)] text-xs text-[var(--text-primary)] pl-7 pr-6 py-1 rounded border border-[var(--border)] focus:border-[var(--accent)] focus:outline-none placeholder:text-[var(--text-muted)] font-mono"
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
    <div
      bind:this={tableContainerEl}
      on:mousedown={handleContainerMouseDown}
      class="flex-1 overflow-y-auto flex flex-col text-xs font-mono select-none relative"
    >
      <!-- Rubberband / Marquee selection overlay box -->
      {#if isMarqueeDragging && marqueeRect.width > 3 && marqueeRect.height > 3}
        <div
          class="absolute border border-[var(--accent)] bg-[var(--accent)]/20 rounded pointer-events-none z-30 transition-none"
          style="left: {marqueeRect.left}px; top: {marqueeRect.top}px; width: {marqueeRect.width}px; height: {marqueeRect.height}px;"
        ></div>
      {/if}

      <!-- Table Header -->
      <div class="grid grid-cols-12 gap-2 px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] text-[var(--text-muted)] font-sans font-semibold text-[11px] sticky top-0 z-10">
        <button
          class="col-span-8 flex items-center gap-1 text-left hover:text-[var(--text-primary)]"
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
          class="col-span-2 flex items-center gap-1 justify-end hover:text-[var(--text-primary)] pr-1"
          on:click={() => sortPaneItems(paneId, 'modified')}
        >
          <span>Modified</span>
          {#if pane.sortBy === 'modified'}
            {#if pane.sortAsc}<ArrowUp size={11} />{:else}<ArrowDown size={11} />{/if}
          {/if}
        </button>
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
            data-row-path={item.path}
            use:registerRow={item.path}
            class="grid grid-cols-12 gap-2 px-3 py-1 items-center cursor-pointer transition-all duration-300 relative {isCasting ? '-translate-y-2.5 bg-amber-500/20 shadow-lg shadow-amber-500/20 text-amber-300 ring-1 ring-amber-400' : isSelected ? 'bg-[var(--accent-subtle)] text-[var(--accent)] font-medium' : isHovered ? 'bg-[var(--bg-hover)] text-[var(--text-primary)]' : 'text-[var(--text-primary)]'}"
            on:click={(e) => handleRowClick(item, e)}
            on:dblclick={() => handleDoubleClick(item)}
            on:mouseenter={(e) => handleRowMouseEnter(item, e)}
            on:mouseleave={handleRowMouseLeave}
            on:wheel={(e) => handleRowWheel(item, e)}
            on:contextmenu={(e) => handleContextMenu(item, e)}
            role="row"
            tabindex="-1"
          >
            <!-- Name Column -->
            <div class="col-span-8 flex items-center gap-2 min-w-0">
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
            <div class="col-span-2 text-[var(--text-muted)] text-[11px] truncate text-right pr-1 font-mono">
              {item.formatted_modified}
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
