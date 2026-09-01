import { writable } from 'svelte/store';

function createPersistentStore<T>(key: string, defaultValue: T) {
  let initial = defaultValue;
  if (typeof localStorage !== 'undefined') {
    try {
      const stored = localStorage.getItem(key);
      if (stored !== null) {
        initial = JSON.parse(stored);
      }
    } catch (e) {
      console.warn(`Failed to read ${key} from localStorage:`, e);
    }
  }

  const store = writable<T>(initial);

  store.subscribe((val) => {
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem(key, JSON.stringify(val));
      } catch (e) {
        console.warn(`Failed to save ${key} to localStorage:`, e);
      }
    }
  });

  return store;
}

export const sidebarWidth = createPersistentStore<number>('flashbrowse_sidebar_width', 208);
export const inspectorWidth = createPersistentStore<number>('flashbrowse_inspector_width', 540);
export const dualInspectorWidth = createPersistentStore<number>('flashbrowse_dual_inspector_width', 360);
export const terminalHeight = createPersistentStore<number>('flashbrowse_terminal_height', 240);
export const terminalWidth = createPersistentStore<number>('flashbrowse_terminal_width', 400);

export function resetLayoutWidths() {
  sidebarWidth.set(208);
  inspectorWidth.set(540);
  dualInspectorWidth.set(360);
  terminalHeight.set(240);
  terminalWidth.set(400);
}
