import { invoke } from '@tauri-apps/api/core';
import type {
  ArchiveSummary,
  BamHeaderData,
  DirectoryIndexGroup,
  DirectoryNotes,
  DirectorySummary,
  DiskInfo,
  FileItem,
  GenomeRefInfo,
  IgvResponse,
  PreviewContent,
  RsnapServerInfo,
  SamViewResult,
  SearchMatch,
  TabCompletionResult,
  TerminalOutput,
  TrackGenomeDetection,
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

export async function transferItems(
  sourceIsSsh: boolean,
  sourceSshHost: string,
  sourcePaths: string[],
  destIsSsh: boolean,
  destSshHost: string,
  destDir: string,
): Promise<string> {
  return await invoke<string>('transfer_items', {
    sourceIsSsh,
    sourceSshHost,
    sourcePaths,
    destIsSsh,
    destSshHost,
    destDir,
  });
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

export async function openFileWith(path: string, appName?: string): Promise<void> {
  await invoke('open_file_with', { path, appName });
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

export async function sshOpenFileLocally(host: string, remotePath: string, appName?: string): Promise<string> {
  return await invoke<string>('ssh_open_file_locally', { host, remotePath, appName });
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
  genomeId?: string,
  refPath?: string,
  gtfPath?: string,
): Promise<string> {
  return await invoke<string>('generate_rsnap_snapshot', {
    bamPath,
    region,
    genomeId,
    refPath,
    gtfPath,
  });
}

export async function launchRsnap(
  paths: string[],
  region?: string,
  genomeId?: string,
  refPath?: string,
  gtfPath?: string,
  connectToServer?: boolean,
  serverAddress?: string,
): Promise<void> {
  await invoke('launch_rsnap', {
    paths,
    region,
    genomeId,
    refPath,
    gtfPath,
    connectToServer,
    serverAddress,
  });
}

export async function startRsnapServer(
  bamDir?: string,
  genomeId?: string,
  port?: number,
): Promise<RsnapServerInfo> {
  return await invoke<RsnapServerInfo>('start_rsnap_server', { bamDir, genomeId, port });
}

export async function stopRsnapServer(): Promise<boolean> {
  return await invoke<boolean>('stop_rsnap_server');
}

export async function getRsnapServerStatus(): Promise<RsnapServerInfo> {
  return await invoke<RsnapServerInfo>('get_rsnap_server_status');
}

export async function getConfiguredGenomes(): Promise<GenomeRefInfo[]> {
  return await invoke<GenomeRefInfo[]>('get_configured_genomes');
}

export async function saveConfiguredGenome(genome: GenomeRefInfo): Promise<GenomeRefInfo[]> {
  return await invoke<GenomeRefInfo[]>('save_configured_genome', { genome });
}

export async function detectTrackGenomes(paths: string[]): Promise<TrackGenomeDetection[]> {
  return await invoke<TrackGenomeDetection[]>('detect_track_genomes', { paths });
}

export async function sendToIgv(
  paths: string[],
  locus?: string,
  genome?: string,
  port?: number,
): Promise<IgvResponse> {
  return await invoke<IgvResponse>('send_to_igv', { paths, locus, genome, port });
}

export async function checkIgvStatus(port?: number): Promise<boolean> {
  return await invoke<boolean>('check_igv_status', { port });
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

export async function createZipArchive(
  sourcePaths: string[],
  outputZipPath?: string,
): Promise<string> {
  return await invoke<string>('create_zip_archive', { sourcePaths, outputZipPath });
}

export async function watchDirectory(path: string): Promise<void> {
  await invoke('watch_directory', { path });
}

export async function deepSearch(
  rootPath: string,
  query: string,
  maxResults = 80,
): Promise<SearchMatch[]> {
  return await invoke<SearchMatch[]>('deep_search', { rootPath, query, maxResults });
}

export async function getSubdirsTree(
  path: string,
  maxDepth = 3,
  maxPerLevel = 8,
): Promise<import('./types').SubdirNode[]> {
  return await invoke('get_subdirs_tree', { path, maxDepth, maxPerLevel });
}
