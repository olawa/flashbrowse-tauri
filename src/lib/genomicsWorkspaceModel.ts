import type { FileItem, GenomicsFileInfo } from './types';

export type GenomicsKind = GenomicsFileInfo['kind'];

export function kindFromName(name: string): GenomicsKind {
  const n = name.toLowerCase();
  if (/\.(bam|cram|sam)$/.test(n)) return 'alignment';
  if (/\.(vcf|vcf\.gz|bcf)$/.test(n)) return 'variant';
  if (/\.(fastq|fq)(\.gz)?$/.test(n)) return 'reads';
  if (/\.(fa|fasta|fna)(\.gz)?$/.test(n)) return 'reference';
  if (/\.bed(\.gz)?$/.test(n)) return 'regions';
  return 'other';
}

export function guessedSample(name: string): string | null {
  const stem = name
    .replace(/\.(fastq|fq|bam|cram|sam|vcf|bcf)(\.gz)?$/i, '')
    .replace(/\.(sorted|dedup|recal|variants)$/i, '')
    .replace(/_001$/i, '')
    .replace(/[_\-.]R?[12]$/i, '')
    .replace(/_L\d{3}$/i, '')
    .replace(/_S\d+$/i, '')
    .replace(/^[_\-.]+|[_\-.]+$/g, '');
  return stem || null;
}

export function readMate(name: string): 1 | 2 | null {
  const stem = name.replace(/\.(fastq|fq)(\.gz)?$/i, '');
  const match = stem.match(/(?:^|[_\-.])R?([12])(?:_001)?$/i);
  return match ? Number(match[1]) as 1 | 2 : null;
}

export interface SampleRow {
  name: string;
  evidence: 'header' | 'filename' | 'unknown';
  r1: FileItem[];
  r2: FileItem[];
  otherReads: FileItem[];
  alignments: FileItem[];
  variants: FileItem[];
  issues: string[];
}

/** Multi-sample VCFs remain separate: assigning them to one specimen would be misleading. */
export function groupSamples(items: FileItem[], info: Record<string, GenomicsFileInfo>): { rows: SampleRow[]; shared: FileItem[] } {
  const rows = new Map<string, SampleRow>();
  const shared: FileItem[] = [];
  for (const item of items) {
    const detail = info[item.path];
    const kind = detail?.kind ?? kindFromName(item.name);
    if (!['reads', 'alignment', 'variant'].includes(kind)) continue;
    if (detail && (detail.sample_count > 1 || kind === 'variant' && detail.header_status === 'read' && detail.sample_count === 0)) {
      shared.push(item);
      continue;
    }
    const sample = detail?.samples[0] ?? guessedSample(item.name) ?? 'Okänt prov';
    const evidence = detail?.sample_source ?? (sample === 'Okänt prov' ? 'unknown' : 'filename');
    const row = rows.get(sample) ?? {
      name: sample, evidence, r1: [], r2: [], otherReads: [], alignments: [], variants: [], issues: [],
    };
    if (evidence === 'header') row.evidence = 'header';
    if (kind === 'reads') {
      const mate = readMate(item.name);
      if (mate === 1) row.r1.push(item);
      else if (mate === 2) row.r2.push(item);
      else row.otherReads.push(item);
    } else if (kind === 'alignment') row.alignments.push(item);
    else row.variants.push(item);
    rows.set(sample, row);
  }
  for (const row of rows.values()) {
    if (row.r1.length && !row.r2.length) row.issues.push('R2 saknas');
    if (row.r2.length && !row.r1.length) row.issues.push('R1 saknas');
    if (row.alignments.length > 1) row.issues.push('Flera alignments');
  }
  return { rows: [...rows.values()].sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true })), shared };
}

export interface CompatibilityResult {
  status: 'conflict' | 'warning' | 'no_detected_conflict' | 'inconclusive';
  headline: string;
  details: string[];
}

/** Compare only observed metadata; a partial/unknown dictionary never proves compatibility. */
export function compareReferenceProfiles(files: GenomicsFileInfo[]): CompatibilityResult {
  if (files.length < 2) return { status: 'inconclusive', headline: 'Välj minst två filer', details: [] };
  const known = files.filter((f) => f.header_status === 'read' && f.contigs.length > 0);
  if (known.length < 2) return { status: 'inconclusive', headline: 'För få läsbara referensprofiler', details: ['Läs metadata för valda filer och kontrollera eventuella fel i filhälsan.'] };
  const conflicts: string[] = [];
  const warnings: string[] = [];
  let matchedLengths = 0;
  for (let i = 0; i < known.length; i++) {
    for (let j = i + 1; j < known.length; j++) {
      const a = known[i], b = known[j];
      const label = `${a.path.split('/').pop()} ↔ ${b.path.split('/').pop()}`;
      if (a.build && b.build && a.build !== b.build) conflicts.push(`${label}: olika referensbyggen (${a.build}, ${b.build}).`);
      const aContigs = new Map(a.contigs.map((c) => [c.name, c.length]));
      const bContigs = new Map(b.contigs.map((c) => [c.name, c.length]));
      const shared = [...aContigs.keys()].filter((name) => bContigs.has(name));
      if (!shared.length) {
        const normalized = new Set([...aContigs.keys()].map((name) => name.replace(/^chr/i, '')));
        const prefixOverlap = [...bContigs.keys()].some((name) => normalized.has(name.replace(/^chr/i, '')));
        warnings.push(`${label}: ${prefixOverlap ? 'contignamn skiljer sig (t.ex. chr1/1)' : 'inga gemensamma contignamn i läst metadata'}.`);
        continue;
      }
      for (const name of shared) {
        const aLength = aContigs.get(name), bLength = bContigs.get(name);
        if (aLength != null && bLength != null) {
          if (aLength !== bLength) conflicts.push(`${label}: ${name} har olika längd (${aLength} respektive ${bLength} bp).`);
          else matchedLengths++;
        }
      }
      if (a.contigs_truncated || b.contigs_truncated) warnings.push(`${label}: bara de första 1 000 contigerna lästes.`);
    }
  }
  if (known.length < files.length) warnings.push(`${files.length - known.length} valda filer saknar läsbar referensprofil.`);
  if (conflicts.length) return { status: 'conflict', headline: 'Konflikt i referensmetadata', details: [...conflicts, ...warnings] };
  if (warnings.length) return { status: 'warning', headline: 'Möjlig skillnad eller ofullständiga data', details: warnings };
  if (matchedLengths) return { status: 'no_detected_conflict', headline: 'Ingen konflikt hittad i jämförda contiger', details: [`${matchedLengths} gemensamma contiglängder stämmer. Det bevisar inte att alla koordinater är utbytbara.`] };
  return { status: 'inconclusive', headline: 'Gemensamma namn, men inga jämförbara längder', details: ['Referensbygget kan inte verifieras från dessa filer.'] };
}
