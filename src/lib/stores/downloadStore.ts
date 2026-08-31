import { writable, get } from 'svelte/store';
import { getHomeDirectory, transferItems } from '../invoke';

export const downloadDirectory = writable<string>('');
export const isSavingFile = writable<boolean>(false);
export const saveNotification = writable<{ text: string; path?: string; success: boolean } | null>(null);

export async function initDownloadStore() {
  try {
    const stored = localStorage.getItem('flashbrowse_download_folder');
    if (stored) {
      downloadDirectory.set(stored);
    } else {
      const home = await getHomeDirectory();
      const defaultDir = `${home}/Downloads`;
      downloadDirectory.set(defaultDir);
      localStorage.setItem('flashbrowse_download_folder', defaultDir);
    }
  } catch {
    downloadDirectory.set('~/Downloads');
  }
}

export function setDownloadDirectory(path: string) {
  const trimmed = path.trim();
  if (trimmed) {
    downloadDirectory.set(trimmed);
    localStorage.setItem('flashbrowse_download_folder', trimmed);
  }
}

export async function saveRemoteOrLocalItem(
  sourceIsSSH: boolean,
  sshHost: string,
  sourcePath: string,
  targetDirOverride?: string
): Promise<{ success: boolean; message: string; targetPath: string }> {
  let targetDir = targetDirOverride || get(downloadDirectory);
  if (!targetDir) {
    const home = await getHomeDirectory();
    targetDir = `${home}/Downloads`;
    downloadDirectory.set(targetDir);
  }

  const fileName = sourcePath.split('/').pop() || 'file';
  const targetPath = `${targetDir}/${fileName}`;

  isSavingFile.set(true);
  try {
    const msg = await transferItems(
      sourceIsSSH,
      sshHost,
      [sourcePath],
      false,
      '',
      targetDir
    );

    saveNotification.set({
      text: `Sparad till ${targetPath}`,
      path: targetPath,
      success: true,
    });
    setTimeout(() => saveNotification.set(null), 4000);
    return { success: true, message: msg, targetPath };
  } catch (err: any) {
    saveNotification.set({
      text: `Kunde inte spara: ${err}`,
      success: false,
    });
    setTimeout(() => saveNotification.set(null), 5000);
    return { success: false, message: String(err), targetPath };
  } finally {
    isSavingFile.set(false);
  }
}

export async function saveMultipleItems(
  sourceIsSSH: boolean,
  sshHost: string,
  sourcePaths: string[],
  targetDirOverride?: string
): Promise<{ success: boolean; message: string }> {
  if (sourcePaths.length === 0) return { success: false, message: 'Inga filer' };

  let targetDir = targetDirOverride || get(downloadDirectory);
  if (!targetDir) {
    const home = await getHomeDirectory();
    targetDir = `${home}/Downloads`;
    downloadDirectory.set(targetDir);
  }

  isSavingFile.set(true);
  try {
    const msg = await transferItems(
      sourceIsSSH,
      sshHost,
      sourcePaths,
      false,
      '',
      targetDir
    );

    saveNotification.set({
      text: `Sparade ${sourcePaths.length} filer till ${targetDir}`,
      success: true,
    });
    setTimeout(() => saveNotification.set(null), 4000);
    return { success: true, message: msg };
  } catch (err: any) {
    saveNotification.set({
      text: `Kunde inte spara: ${err}`,
      success: false,
    });
    setTimeout(() => saveNotification.set(null), 5000);
    return { success: false, message: String(err) };
  } finally {
    isSavingFile.set(false);
  }
}
