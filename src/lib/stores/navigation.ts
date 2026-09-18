import { writable, get } from 'svelte/store';
import { createPersistentStore } from './layoutStore';
import {
  listDirectory,
  getHomeDirectory,
  sshListDirectory,
  startTransfer,
  cancelTransfer,
  findCompanions,
  watchDirectory,
  parseConflictError,
  type ConflictStrategy,
  type TransferProgress,
  type Companion,
} from '../invoke';
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
/**
 * Which shell the app wears.
 *
 * `pro` is everything: two browsers, inspector presets, the terminal, the
 * bioinformatics tools. `clean` is one browser with the search field as the
 * main surface - the layout for someone who is only trying to find a file.
 * It is a layout, not a theme: both modes use the same palette.
 */
export type LayoutMode = 'pro' | 'clean';

export const layoutMode = createPersistentStore<LayoutMode>('flashbrowse_layout_mode', 'pro');

export function setLayoutMode(mode: LayoutMode) {
  layoutMode.set(mode);
  // Clean has one browser, so the keyboard must not act on a hidden one.
  if (mode === 'clean') activePaneId.set('left');
}

/**
 * One or two file browsers. Remembered between sessions.
 *
 * One by default: two panes on the same folder is the same listing twice, and
 * the width it costs is width the inspector could have had. The second pane
 * appears when there is something to put in it - a remote host - or when it is
 * asked for. The key is versioned because the old default was two, and a
 * remembered `true` would otherwise outlive the decision.
 */
export const isDualPane = createPersistentStore<boolean>('flashbrowse_dual_pane_v2', false);

/**
 * Open a host in a browser, giving it a pane of its own.
 *
 * Local and remote side by side is what the second pane is for, so connecting
 * opens one rather than replacing the folder the user was looking at. An
 * explicit pane choice is honoured as it is.
 */
export function connectToSshHost(host: string, path = '~', paneId?: 'left' | 'right') {
    let target = paneId ?? get(activePaneId);

    if (!paneId && !get(isDualPane)) {
        isDualPane.set(true);
        // Keep what is on screen where it is and put the host beside it.
        target = get(activePaneId) === 'left' ? 'right' : 'left';
    }

    const store = target === 'left' ? leftPane : rightPane;
    store.update((s) => ({ ...s, isSSH: true, sshHost: host }));
    activePaneId.set(target);
    navigatePane(target, path);
}
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
    const { emit } = await import('@tauri-apps/api/event');
    await emit('inspector-cast-item', item);
  } catch (err) {
    console.error('Failed to cast to secondary inspector:', err);
  }
}

let isWatchingStarted = false;
let refreshDebounceTimer: any = null;
const navRequestCounters = { left: 0, right: 0 };

/// Where each pane was when the app was last closed.
const SESSION_KEY = 'flashbrowse_session_v1';

interface SessionState {
  left?: string;
  right?: string;
}

function readSession(): SessionState {
  if (typeof localStorage === 'undefined') return {};
  try {
    return JSON.parse(localStorage.getItem(SESSION_KEY) || '{}');
  } catch {
    return {};
  }
}

/**
 * Remember the folder each pane is showing.
 *
 * Only local paths: restoring an SSH pane would open a connection during
 * startup, which hangs on a laptop that is off the VPN.
 */
let sessionSaveTimer: any = null;

/** Coalesce the writes: the pane stores also change on selection and filtering. */
function scheduleSessionSave() {
  clearTimeout(sessionSaveTimer);
  sessionSaveTimer = setTimeout(saveSession, 400);
}

function saveSession() {
  if (typeof localStorage === 'undefined') return;
  const left = get(leftPane);
  const right = get(rightPane);
  const state: SessionState = {
    left: left.isSSH ? undefined : left.currentPath || undefined,
    right: right.isSSH ? undefined : right.currentPath || undefined,
  };
  try {
    localStorage.setItem(SESSION_KEY, JSON.stringify(state));
  } catch (e) {
    console.warn('Failed to save session:', e);
  }
}

/** Go to the remembered folder, falling back to home if it is gone. */
async function restorePane(paneId: 'left' | 'right', saved: string | undefined, home: string) {
  if (saved && saved !== home) {
    await navigatePane(paneId, saved);
    const state = get(paneId === 'left' ? leftPane : rightPane);
    if (!state.errorMessage) return;
  }
  await navigatePane(paneId, home);
}

