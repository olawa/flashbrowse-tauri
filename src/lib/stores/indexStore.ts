import { writable, get, derived } from 'svelte/store';
import { scanDirectoryIndex, getHomeDirectory } from '../invoke';
import type { DirectoryIndexGroup, FileItem, FileTypeIndexMeta } from '../types';

export const activeIndexMeta = writable<FileTypeIndexMeta | null>(null);
export const indexRootPath = writable<string>('');
export const indexedGroups = writable<DirectoryIndexGroup[]>([]);
export const selectedDirectories = writable<Set<string>>(new Set());
export const isIndexScanning = writable<boolean>(false);
export const indexSearchQuery = writable<string>('');
export const activeHighlightedParentDir = writable<string | null>(null);

export const activeIndexGroups = derived(
  [indexedGroups, selectedDirectories],
  ([$groups, $selected]) => {
    if ($selected.size === 0) return $groups;
    return $groups.filter((g) => $selected.has(g.directory_path));
  }
);

export type IndexSortBy = 'name' | 'size' | 'modified';

/** How the index list is sorted. Clicking a column header changes it. */
export const indexSortBy = writable<IndexSortBy>('name');
export const indexSortAsc = writable<boolean>(true);

/** Click a column: first click sorts by it, clicking it again reverses. */
export function sortIndexItems(column: IndexSortBy) {
  if (get(indexSortBy) === column) {
    indexSortAsc.update((v) => !v);
  } else {
    indexSortBy.set(column);
    // Size and date are most useful largest/newest first; names read A-Z.
    indexSortAsc.set(column === 'name');
  }
}

export const activeIndexFilteredItems = derived(
  [activeIndexGroups, indexSearchQuery, indexSortBy, indexSortAsc],
  ([$groups, $query, $sortBy, $sortAsc]) => {
    const allItems: FileItem[] = [];
    for (const g of $groups) {
      allItems.push(...g.items);
    }

    const q = $query.trim().toLowerCase();
    const matched = q
      ? allItems.filter(
          (item) => item.name.toLowerCase().includes(q) || item.path.toLowerCase().includes(q)
        )
      : allItems;

    const sorted = [...matched].sort((a, b) => {
      let cmp = 0;
      if ($sortBy === 'size') {
        cmp = a.size_bytes - b.size_bytes;
      } else if ($sortBy === 'modified') {
        cmp = a.modified_timestamp - b.modified_timestamp;
      } else {
        cmp = a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' });
      }
      // Ties on size or date fall back to the name, so the order is stable
      // rather than dependent on which directory was scanned first.
      if (cmp === 0 && $sortBy !== 'name') {
        cmp = a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' });
      }
      return $sortAsc ? cmp : -cmp;
    });

    return sorted;
  }
);

function getCacheKey(root: string, categoryId: string): string {
  return `flashbrowse_idx_cache_v1_${root}_${categoryId}`;
}

let currentScanId = 0;

export async function openIndexScan(meta: FileTypeIndexMeta, root?: string, forceRefresh = false) {
  const scanId = ++currentScanId;
  activeIndexMeta.set(meta);
  indexSearchQuery.set('');

  let targetRoot = root;
  if (!targetRoot) {
    const cur = get(indexRootPath);
    targetRoot = cur || (await getHomeDirectory());
  }
  indexRootPath.set(targetRoot);

  const cacheKey = getCacheKey(targetRoot, meta.id);

  // 1. Check local persistent cache if not forcing refresh
  if (!forceRefresh) {
    try {
      const cached = localStorage.getItem(cacheKey);
      if (cached) {
        const parsed: DirectoryIndexGroup[] = JSON.parse(cached);
        if (Array.isArray(parsed) && parsed.length > 0) {
          if (scanId !== currentScanId || !get(activeIndexMeta)) return;
          indexedGroups.set(parsed);
          selectedDirectories.set(new Set(parsed.map((g) => g.directory_path)));
          isIndexScanning.set(false);
          return;
        }
      }
    } catch (e) {
      console.warn('Failed to read index cache from localStorage:', e);
    }
  }

  // 2. Perform scan if no cache or forced refresh
  isIndexScanning.set(true);
  try {
    const groups = await scanDirectoryIndex(targetRoot, meta.extensions, 8);
    if (scanId !== currentScanId || !get(activeIndexMeta)) return;
    indexedGroups.set(groups);
    selectedDirectories.set(new Set(groups.map((g) => g.directory_path)));

    // Persist to localStorage
    try {
      localStorage.setItem(cacheKey, JSON.stringify(groups));
    } catch (saveErr) {
      console.warn('Failed to save index cache to localStorage:', saveErr);
    }
  } catch (err) {
    console.error('Failed to scan index:', err);
    if (scanId === currentScanId) {
      indexedGroups.set([]);
      selectedDirectories.set(new Set());
    }
  } finally {
    if (scanId === currentScanId) {
      isIndexScanning.set(false);
    }
  }
}

export async function refreshCurrentIndex() {
  const meta = get(activeIndexMeta);
  const root = get(indexRootPath);
  if (meta && root) {
    await openIndexScan(meta, root, true);
  }
}

export function closeIndexView() {
  currentScanId++;
  activeIndexMeta.set(null);
  indexedGroups.set([]);
  selectedDirectories.set(new Set());
  indexSearchQuery.set('');
  activeHighlightedParentDir.set(null);
  isIndexScanning.set(false);
}

export function selectAllIndexDirs() {
  const groups = get(indexedGroups);
  selectedDirectories.set(new Set(groups.map((g) => g.directory_path)));
}

export function deselectAllIndexDirs() {
  selectedDirectories.set(new Set());
}

export function toggleIndexDir(path: string, isCtrlOrCmd = false) {
  selectedDirectories.update((set) => {
    const next = new Set(set);
    if (isCtrlOrCmd) {
      if (next.has(path)) {
        if (next.size > 1) next.delete(path);
      } else {
        next.add(path);
      }
    } else {
      if (next.has(path) && next.size === 1) {
        return new Set(get(indexedGroups).map((g) => g.directory_path));
      }
      return new Set([path]);
    }
    return next;
  });
}
