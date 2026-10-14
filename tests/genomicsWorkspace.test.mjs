import test from 'node:test';
import assert from 'node:assert/strict';
import { compareReferenceProfiles, groupSamples, guessedSample, readMate } from '../src/lib/genomicsWorkspaceModel.ts';

const item = (name, directory = '/project') => ({ name, path: `${directory}/${name}`, is_dir: false });
const info = (path, overrides = {}) => ({
  path, kind: 'alignment', samples: [], sample_source: 'unknown', sample_count: 0,
  contigs: [], contigs_truncated: false, build: null, header_status: 'read', header_message: null,
  index_status: 'not_applicable', index_path: null, quickcheck_status: 'not_run', quickcheck_message: null,
  ...overrides,
});

test('pairs FASTQ lanes across folders and identifies a missing mate', () => {
  const files = [
    item('Patient-A_S3_L001_R1_001.fastq.gz', '/project/fastq/lane1'),
    item('Patient-A_S3_L001_R2_001.fastq.gz', '/project/fastq/lane1'),
    item('Patient-A_S3_L002_R1_001.fastq.gz', '/project/fastq/lane2'),
    item('Case-B_R1.fastq.gz', '/project/fastq'),
  ];
  assert.equal(guessedSample(files[0].name), 'Patient-A');
  assert.equal(readMate(files[0].name), 1);
  assert.equal(readMate(files[1].name), 2);
  const grouped = groupSamples(files, {});
  const patient = grouped.rows.find((row) => row.name === 'Patient-A');
  assert.equal(patient.r1.length, 2);
  assert.equal(patient.r2.length, 1);
  assert.deepEqual(patient.issues, []);
  assert.deepEqual(grouped.rows.find((row) => row.name === 'Case-B').issues, ['R2 saknas']);
});

test('header sample names take precedence while multi-sample VCF stays separate', () => {
  const bam = item('ambiguous.sorted.bam');
  const vcf = item('cohort.vcf.gz');
  const fasta = item('GRCh38.fa');
  const result = groupSamples([bam, vcf, fasta], {
    [bam.path]: info(bam.path, { samples: ['Patient-A'], sample_source: 'header', sample_count: 1 }),
    [vcf.path]: info(vcf.path, { kind: 'variant', samples: ['Patient-A', 'Case-B'], sample_source: 'header', sample_count: 2 }),
  });
  assert.equal(result.rows.length, 1);
  assert.equal(result.rows[0].name, 'Patient-A');
  assert.equal(result.rows[0].evidence, 'header');
  assert.deepEqual(result.rows[0].alignments, [bam]);
  assert.deepEqual(result.shared, [vcf]);
});

test('a VCF without sample columns is not assigned from its filename', () => {
  const vcf = item('Patient-A.vcf');
  const result = groupSamples([vcf], {
    [vcf.path]: info(vcf.path, { kind: 'variant', header_status: 'read', sample_count: 0 }),
  });
  assert.deepEqual(result.rows, []);
  assert.deepEqual(result.shared, [vcf]);
});

test('different chr1 lengths are a verified conflict', () => {
  const first = info('/project/a.bam', { build: 'GRCh38/hg38', contigs: [{ name: 'chr1', length: 248956422 }] });
  const second = info('/project/b.vcf', { kind: 'variant', build: 'GRCh37/hg19', contigs: [{ name: 'chr1', length: 249250621 }] });
  const result = compareReferenceProfiles([first, second]);
  assert.equal(result.status, 'conflict');
  assert.match(result.details.join(' '), /olika längd/);
});

test('contig prefix difference and missing lengths do not become a green verdict', () => {
  const first = info('/a.bam', { contigs: [{ name: 'chr1', length: 100 }] });
  const prefix = info('/b.bed', { kind: 'regions', contigs: [{ name: '1', length: null }] });
  assert.equal(compareReferenceProfiles([first, prefix]).status, 'warning');
  const unknownLength = info('/c.bed', { kind: 'regions', contigs: [{ name: 'chr1', length: null }] });
  assert.equal(compareReferenceProfiles([first, unknownLength]).status, 'inconclusive');
  const unreadable = info('/d.vcf', { kind: 'variant', header_status: 'error', contigs: [] });
  assert.equal(compareReferenceProfiles([first, unreadable]).status, 'inconclusive');
});

test('equal observed contigs get a qualified result', () => {
  const first = info('/a.bam', { contigs: [{ name: 'chr1', length: 100 }] });
  const second = info('/b.vcf', { kind: 'variant', contigs: [{ name: 'chr1', length: 100 }] });
  const result = compareReferenceProfiles([first, second]);
  assert.equal(result.status, 'no_detected_conflict');
  assert.match(result.details.join(' '), /bevisar inte/);
});
