import { get, writable } from 'svelte/store';
import type { FileItem, RsnapServerInfo } from '../types';
import { getRsnapServerStatus, startRsnapServer, stopRsnapServer, checkIgvStatus } from '../invoke';

export interface StagedGenomicsTrack {
  path: string;
  name: string;
  kind: 'bam' | 'vcf' | 'bed' | 'other';
  formatted_size?: string;
}

export const stagedTracks = writable<StagedGenomicsTrack[]>([]);
export const selectedLocus = writable<string>('chr7:55152000-55153000');
export const selectedGenome = writable<string>('hg38');
export const isRsnapServerRunning = writable<boolean>(false);
export const rsnapServerPid = writable<number | null>(null);
export const isIgvConnected = writable<boolean>(false);
export const isGenomicsHubOpen = writable<boolean>(false);

export function addTracksToHub(items: (FileItem | StagedGenomicsTrack | string)[]) {
  stagedTracks.update((current) => {
    const existingPaths = new Set(current.map((t) => t.path));
    const next = [...current];

    for (const item of items) {
      const path = typeof item === 'string' ? item : item.path;
      if (existingPaths.has(path)) continue;

      const name = typeof item === 'string' ? path.split('/').pop() || path : item.name;
      const lower = name.toLowerCase();
      let kind: 'bam' | 'vcf' | 'bed' | 'other' = 'other';
      if (lower.endsWith('.bam') || lower.endsWith('.cram') || lower.endsWith('.sam')) kind = 'bam';
      else if (lower.endsWith('.vcf') || lower.endsWith('.vcf.gz') || lower.endsWith('.bcf')) kind = 'vcf';
      else if (lower.endsWith('.bed') || lower.endsWith('.bw') || lower.endsWith('.bigwig') || lower.endsWith('.bedgraph')) kind = 'bed';

      const formatted_size = typeof item === 'object' && 'formatted_size' in item ? item.formatted_size : undefined;

      next.push({ path, name, kind, formatted_size });
      existingPaths.add(path);
    }
    return next;
  });
}

export function removeTrackFromHub(path: string) {
  stagedTracks.update((current) => current.filter((t) => t.path !== path));
}

export function clearTracksInHub() {
  stagedTracks.set([]);
}

export async function pollGenomicsStatuses() {
  try {
    const info = await getRsnapServerStatus();
    if (get(isRsnapServerRunning) !== info.is_running) isRsnapServerRunning.set(info.is_running);
    if (get(rsnapServerPid) !== (info.pid || null)) rsnapServerPid.set(info.pid || null);
  } catch {
    if (get(isRsnapServerRunning) !== false) isRsnapServerRunning.set(false);
    if (get(rsnapServerPid) !== null) rsnapServerPid.set(null);
  }

  try {
    const igvOk = await checkIgvStatus(60151);
    if (get(isIgvConnected) !== igvOk) isIgvConnected.set(igvOk);
  } catch {
    if (get(isIgvConnected) !== false) isIgvConnected.set(false);
  }
}

