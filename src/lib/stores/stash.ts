import { writable } from 'svelte/store';
import type { FileItem } from '../types';

export const stashItems = writable<FileItem[]>([]);
export const isStashOpen = writable<boolean>(false);

export function addToStash(item: FileItem) {
  stashItems.update((items) => {
    if (items.some((i) => i.path === item.path)) {
      return items;
    }
    return [...items, item];
  });
  isStashOpen.set(true);
}

export function addMultipleToStash(newItems: FileItem[]) {
  stashItems.update((items) => {
    const existing = new Set(items.map((i) => i.path));
    const toAdd = newItems.filter((i) => !existing.has(i.path));
    return [...items, ...toAdd];
  });
  isStashOpen.set(true);
}

export function removeFromStash(path: string) {
  stashItems.update((items) => items.filter((i) => i.path !== path));
}

export function clearStash() {
  stashItems.set([]);
}

export function toggleStash() {
  isStashOpen.update((v) => !v);
}