export async function initNavigation() {
  const home = await getHomeDirectory();
  const session = readSession();

  await Promise.all([
    restorePane('left', session.left, home),
    restorePane('right', session.right, home),
  ]);

  leftPane.subscribe(scheduleSessionSave);
  rightPane.subscribe(scheduleSessionSave);

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
/** Live progress of the running transfer, or null when nothing is running. */
export const transferProgress = writable<TransferProgress | null>(null);

let activeTransferId: string | null = null;
let progressListenerStarted = false;

function startProgressListener() {
  if (progressListenerStarted) return;
  progressListenerStarted = true;
  listen<TransferProgress>('transfer-progress', (event) => {
    // Ignore progress from an older transfer that is still winding down.
    if (event.payload.id !== activeTransferId) return;
    transferProgress.set(event.payload.done ? null : event.payload);
  }).catch(console.error);
}

/** Stop the transfer that is currently running, keeping partial data. */
export async function cancelActiveTransfer() {
  if (!activeTransferId) return;
  try {
    await cancelTransfer(activeTransferId);
  } catch (err) {
    console.error('Failed to cancel transfer:', err);
  }
}

/** Whether companion files are added automatically, asked about, or ignored. */
export type CompanionMode = 'ask' | 'always' | 'never';

export const companionMode = writable<CompanionMode>(
  (typeof localStorage !== 'undefined'
    ? (localStorage.getItem('flashbrowse_companion_mode') as CompanionMode)
    : null) || 'ask'
);

companionMode.subscribe((mode) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('flashbrowse_companion_mode', mode);
  }
});

const COMPANION_LABELS: Record<Companion['kind'], string> = {
  index: 'index',
  checksum: 'kontrollsumma',
  mate: 'parfil',
};

/**
 * Return the paths to transfer, including companions.
 *
 * Returns null if the user cancelled at the prompt.
 */
async function includeCompanions(
  paths: string[],
  isSSH: boolean,
  sshHost: string
): Promise<string[] | null> {
  const mode = get(companionMode);
  if (mode === 'never') return paths;

  let sets;
  try {
    sets = await findCompanions(paths, isSSH, sshHost);
  } catch (err) {
    // Finding companions is a convenience: never block the transfer on it.
    console.warn('Companion lookup failed:', err);
    return paths;
  }

  const extra = sets.flatMap((s) => s.companions);
  if (extra.length === 0) return paths;

  if (mode === 'ask') {
    const listed = extra
      .slice(0, 8)
      .map((c) => `  ${c.name}  (${COMPANION_LABELS[c.kind]}, ${c.formatted_size})`)
      .join('\n');
    const more = extra.length > 8 ? `\n  …och ${extra.length - 8} till` : '';
    const takeAlong = confirm(
      `${extra.length} följeslagarfiler hör ihop med markeringen:\n\n${listed}${more}\n\n` +
        'OK = ta med dem\nAvbryt = överför bara markerade filer'
    );
    if (!takeAlong) return paths;
  }

  const merged = new Set(paths);
  for (const c of extra) merged.add(c.path);
  return Array.from(merged);
}

export async function transferBetweenPanes(
  fromPaneId: 'left' | 'right',
  toPaneId: 'left' | 'right',
  explicitPaths?: string[]
) {
  const fromStore = fromPaneId === 'left' ? leftPane : rightPane;
  const toStore = toPaneId === 'left' ? leftPane : rightPane;

  const fromState = get(fromStore);
  const toState = get(toStore);

  let paths = explicitPaths && explicitPaths.length > 0
    ? explicitPaths
    : Array.from(fromState.selectedPaths);

  if (paths.length === 0) return;
  if (!toState.currentPath) return;

  // Offer to bring index files, checksums and pair mates along: a BAM that
  // arrives without its .bai is broken for everything downstream.
  const withCompanions = await includeCompanions(paths, fromState.isSSH, fromState.sshHost);
  if (withCompanions === null) return;
  paths = withCompanions;

  isTransferring.set(true);
  transferStatus.set(`Överför ${paths.length} objekt...`);

  startProgressListener();
  const transferId = `t${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  activeTransferId = transferId;

  const run = async (onConflict: ConflictStrategy) =>
    await startTransfer(
      transferId,
      fromState.isSSH,
      fromState.sshHost,
      paths,
      toState.isSSH,
      toState.sshHost,
      toState.currentPath,
      onConflict
    );

  try {
    let resultMsg: string;
    try {
      resultMsg = await run('fail');
    } catch (err: any) {
      // The backend refuses to overwrite; let the user decide instead of
      // silently destroying files that are already in the destination.
      const conflicts = parseConflictError(err);
      if (!conflicts) throw err;

      const shown = conflicts.slice(0, 8).join('\n');
      const more = conflicts.length > 8 ? `\n…och ${conflicts.length - 8} till` : '';
      const keepBoth = confirm(
        `${conflicts.length} objekt finns redan i målmappen:\n\n${shown}${more}\n\n` +
          'OK = behåll båda (kopiorna döps om)\nAvbryt = avbryt överföringen'
      );
      if (!keepBoth) {
        transferStatus.set('Överföring avbruten');
        setTimeout(() => transferStatus.set(null), 3000);
        return;
      }
      resultMsg = await run('rename');
    }

    transferStatus.set(resultMsg);
    setTimeout(() => transferStatus.set(null), 3000);
    await reloadPane(toPaneId);
  } catch (err: any) {
    transferStatus.set(`Fel vid överföring: ${err}`);
    setTimeout(() => transferStatus.set(null), 6000);
  } finally {
    isTransferring.set(false);
    transferProgress.set(null);
    activeTransferId = null;
  }
}
