import { writable, get, derived } from 'svelte/store';
import { scanDirectoryIndex, getHomeDirectory, classifyAlignments, type AlignmentClass } from '../invoke';
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

export type IndexGrouping = 'directory' | 'type';

/**
 * Virtual folders: group the indexed alignments by what they are (RNA, short
 * read, HiFi, ONT) instead of by where they sit on disk. The classification
 * comes from each file's header, so it is read on demand the first time.
 */
export const indexGrouping = writable<IndexGrouping>('directory');
export const alignmentClasses = writable<Record<string, AlignmentClass>>({});
export const selectedTypes = writable<Set<string>>(new Set());
export const isClassifying = writable<boolean>(false);
export const classifyError = writable<string>('');

const ALIGNMENT_RE = /\.(bam|cram)$/i;

/** Virtual folders with their counts, ordered with the useful ones first. */
export const indexTypeGroups = derived(
  [indexedGroups, alignmentClasses],
  ([$groups, $classes]) => {
    const counts = new Map<string, { id: string; label: string; count: number }>();
    for (const g of $groups) {
      for (const item of g.items) {
        if (!ALIGNMENT_RE.test(item.path)) continue;
        const cls = $classes[item.path];
        const id = cls?.type_id ?? 'unclassified';
        const label = cls?.type_label ?? 'Inte läst än';
        const entry = counts.get(id) ?? { id, label, count: 0 };
        entry.count += 1;
        counts.set(id, entry);
      }
    }

    const order = ['rna', 'sr', 'hifi', 'ont', 'lr', 'unknown', 'unclassified'];
    return Array.from(counts.values()).sort(
      (a, b) => order.indexOf(a.id) - order.indexOf(b.id) || a.label.localeCompare(b.label)
    );
  }
);

/** Read the headers of every indexed alignment and remember what they are. */
export async function classifyIndexedAlignments() {
  const groups = get(indexedGroups);
  const paths = groups
    .flatMap((g) => g.items)
    .filter((i) => ALIGNMENT_RE.test(i.path))
    .map((i) => i.path);

  if (paths.length === 0) {
    alignmentClasses.set({});
    return;
  }

  isClassifying.set(true);
  classifyError.set('');
  try {
    const classes = await classifyAlignments(paths);
    const byPath: Record<string, AlignmentClass> = {};
    for (const c of classes) byPath[c.path] = c;
    alignmentClasses.set(byPath);
  } catch (e: any) {
    classifyError.set(String(e));
  } finally {
    isClassifying.set(false);
  }
}

export function toggleIndexType(id: string) {
  selectedTypes.update((s) => {
    const next = new Set(s);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  });
}

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
  [
    activeIndexGroups,
    indexSearchQuery,
    indexSortBy,
    indexSortAsc,
    indexGrouping,
    selectedTypes,
    alignmentClasses,
  ],
  ([$groups, $query, $sortBy, $sortAsc, $grouping, $types, $classes]) => {
    let allItems: FileItem[] = [];
    for (const g of $groups) {
      allItems.push(...g.items);
    }

    // In virtual-folder mode the selected types replace the directory choice.
    if ($grouping === 'type' && $types.size > 0) {
      allItems = allItems.filter((item) => {
        const id = $classes[item.path]?.type_id ?? 'unclassified';
        return $types.has(id);
      });
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
        const sizeA = typeof a.size_bytes === 'number' ? a.size_bytes : 0;
        const sizeB = typeof b.size_bytes === 'number' ? b.size_bytes : 0;
        cmp = sizeA < sizeB ? -1 : sizeA > sizeB ? 1 : 0;
      } else if ($sortBy === 'modified') {
        const modA = typeof a.modified_timestamp === 'number' ? a.modified_timestamp : 0;
        const modB = typeof b.modified_timestamp === 'number' ? b.modified_timestamp : 0;
        cmp = modA < modB ? -1 : modA > modB ? 1 : 0;
      } else {
        const nameA = a.name || '';
        const nameB = b.name || '';
        cmp = nameA.localeCompare(nameB, undefined, { numeric: true, sensitivity: 'base' });
      }
      // Ties on size or date fall back to the name, so the order is stable
      // rather than dependent on which directory was scanned first.
      if (cmp === 0 && $sortBy !== 'name') {
        const nameA = a.name || '';
        const nameB = b.name || '';
        cmp = nameA.localeCompare(nameB, undefined, { numeric: true, sensitivity: 'base' });
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
  // A new scan means a new set of files; drop what was classified before.
  alignmentClasses.set({});
  selectedTypes.set(new Set());

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
