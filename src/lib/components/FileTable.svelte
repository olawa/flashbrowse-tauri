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
    isDualPane,
    transferBetweenPanes,
    isTransferring,
    transferStatus,
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
    LayoutList,
    FileStack,
    Dna,
    ArrowDownToLine,
    ArrowRightLeft,
  } from 'lucide-svelte';

  export let paneId: 'left' | 'right' = 'left';
  export let onSelectPreview: (item: FileItem | null) => void;

  let isDragOver = false;

  $: pane = paneId === 'left' ? $leftPane : $rightPane;
  $: isActive = $activePaneId === paneId;

  let filterText = '';
  $: if (pane.filterQuery !== undefined && filterText !== pane.filterQuery) {
    filterText = pane.filterQuery;
  }
  let contextMenuItem: FileItem | null = null;
  let contextMenuPos = { x: 0, y: 0 };
  let tableContainerEl: HTMLElement;

  let isGroupedMode = false;
  let hoveredGroupId: string | null = null;

  interface FileTypeGroup {
    id: string;
    label: string;
    sublabel: string;
    icon: any;
    color: string;
    count: number;
    totalBytes: number;
    formattedSize: string;
    items: FileItem[];
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  // Group items by file type clusters
  $: fileTypeGroups = (() => {
    const map = new Map<string, { label: string; sublabel: string; icon: any; color: string; items: FileItem[] }>();

    for (const item of filteredItems) {
      if (item.is_dir) {
        const entry = map.get('dir') || {
          label: 'Mappar',
          sublabel: 'Undermappar',
          icon: Folder,
          color: 'text-amber-400 bg-amber-950/40 border-amber-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('dir', entry);
        continue;
      }

      const ext = item.extension.toLowerCase();
      const name = item.name.toLowerCase();

      if (ext === 'bam' || ext === 'cram' || ext === 'sam' || name.endsWith('.bam') || name.endsWith('.cram')) {
        const entry = map.get('bam') || {
          label: 'BAM / CRAM Alignments',
          sublabel: '.bam, .cram, .sam',
          icon: Dna,
          color: 'text-emerald-400 bg-emerald-950/40 border-emerald-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('bam', entry);
      } else if (ext === 'vcf' || ext === 'bcf' || name.endsWith('.vcf.gz') || name.endsWith('.bcf')) {
        const entry = map.get('vcf') || {
          label: 'VCF / BCF Varianter',
          sublabel: '.vcf, .vcf.gz, .bcf',
          icon: Dna,
          color: 'text-purple-400 bg-purple-950/40 border-purple-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('vcf', entry);
      } else if (ext === 'fastq' || ext === 'fq' || name.endsWith('.fastq.gz') || name.endsWith('.fq.gz')) {
        const entry = map.get('fastq') || {
          label: 'FASTQ Sekvensdata',
          sublabel: '.fastq, .fq.gz',
          icon: Dna,
          color: 'text-cyan-400 bg-cyan-950/40 border-cyan-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('fastq', entry);
      } else if (['fasta', 'fa', 'fna', 'faa'].includes(ext) || name.endsWith('.fa.gz')) {
        const entry = map.get('fasta') || {
          label: 'FASTA Referenser',
          sublabel: '.fa, .fasta, .fna',
          icon: Dna,
          color: 'text-teal-400 bg-teal-950/40 border-teal-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('fasta', entry);
      } else if (['tsv', 'csv', 'tab', 'xlsx', 'xls'].includes(ext) || name.endsWith('.tsv.gz') || name.endsWith('.csv.gz')) {
        const entry = map.get('table') || {
          label: 'Tabeller & Kalkylark',
          sublabel: '.tsv, .csv, .xlsx',
          icon: FileSpreadsheet,
          color: 'text-blue-400 bg-blue-950/40 border-blue-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('table', entry);
      } else if (['rs', 'py', 'ts', 'js', 'sh', 'c', 'cpp', 'swift', 'r', 'json', 'yaml', 'toml'].includes(ext)) {
        const entry = map.get('code') || {
          label: 'Kod & Skript',
          sublabel: '.py, .sh, .rs, .r, .json',
          icon: FileCode,
          color: 'text-yellow-400 bg-yellow-950/40 border-yellow-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('code', entry);
      } else if (['png', 'jpg', 'jpeg', 'webp', 'svg', 'gif'].includes(ext)) {
        const entry = map.get('image') || {
          label: 'Bilder',
          sublabel: '.png, .jpg, .svg',
          icon: FileImage,
          color: 'text-pink-400 bg-pink-950/40 border-pink-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('image', entry);
      } else if (['txt', 'md', 'pdf', 'log', 'doc', 'docx'].includes(ext)) {
        const entry = map.get('doc') || {
          label: 'Dokument & Loggar',
          sublabel: '.txt, .md, .pdf, .log',
          icon: FileText,
          color: 'text-slate-300 bg-slate-800/40 border-slate-700/60',
          items: [],
        };
        entry.items.push(item);
        map.set('doc', entry);
      } else if (['zip', 'tar', 'gz', 'tgz', 'bz2', '7z'].includes(ext)) {
        const entry = map.get('archive') || {
          label: 'Arkiv & Komprimerat',
          sublabel: '.zip, .tar.gz, .gz',
          icon: FileArchive,
          color: 'text-orange-400 bg-orange-950/40 border-orange-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set('archive', entry);
      } else {
        const genericKey = ext ? `ext_${ext}` : 'other';
        const entry = map.get(genericKey) || {
          label: ext ? `.${ext.toUpperCase()} filer` : 'Övriga filer',
          sublabel: ext ? `.${ext}` : 'Filer utan ändelse',
          icon: File,
          color: 'text-indigo-400 bg-indigo-950/40 border-indigo-800/60',
          items: [],
        };
        entry.items.push(item);
        map.set(genericKey, entry);
      }
    }

    const groups: FileTypeGroup[] = [];
    map.forEach((val, key) => {
      const totalBytes = val.items.reduce((acc, i) => acc + (i.is_dir ? 0 : i.size_bytes), 0);
      groups.push({
        id: key,
        label: val.label,
        sublabel: val.sublabel,
        icon: val.icon,
        color: val.color,
        count: val.items.length,
        totalBytes,
        formattedSize: formatBytes(totalBytes),
        items: val.items,
      });
    });

    return groups.sort((a, b) => {
      if (a.id === 'dir') return -1;
      if (b.id === 'dir') return 1;
      return b.count - a.count;
    });
  })();

  function handleGroupHover(group: FileTypeGroup) {
    hoveredGroupId = group.id;
    if ($isDualPane) {
      const otherPaneId = paneId === 'left' ? 'right' : 'left';
      const otherStore = otherPaneId === 'left' ? leftPane : rightPane;
      otherStore.update((s) => ({
        ...s,
        items: group.items,
        currentPath: pane.currentPath,
        filterQuery: '',
        selectedPaths: new Set(group.items.length > 0 ? [group.items[0].path] : []),
      }));
      if (group.items.length > 0) {
        onSelectPreview(group.items[0]);
      }
    } else if (group.items.length > 0) {
      onSelectPreview(group.items[0]);
    }
  }

  function handleGroupMouseLeave() {
    hoveredGroupId = null;
  }

  function handleGroupClick(group: FileTypeGroup) {
    const store = paneId === 'left' ? leftPane : rightPane;
    store.update((s) => ({
      ...s,
      selectedPaths: new Set(group.items.map((i) => i.path)),
    }));
    if (group.items.length > 0) {
      onSelectPreview(group.items[0]);
    }
  }

  function handleGroupDblClick(group: FileTypeGroup) {
    filterText = group.sublabel.split(',')[0].trim().replace(/^\./, '*.');
    const store = paneId === 'left' ? leftPane : rightPane;
    store.update((s) => ({ ...s, filterQuery: filterText }));
    isGroupedMode = false;
  }

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

  $: filteredItems = (() => {
    let result = pane.items.filter((item) => matchFilter(item.name, filterText));
    const { sortBy, sortAsc } = pane;

    return result.sort((a, b) => {
      if (a.is_dir !== b.is_dir) {
        return b.is_dir ? 1 : -1;
      }

      let cmp = 0;
      if (sortBy === 'name') {
        cmp = a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' });
      } else if (sortBy === 'size') {
        cmp = a.size_bytes - b.size_bytes;
      } else if (sortBy === 'modified') {
        cmp = a.modified_timestamp - b.modified_timestamp;
      } else if (sortBy === 'type') {
        cmp = a.extension.localeCompare(b.extension);
      }

      return sortAsc ? cmp : -cmp;
    });
  })();

  // MARK: - Drag and Drop between Panels
  function handleRowDragStart(item: FileItem, e: DragEvent) {
    const selected = pane.selectedPaths.has(item.path)
      ? Array.from(pane.selectedPaths)
      : [item.path];
    if (e.dataTransfer) {
      e.dataTransfer.setData(
        'application/json',
        JSON.stringify({ sourcePaneId: paneId, paths: selected })
      );
      e.dataTransfer.effectAllowed = 'copy';
    }
  }

  function handleContainerDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'copy';
    }
    isDragOver = true;
  }

  function handleContainerDragLeave() {
    isDragOver = false;
  }

  async function handleContainerDrop(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
    if (!e.dataTransfer) return;
    try {
      const raw = e.dataTransfer.getData('application/json');
      if (raw) {
        const data = JSON.parse(raw);
        if (data.sourcePaneId && data.sourcePaneId !== paneId && data.paths?.length > 0) {
          await transferBetweenPanes(data.sourcePaneId, paneId, data.paths);
        }
      }
    } catch (err) {
      console.error('Drop transfer failed:', err);
    }
  }

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

    // Cmd + < / Cmd + > / Cmd + § / Cmd + ` / Cmd + [ / Cmd + ] / Cmd + Alt + ArrowLeft/Right / Ctrl + Tab: Switch active pane focus
    if (
      ((e.metaKey || e.ctrlKey) && (e.key === '<' || e.key === '>' || e.key === '§' || e.key === '`' || e.key === '[' || e.key === ']')) ||
      ((e.metaKey || e.ctrlKey) && e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) ||
      (e.ctrlKey && e.key === 'Tab')
    ) {
      e.preventDefault();
      activePaneId.update((p) => (p === 'left' ? 'right' : 'left'));
      return;
    }

    // F5 or Cmd + E -> Transfer selected items to other pane!
    if (e.key === 'F5' || ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'e')) {
      e.preventDefault();
      const otherPaneId = paneId === 'left' ? 'right' : 'left';
      await transferBetweenPanes(paneId, otherPaneId);
      return;
    }

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
  <!-- Search / Quick Filter & View Mode Bar -->
  <div class="px-2.5 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] flex items-center justify-between gap-2 shrink-0">
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

    <!-- View Mode Selector (List vs Grouped Clusters) -->
    <div class="flex items-center gap-1 shrink-0">
      <button
        class="px-2 py-1 rounded text-[11px] font-medium border flex items-center gap-1 transition-colors {!isGroupedMode ? 'bg-[var(--accent)] text-white border-[var(--accent)] font-semibold shadow-sm' : 'bg-[var(--bg-panel)] text-slate-400 border-[var(--border)] hover:text-white'}"
        on:click={() => (isGroupedMode = false)}
        title="Vanlig fillista"
      >
        <LayoutList size={11} />
        <span>Filer ({filteredItems.length})</span>
      </button>

      <button
        class="px-2 py-1 rounded text-[11px] font-medium border flex items-center gap-1 transition-colors {isGroupedMode ? 'bg-purple-600 text-white border-purple-500 font-bold shadow-sm' : 'bg-[var(--bg-panel)] text-slate-400 border-[var(--border)] hover:text-white'}"
        on:click={() => (isGroupedMode = true)}
        title="Gruppera filer per typ (BAM, VCF, FASTQ, etc.)"
      >
        <FileStack size={11} />
        <span>Grupper ({fileTypeGroups.length})</span>
      </button>
    </div>
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
  {:else if isGroupedMode}
    <!-- Grouped Clusters View -->
    <div class="flex-1 overflow-y-auto p-2.5 space-y-2 select-none bg-[var(--bg-base)]">
      <div class="flex items-center justify-between text-[11px] text-slate-400 px-1">
        <span>Klustrade filtyper ({fileTypeGroups.length} grupper)</span>
        <span class="text-[10px] font-mono opacity-70">
          {$isDualPane ? 'Hovra för att lista i högra panelen' : 'Klicka för att markera'}
        </span>
      </div>

      {#each fileTypeGroups as group}
        {@const isHovered = hoveredGroupId === group.id}
        {@const isAllSelected = group.items.length > 0 && group.items.every(i => pane.selectedPaths.has(i.path))}
        {@const proportion = Math.min(100, (group.count / Math.max(1, filteredItems.length)) * 100)}
        <div
          class="p-2.5 rounded-xl border transition-all cursor-pointer flex items-center justify-between gap-3 relative overflow-hidden group {isAllSelected ? 'bg-purple-950/40 border-purple-500 ring-2 ring-purple-500/50' : isHovered ? 'bg-[#181d28] border-purple-500/60 shadow-lg ring-1 ring-purple-500/40' : 'bg-[#11141b] border-[#222938] hover:bg-[#161a22]'}"
          on:mouseenter={() => handleGroupHover(group)}
          on:mouseleave={handleGroupMouseLeave}
          on:click={() => handleGroupClick(group)}
          on:dblclick={() => handleGroupDblClick(group)}
          role="button"
          tabindex="-1"
        >
          <!-- Background Proportion Bar -->
          <div
            class="absolute left-0 bottom-0 top-0 opacity-10 bg-purple-500 transition-all pointer-events-none"
            style="width: {proportion}%"
          ></div>

          <!-- Left info -->
          <div class="flex items-center gap-2.5 min-w-0 flex-1 relative z-10">
            <div class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 border {group.color}">
              <svelte:component this={group.icon} size={16} />
            </div>
            <div class="flex flex-col min-w-0">
              <div class="flex items-center gap-2">
                <span class="font-semibold text-xs text-white truncate">{group.label}</span>
                <span class="px-1.5 py-0.2 rounded-full bg-[#202738] text-purple-300 font-mono text-[10px] font-bold border border-[#2b354c]">
                  {group.count} {group.count === 1 ? 'fil' : 'filer'}
                </span>
              </div>
              <span class="text-[10px] text-slate-400 font-mono truncate">{group.sublabel}</span>
            </div>
          </div>

          <!-- Right Size & Hint -->
          <div class="flex items-center gap-2 shrink-0 relative z-10">
            <span class="font-mono text-xs text-slate-300 font-semibold">{group.formattedSize}</span>
            <span class="text-[10px] text-purple-400 opacity-0 group-hover:opacity-100 transition-opacity font-medium hidden sm:inline">
              {$isDualPane ? '→ Höger panel' : 'Klicka'}
            </span>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <!-- Pro Table View -->
    <div
      bind:this={tableContainerEl}
      on:mousedown={handleContainerMouseDown}
      on:dragover={handleContainerDragOver}
      on:dragleave={handleContainerDragLeave}
      on:drop={handleContainerDrop}
      class="flex-1 overflow-y-auto flex flex-col text-xs font-mono select-none relative {isDragOver ? 'ring-2 ring-cyan-500 ring-inset bg-cyan-950/20' : ''}"
    >
      <!-- Drop Overlay -->
      {#if isDragOver}
        <div class="absolute inset-0 z-40 bg-cyan-950/85 border-2 border-cyan-400 border-dashed rounded m-1 flex flex-col items-center justify-center text-cyan-200 pointer-events-none backdrop-blur-sm animate-pulse">
          <ArrowDownToLine size={28} class="text-cyan-400 mb-1.5" />
          <span class="font-bold text-xs">Släpp för att föra över till denna mapp</span>
          <span class="text-[10px] text-slate-300 font-mono mt-0.5">{pane.currentPath}</span>
        </div>
      {/if}

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
          class="col-span-8 flex items-center gap-1 text-left hover:text-[var(--text-primary)] transition-colors"
          on:click={() => sortPaneItems(paneId, 'name')}
          title="Sortera efter namn"
        >
          <span>Namn</span>
          {#if pane.sortBy === 'name'}
            {#if pane.sortAsc}<ArrowUp size={11} class="text-[var(--accent)]" />{:else}<ArrowDown size={11} class="text-[var(--accent)]" />{/if}
          {/if}
        </button>

        <button
          class="col-span-2 flex items-center gap-1 justify-end hover:text-[var(--text-primary)] transition-colors"
          on:click={() => sortPaneItems(paneId, 'size')}
          title="Sortera efter storlek"
        >
          <span>Storlek</span>
          {#if pane.sortBy === 'size'}
            {#if pane.sortAsc}<ArrowUp size={11} class="text-[var(--accent)]" />{:else}<ArrowDown size={11} class="text-[var(--accent)]" />{/if}
          {/if}
        </button>

        <button
          class="col-span-2 flex items-center gap-1 justify-end hover:text-[var(--text-primary)] transition-colors pr-1"
          on:click={() => sortPaneItems(paneId, 'modified')}
          title="Sortera efter ändringsdatum"
        >
          <span>Ändrad</span>
          {#if pane.sortBy === 'modified'}
            {#if pane.sortAsc}<ArrowUp size={11} class="text-[var(--accent)]" />{:else}<ArrowDown size={11} class="text-[var(--accent)]" />{/if}
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
            draggable="true"
            on:dragstart={(e) => handleRowDragStart(item, e)}
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
