import { invoke } from '@tauri-apps/api/core';
import type {
  ArchiveSummary,
  BamHeaderData,
  DirectoryIndexGroup,
  DirectoryNotes,
  DirectorySummary,
  DiskInfo,
  FileItem,
  PreviewContent,
  SamViewResult,
  TabCompletionResult,
  TerminalOutput,
} from './types';

export async function getHomeDirectory(): Promise<string> {
  return await invoke<string>('get_home_directory');
}

export async function listDirectory(path: string, showHidden = false): Promise<FileItem[]> {
  return await invoke<FileItem[]>('list_directory', { path, showHidden });
}

export async function getDiskInfo(path: string): Promise<DiskInfo> {
  return await invoke<DiskInfo>('get_disk_info', { path });
}

export async function calculateDirSize(path: string): Promise<DirectorySummary> {
  return await invoke<DirectorySummary>('calculate_dir_size', { path });
}

export async function trashItems(paths: string[]): Promise<void> {
  await invoke('trash_items', { paths });
}

export async function copyItems(paths: string[], destinationDir: string): Promise<void> {
  await invoke('copy_items', { paths, destinationDir });
}

export async function moveItems(paths: string[], destinationDir: string): Promise<void> {
  await invoke('move_items', { paths, destinationDir });
}

export async function createDirectory(parent: string, name: string): Promise<string> {
  return await invoke<string>('create_directory', { parent, name });
}

export async function createFile(parent: string, name: string): Promise<string> {
  return await invoke<string>('create_file', { parent, name });
}

export async function renameItem(path: string, newName: string): Promise<string> {
  return await invoke<string>('rename_item', { path, newName });
}

export async function openInDefault(path: string): Promise<void> {
  await invoke('open_in_default', { path });
}

export async function revealInOs(path: string): Promise<void> {
  await invoke('reveal_in_os', { path });
}

export async function getPreview(path: string, maxBytes?: number): Promise<PreviewContent> {
  return await invoke<PreviewContent>('get_preview', { path, maxBytes });
}

export async function runCommand(cmd: string, cwd: string): Promise<TerminalOutput> {
  return await invoke<TerminalOutput>('run_command', { cmd, cwd });
}

export async function tabComplete(input: string, cwd: string): Promise<TabCompletionResult> {
  return await invoke<TabCompletionResult>('tab_complete', { input, cwd });
}

export interface SshDirectoryResult {
  current_path: string;
  items: FileItem[];
}

export async function sshListDirectory(host: string, path: string): Promise<SshDirectoryResult> {
  return await invoke<SshDirectoryResult>('ssh_list_directory', { host, path });
}

export async function sshGetPreview(host: string, path: string): Promise<PreviewContent> {
  return await invoke<PreviewContent>('ssh_get_preview', { host, path });
}

export async function sshRunCommand(host: string, cmd: string, cwd: string): Promise<TerminalOutput> {
  return await invoke<TerminalOutput>('ssh_run_command', { host, cmd, cwd });
}

export async function quickLook(path: string): Promise<void> {
  await invoke('quick_look', { path });
}

export async function toggleDetachedInspector(path?: string): Promise<void> {
  await invoke('toggle_detached_inspector', { path });
}

export async function getBamHeader(path: string): Promise<BamHeaderData> {
  return await invoke<BamHeaderData>('get_bam_header', { path });
}

export async function generateRsnapSnapshot(
  bamPath: string,
  region: string,
  refPath?: string,
): Promise<string> {
  return await invoke<string>('generate_rsnap_snapshot', {
    bamPath,
    region,
    refPath,
  });
}

export async function launchRsnap(
  paths: string[],
  region?: string,
  refPath?: string,
): Promise<void> {
  await invoke('launch_rsnap', { paths, region, refPath });
}

export async function runRsQc(bamPath: string): Promise<string> {
  return await invoke<string>('run_rs_qc', { bamPath });
}

export async function listArchiveContents(path: string): Promise<ArchiveSummary> {
  return await invoke<ArchiveSummary>('list_archive_contents', { path });
}

export async function scanDirectoryIndex(
  rootPath: string,
  extensions: string[],
  maxDepth = 8,
): Promise<DirectoryIndexGroup[]> {
  return await invoke<DirectoryIndexGroup[]>('scan_directory_index', {
    rootPath,
    extensions,
    maxDepth,
  });
}

export async function getBamAlignments(
  path: string,
  region?: string,
  limit = 50,
  offset = 0,
): Promise<SamViewResult> {
  return await invoke<SamViewResult>('get_bam_alignments', {
    path,
    region,
    limit,
    offset,
  });
}

export async function getDirectoryNotes(dirPath: string): Promise<DirectoryNotes> {
  return await invoke<DirectoryNotes>('get_directory_notes', { dirPath });
}

export async function saveDirectoryNotes(
  dirPath: string,
  content: string,
  filename?: string,
): Promise<DirectoryNotes> {
  return await invoke<DirectoryNotes>('save_directory_notes', { dirPath, content, filename });
}
