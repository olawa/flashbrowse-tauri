import { get, writable, derived } from 'svelte/store';
import type { FileItem, GenomeRefInfo, RsnapServerInfo, TrackGenomeDetection } from '../types';
import {
  getRsnapServerStatus,
  startRsnapServer,
  stopRsnapServer,
  checkIgvStatus,
  getConfiguredGenomes,
  saveConfiguredGenome,
  detectTrackGenomes,
} from '../invoke';

export interface StagedGenomicsTrack {
  path: string;
  name: string;
  kind: 'bam' | 'vcf' | 'bed' | 'other';
  formatted_size?: string;
  detected_build?: string;
  detected_label?: string;
}

export const stagedTracks = writable<StagedGenomicsTrack[]>([]);
export const selectedLocus = writable<string>('chr7:55152000-55153000');
export const selectedGenome = writable<string>('hg38');
export const isRsnapServerRunning = writable<boolean>(false);
export const rsnapServerPid = writable<number | null>(null);
export const isIgvConnected = writable<boolean>(false);
export const isGenomicsHubOpen = writable<boolean>(false);
export const configuredGenomes = writable<GenomeRefInfo[]>([]);

export async function loadGenomes() {
  try {
    const list = await getConfiguredGenomes();
    configuredGenomes.set(list);
  } catch (err) {
    console.warn('Failed to load configured genomes:', err);
  }
}

export const genomeMismatchInfo = derived(stagedTracks, ($tracks) => {
  const detected = new Set<string>();
  for (const t of $tracks) {
    if (t.detected_build && t.detected_build !== 'unknown' && t.detected_build !== 'custom') {
      detected.add(t.detected_build);
    }
  }
  const builds = Array.from(detected);
  return {
    hasMismatch: builds.length > 1,
    builds,
  };
});

export async function addTracksToHub(items: (FileItem | StagedGenomicsTrack | string)[]) {
  const newTracks: StagedGenomicsTrack[] = [];
  const existingPaths = new Set(get(stagedTracks).map((t) => t.path));

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

    newTracks.push({ path, name, kind, formatted_size });
    existingPaths.add(path);
  }

  if (newTracks.length === 0) return;

  stagedTracks.update((current) => [...current, ...newTracks]);

  // Detect genomes for newly added tracks
  try {
    const detections = await detectTrackGenomes(newTracks.map((t) => t.path));
    const detMap = new Map(detections.map((d) => [d.path, d]));

    stagedTracks.update((current) =>
      current.map((t) => {
        const d = detMap.get(t.path);
        if (d) {
          return { ...t, detected_build: d.detected_build, detected_label: d.detected_label };
        }
        return t;
      })
    );

    // If first track has a clear genome detected (e.g. hg19 or hg38), auto-select it if not manually changed
    if (detections.length > 0 && detections[0].detected_build !== 'unknown') {
      selectedGenome.set(detections[0].detected_build);
    }
  } catch (err) {
    console.warn('Track genome detection error:', err);
  }
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


