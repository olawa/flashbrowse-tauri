import { writable, get } from 'svelte/store';
import { openFileWith, sshOpenFileLocally } from '../invoke';
import { saveNotification } from './downloadStore';

export interface EditorOption {
  id: string;
  name: string;
  appName: string;
  badge: string;
}

export const SUPPORTED_EDITORS: EditorOption[] = [
  { id: 'vscode', name: 'Visual Studio Code', appName: 'Visual Studio Code', badge: 'VSCode' },
  { id: 'cursor', name: 'Cursor', appName: 'Cursor', badge: 'Cursor' },
  { id: 'zed', name: 'Zed', appName: 'Zed', badge: 'Zed' },
  { id: 'pycharm', name: 'PyCharm', appName: 'PyCharm', badge: 'PyCharm' },
  { id: 'sublime', name: 'Sublime Text', appName: 'Sublime Text', badge: 'Sublime' },
  { id: 'textedit', name: 'TextEdit', appName: 'TextEdit', badge: 'TextEdit' },
];

function getInitialEditor(): string {
  if (typeof window !== 'undefined') {
    const saved = localStorage.getItem('flashbrowse_favorite_editor');
    if (saved) return saved;
  }
  return 'Visual Studio Code';
}

export const favoriteEditor = writable<string>(getInitialEditor());

export function setFavoriteEditor(appName: string) {
  favoriteEditor.set(appName);
  try {
    localStorage.setItem('flashbrowse_favorite_editor', appName);
  } catch {}
}

export async function openInFavoriteEditor(path: string, isSSH = false, host = ''): Promise<void> {
  if (!path) return;
  const editor = get(favoriteEditor);
  const fileName = path.split('/').pop() || path;

  if (isSSH && host) {
    saveNotification.set({
      text: `⬇️ Öppnar ${fileName} i ${editor} över SSH...`,
      success: true,
    });
    try {
      const localPath = await sshOpenFileLocally(host, path, editor);
      saveNotification.set({
        text: `🚀 Öppnade ${fileName} i ${editor}`,
        path: localPath,
        success: true,
      });
      setTimeout(() => saveNotification.set(null), 4000);
    } catch (err: any) {
      saveNotification.set({
        text: `❌ Kunde inte öppna i ${editor}: ${err?.message || err}`,
        success: false,
      });
      setTimeout(() => saveNotification.set(null), 5000);
    }
  } else {
    try {
      await openFileWith(path, editor);
      saveNotification.set({
        text: `🚀 Öppnade i ${editor}`,
        success: true,
      });
      setTimeout(() => saveNotification.set(null), 2500);
    } catch (err: any) {
      saveNotification.set({
        text: `❌ Kunde inte öppna i ${editor}: ${err?.message || err}`,
        success: false,
      });
      setTimeout(() => saveNotification.set(null), 4000);
    }
  }
}
