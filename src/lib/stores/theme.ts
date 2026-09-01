import { writable } from 'svelte/store';
import type { ThemeName } from '../types';

function getInitialTheme(): ThemeName {
  if (typeof localStorage !== 'undefined') {
    const saved = localStorage.getItem('flashbrowse_theme') as ThemeName;
    if (saved) return saved;
  }
  return 'swift-dark';
}

export const currentTheme = writable<ThemeName>(getInitialTheme());
export const isKidsMode = writable<boolean>(false);
export const kidsModePin = writable<string>('1234');
export const isPinModalOpen = writable<boolean>(false);

export function setTheme(theme: ThemeName) {
  currentTheme.set(theme);
  isKidsMode.set(theme === 'kids-mode');
  if (typeof localStorage !== 'undefined') {
    try {
      localStorage.setItem('flashbrowse_theme', theme);
    } catch {}
  }
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.theme = theme;
  }
}
