import { writable, get, derived } from 'svelte/store';
import { scanDirectoryIndex, getHomeDirectory } from '../invoke';
import type { DirectoryIndexGroup, FileItem, FileTypeIndexMeta } from '../types';

export const activeIndexMeta = writable<FileTypeIndexMeta | null>(null);
export const indexRootPath = writable<string>('');
export const indexedGroups = writable<DirectoryIndexGroup[]>([]);
export const selectedDirectories = writable<Set<string>>(new Set());
export const isIndexScanning = writable<boolean>(false);
export const indexSearchQuery = writable<string>('');

export const activeIndexGroups = derived(
  [indexedGroups, selectedDirectories],
  ([$groups, $selected]) => {
    if ($selected.size === 0) return $groups;
    return $groups.filter((g) => $selected.has(g.directory_path));
  }
);

export const activeIndexFilteredItems = derived(
  [activeIndexGroups, indexSearchQuery],
  ([$groups, $query]) => {
    const allItems: FileItem[] = [];
    for (const g of $groups) {
      allItems.push(...g.items);
    }
    const q = $query.trim().toLowerCase();
    if (!q) return allItems;
    return allItems.filter((item) => item.name.toLowerCase().includes(q) || item.path.toLowerCase().includes(q));
  }
);

export async function openIndexScan(meta: FileTypeIndexMeta, root?: string) {
  activeIndexMeta.set(meta);
  isIndexScanning.set(true);
  indexSearchQuery.set('');

  let targetRoot = root;
  if (!targetRoot) {
    const cur = get(indexRootPath);
    targetRoot = cur || (await getHomeDirectory());
  }
  indexRootPath.set(targetRoot);

  try {
    const groups = await scanDirectoryIndex(targetRoot, meta.extensions, 8);
    indexedGroups.set(groups);
    // Default select all folders
    selectedDirectories.set(new Set(groups.map((g) => g.directory_path)));
  } catch (err) {
    console.error('Failed to scan index:', err);
    indexedGroups.set([]);
    selectedDirectories.set(new Set());
  } finally {
    isIndexScanning.set(false);
  }
}

export function closeIndexView() {
  activeIndexMeta.set(null);
  indexedGroups.set([]);
  selectedDirectories.set(new Set());
  indexSearchQuery.set('');
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
      // Single click selects only this folder, or toggles if alone
      if (next.has(path) && next.size === 1) {
        // Toggle to all
        return new Set(get(indexedGroups).map((g) => g.directory_path));
      }
      return new Set([path]);
    }
    return next;
  });
}
