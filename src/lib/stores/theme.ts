import { writable } from 'svelte/store';
import type { ThemeName } from '../types';

export const currentTheme = writable<ThemeName>('pro-dark');
export const isKidsMode = writable<boolean>(false);
export const kidsModePin = writable<string>('1234');
export const isPinModalOpen = writable<boolean>(false);

export function setTheme(theme: ThemeName) {
  currentTheme.set(theme);
  isKidsMode.set(theme === 'kids-mode');
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.theme = theme;
  }
}
