import { writable, get } from 'svelte/store';
import { listDirectory, getHomeDirectory, sshListDirectory } from '../invoke';
import type { FileItem } from '../types';

export interface PaneState {
  currentPath: string;
  history: string[];
  historyIndex: number;
  items: FileItem[];
  selectedPaths: Set<string>;
  focusedIndex: number;
  isLoading: boolean;
  filterQuery: string;
  sortBy: 'name' | 'size' | 'modified' | 'type';
  sortAsc: boolean;
  isSSH: boolean;
  sshHost: string;
  errorMessage?: string;
}

function createDefaultPaneState(): PaneState {
  return {
    currentPath: '',
    history: [],
    historyIndex: -1,
    items: [],
    selectedPaths: new Set(),
    focusedIndex: 0,
    isLoading: false,
    filterQuery: '',
    sortBy: 'name',
    sortAsc: true,
    isSSH: false,
    sshHost: 'marvin.cgu.igp.uu.se',
  };
}

export const leftPane = writable<PaneState>(createDefaultPaneState());
export const rightPane = writable<PaneState>(createDefaultPaneState());
export type InspectorPreset = 'center' | 'right' | 'dual' | 'none';

export const activePaneId = writable<'left' | 'right'>('left');
export const isDualPane = writable<boolean>(true);
export const isDualInspector = writable<boolean>(false);
export const inspectorPreset = writable<InspectorPreset>('center');
export const isInspectorDetached = writable<boolean>(false);
export const isSecondaryInspectorOpen = writable<boolean>(false);
export const lastCastedItem = writable<FileItem | null>(null);
export const showHiddenFiles = writable<boolean>(false);
export const clickMode = writable<'folders-only' | 'always' | 'double-click'>('folders-only');
export const smartHoverPreview = writable<boolean>(true);
export const activeHoveredItem = writable<FileItem | null>(null);

// Remote Inspector Scroll Channel
export const inspectorScroll = writable<{ deltaY: number; pulse: number }>({ deltaY: 0, pulse: 0 });

export function triggerInspectorScroll(deltaY: number) {
  inspectorScroll.update((s) => ({ deltaY, pulse: s.pulse + 1 }));
}

export async function castToSecondaryInspector(item: FileItem) {
  lastCastedItem.set(item);
  isSecondaryInspectorOpen.set(true);
  try {
    const { toggleDetachedInspector } = await import('../invoke');
    await toggleDetachedInspector(item.path);
  } catch (err) {
    console.error('Failed to cast to secondary inspector:', err);
  }
}

export async function initNavigation() {
  const home = await getHomeDirectory();
  await navigatePane('left', home);
  await navigatePane('right', home);
}

export async function navigatePane(
  paneId: 'left' | 'right',
  path: string,
  addToHistory = true
) {
  const store = paneId === 'left' ? leftPane : rightPane;
  const current = get(store);
  const showHidden = get(showHiddenFiles);

  store.update((s) => ({
    ...s,
    isLoading: true,
    errorMessage: undefined,
  }));

  try {
    let items: FileItem[] = [];
    if (current.isSSH) {
      items = await sshListDirectory(current.sshHost, path);
    } else {
      items = await listDirectory(path, showHidden);
    }

    store.update((s) => {
      let newHistory = s.history;
      let newIndex = s.historyIndex;

      if (addToHistory && s.currentPath !== path) {
        newHistory = s.history.slice(0, s.historyIndex + 1);
        newHistory.push(path);
        newIndex = newHistory.length - 1;
      }

      const newSelected = new Set<string>();
      if (items.length > 0) {
        newSelected.add(items[0].path);
      }

      return {
        ...s,
        currentPath: path,
        history: newHistory,
        historyIndex: newIndex,
        items,
        selectedPaths: newSelected,
        focusedIndex: 0,
        isLoading: false,
        filterQuery: '',
      };
    });
  } catch (err: any) {
    store.update((s) => ({
      ...s,
      isLoading: false,
      errorMessage: String(err),
    }));
  }
}

export async function refreshPane(paneId: 'left' | 'right') {
  const store = paneId === 'left' ? leftPane : rightPane;
  const current = get(store);
  if (current.currentPath) {
    await navigatePane(paneId, current.currentPath, false);
  }
}

export async function goBack(paneId: 'left' | 'right') {
  const store = paneId === 'left' ? leftPane : rightPane;
  const current = get(store);
  if (current.historyIndex > 0) {
    const target = current.history[current.historyIndex - 1];
    store.update((s) => ({ ...s, historyIndex: s.historyIndex - 1 }));
    await navigatePane(paneId, target, false);
  }
}

export async function goForward(paneId: 'left' | 'right') {
  const store = paneId === 'left' ? leftPane : rightPane;
  const current = get(store);
  if (current.historyIndex < current.history.length - 1) {
    const target = current.history[current.historyIndex + 1];
    store.update((s) => ({ ...s, historyIndex: s.historyIndex + 1 }));
    await navigatePane(paneId, target, false);
  }
}

export async function goUp(paneId: 'left' | 'right') {
  const store = paneId === 'left' ? leftPane : rightPane;
  const current = get(store);
  if (!current.currentPath) return;

  const parts = current.currentPath.split('/').filter(Boolean);
  if (parts.length > 0) {
    parts.pop();
    const parentPath = '/' + parts.join('/');
    await navigatePane(paneId, parentPath || '/');
  }
}

export function sortPaneItems(paneId: 'left' | 'right', sortBy: 'name' | 'size' | 'modified' | 'type') {
  const store = paneId === 'left' ? leftPane : rightPane;
  store.update((s) => {
    const isSame = s.sortBy === sortBy;
    const sortAsc = isSame ? !s.sortAsc : true;
    return { ...s, sortBy, sortAsc };
  });
}
