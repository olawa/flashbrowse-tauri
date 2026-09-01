import { writable, get } from 'svelte/store';
import { listDirectory, getHomeDirectory, sshListDirectory, transferItems, watchDirectory } from '../invoke';
import { listen } from '@tauri-apps/api/event';
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
export const isInspectorLocked = writable<boolean>(false);

export function toggleInspectorLock() {
  isInspectorLocked.update((v) => !v);
}

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

let isWatchingStarted = false;
let refreshDebounceTimer: any = null;
const navRequestCounters = { left: 0, right: 0 };

export async function initNavigation() {
  const home = await getHomeDirectory();
  await navigatePane('left', home);
  await navigatePane('right', home);

  if (!isWatchingStarted) {
    isWatchingStarted = true;
    listen<string>('directory-changed', (event) => {
      const changedPath = event.payload;
      clearTimeout(refreshDebounceTimer);
      refreshDebounceTimer = setTimeout(() => {
        const left = get(leftPane);
        const right = get(rightPane);
        if (!left.isSSH && left.currentPath === changedPath) {
          quietRefreshPane('left');
        }
        if (!right.isSSH && right.currentPath === changedPath) {
          quietRefreshPane('right');
        }
      }, 500);
    }).catch(console.error);
  }
}

export async function reloadPane(paneId: 'left' | 'right') {
  const store = paneId === 'left' ? leftPane : rightPane;
  const cur = get(store);
  if (cur.currentPath) {
    await navigatePane(paneId, cur.currentPath, false);
  }
}

export async function quietRefreshPane(paneId: 'left' | 'right') {
  const store = paneId === 'left' ? leftPane : rightPane;
  const current = get(store);
  if (!current.currentPath || current.isSSH) return;

  const showHidden = get(showHiddenFiles);
  const reqId = ++navRequestCounters[paneId];
  try {
    const newItems = await listDirectory(current.currentPath, showHidden);
    if (reqId !== navRequestCounters[paneId]) return;

    // Content diff check: only update store if items actually changed
    const isSame =
      current.items.length === newItems.length &&
      current.items.every(
        (it, i) =>
          it.path === newItems[i].path &&
          it.size_bytes === newItems[i].size_bytes &&
          it.modified_timestamp === newItems[i].modified_timestamp
      );

    if (isSame) return;

    store.update((s) => ({
      ...s,
      items: newItems,
    }));
  } catch (e) {
    console.warn('Quiet refresh error:', e);
  }
}

export async function navigatePane(
  paneId: 'left' | 'right',
  path: string,
  addToHistory = true
) {
  const reqId = ++navRequestCounters[paneId];
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
    let resolvedPath = path;
    if (current.isSSH) {
      const res = await sshListDirectory(current.sshHost, path);
      if (reqId !== navRequestCounters[paneId]) return;
      items = res.items;
      resolvedPath = res.current_path;
    } else {
      items = await listDirectory(path, showHidden);
      if (reqId !== navRequestCounters[paneId]) return;
      // Start live file watcher on current folder
      watchDirectory(path).catch(() => {});
    }

    store.update((s) => {
      if (reqId !== navRequestCounters[paneId]) return s;

      let newHistory = s.history;
      let newIndex = s.historyIndex;

      if (addToHistory && s.currentPath !== resolvedPath) {
        newHistory = s.history.slice(0, s.historyIndex + 1);
        newHistory.push(resolvedPath);
        newIndex = newHistory.length - 1;
      }

      const newSelected = new Set<string>();
      if (items.length > 0) {
        newSelected.add(items[0].path);
      }

      return {
        ...s,
        currentPath: resolvedPath,
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
    if (reqId !== navRequestCounters[paneId]) return;
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

export const isTransferring = writable<boolean>(false);
export const transferStatus = writable<string | null>(null);

export async function transferBetweenPanes(
  fromPaneId: 'left' | 'right',
  toPaneId: 'left' | 'right',
  explicitPaths?: string[]
) {
  const fromStore = fromPaneId === 'left' ? leftPane : rightPane;
  const toStore = toPaneId === 'left' ? leftPane : rightPane;

  const fromState = get(fromStore);
  const toState = get(toStore);

  const paths = explicitPaths && explicitPaths.length > 0
    ? explicitPaths
    : Array.from(fromState.selectedPaths);

  if (paths.length === 0) return;
  if (!toState.currentPath) return;

  isTransferring.set(true);
  transferStatus.set(`Överför ${paths.length} objekt...`);

  try {
    const resultMsg = await transferItems(
      fromState.isSSH,
      fromState.sshHost,
      paths,
      toState.isSSH,
      toState.sshHost,
      toState.currentPath
    );
    transferStatus.set(resultMsg);
    setTimeout(() => transferStatus.set(null), 3000);
    await reloadPane(toPaneId);
  } catch (err: any) {
    transferStatus.set(`Fel vid överföring: ${err}`);
    setTimeout(() => transferStatus.set(null), 6000);
  } finally {
    isTransferring.set(false);
  }
}
