<script lang="ts">
  import { onDestroy } from 'svelte';
  import { createGenomicsIndex, inspectGenomicsFiles } from '../invoke';
  import { compareReferenceProfiles, groupSamples, kindFromName } from '../genomicsWorkspaceModel';
  import type { DirectoryIndexGroup, FileItem, GenomicsFileInfo } from '../types';

  export let groups: DirectoryIndexGroup[] = [];
  export let query = '';
  export let onSelectPreview: (item: FileItem) => void = () => {};
  export let onNavigateFolder: (path: string) => void = () => {};

  let tab: 'samples' | 'health' | 'compatibility' = 'samples';
  let infoByPath: Record<string, GenomicsFileInfo> = {};
  let selectedPaths = new Set<string>();
  let isAnalyzing = false;
  let analysisCount = 0;
  let analysisTotal = 0;
  let metadataCursor = 0;
  let sampleLimit = 100;
  let healthLimit = 100;
  let compareLimit = 100;
  let analysisError = '';
  let indexingPath = '';
  let actionError = '';
  let generation = 0;

  $: files = groups.flatMap((group) => group.items);
  $: visibleFiles = files.filter((item) => !query.trim() ||
    `${item.name} ${item.path}`.toLowerCase().includes(query.trim().toLowerCase()));
  $: samples = groupSamples(visibleFiles, infoByPath);
  $: visibleSamples = samples.rows.slice(0, sampleLimit);
  $: visibleHealthFiles = visibleFiles.slice(0, healthLimit);
  $: selectableFiles = visibleFiles.filter((item) => ['alignment', 'variant', 'regions', 'reference'].includes(kindFromName(item.name)));
  $: selectedInfos = [...selectedPaths].map((path) => infoByPath[path]).filter((info): info is GenomicsFileInfo => !!info);
  $: comparison = compareReferenceProfiles(selectedInfos);
  $: if (selectedPaths.size > selectedInfos.length && comparison.status === 'no_detected_conflict') {
    comparison = {
      status: 'warning',
      headline: 'Ofullständig jämförelse',
      details: [...comparison.details, `${selectedPaths.size - selectedInfos.length} valda filer saknar analyserad metadata.`],
    };
  }
  $: metadataButtonLabel = metadataCursor === 0 ? 'Läs metadata' : metadataCursor < files.length ? 'Läs nästa 500' : 'Läs om metadata';
  $: if (groups) {
    selectedPaths = new Set();
    metadataCursor = 0;
    actionError = '';
    sampleLimit = 100;
    healthLimit = 100;
    compareLimit = 100;
    void analyze(files, 'initial');
  }
  $: if (query) {
    sampleLimit = 100;
    healthLimit = 100;
    compareLimit = 100;
  }

  onDestroy(() => { generation++; });

  async function analyze(source: FileItem[], mode: 'initial' | 'next') {
    const request = ++generation;
    let start = mode === 'initial' ? 0 : metadataCursor;
    if (start >= source.length) start = 0;
    if (start === 0) infoByPath = {};
    analysisCount = start;
    analysisTotal = Math.min(source.length, start + 500);
    analysisError = '';
    if (!source.length || (mode === 'initial' && source.length > 200)) {
      metadataCursor = 0;
      isAnalyzing = false;
      return;
    }
    isAnalyzing = true;
    try {
      for (let offset = start; offset < analysisTotal; offset += 30) {
        const batch = await inspectGenomicsFiles(source.slice(offset, Math.min(offset + 30, analysisTotal)).map((item) => item.path));
        if (request !== generation) return;
        infoByPath = { ...infoByPath, ...Object.fromEntries(batch.map((info) => [info.path, info])) };
        analysisCount = Math.min(offset + batch.length, source.length);
        metadataCursor = analysisCount;
      }
    } catch (error) {
      if (request === generation) analysisError = String(error);
    } finally {
      if (request === generation) isAnalyzing = false;
    }
  }

  async function quickcheckVisible() {
    if (isAnalyzing) return;
    const paths = visibleHealthFiles.filter((item) => kindFromName(item.name) === 'alignment').map((item) => item.path);
    if (!paths.length) return;
    const request = ++generation;
    isAnalyzing = true;
    analysisError = '';
    analysisCount = 0;
    analysisTotal = paths.length;
    try {
      for (let offset = 0; offset < paths.length; offset += 30) {
        const batch = await inspectGenomicsFiles(paths.slice(offset, offset + 30), true);
        if (request !== generation) return;
        infoByPath = { ...infoByPath, ...Object.fromEntries(batch.map((info) => [info.path, info])) };
        analysisCount = Math.min(offset + batch.length, paths.length);
      }
    } catch (error) {
      if (request === generation) analysisError = String(error);
    } finally {
      if (request === generation) isAnalyzing = false;
    }
  }

  async function createIndex(item: FileItem) {
    if (indexingPath || isAnalyzing) return;
    indexingPath = item.path;
    actionError = '';
    try {
      const updated = await createGenomicsIndex(item.path);
      infoByPath = { ...infoByPath, [item.path]: updated };
    } catch (error) {
      actionError = `${item.name}: ${error}`;
    } finally {
      indexingPath = '';
    }
  }

  function toggleSelected(path: string) {
    const next = new Set(selectedPaths);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    selectedPaths = next;
  }

  function healthLabel(info?: GenomicsFileInfo): string {
    if (!info) return 'Ej analyserad';
    if (info.quickcheck_status === 'failed') return 'Snabbkontroll underkänd';
    if (info.header_status === 'error') return 'Metadatafel';
    if (info.index_status === 'missing') return 'Index saknas';
    if (info.index_status === 'stale') return 'Index äldre än fil';
    if (info.quickcheck_status === 'unavailable') return 'Snabbkontroll ej tillgänglig';
    if (info.header_status === 'unavailable') return 'Verktyg saknas';
    if (info.quickcheck_status === 'passed') return 'Snabbkontroll godkänd';
    if (info.index_status === 'present') return 'Index finns';
    return 'Ingen kontroll körd';
  }

  function healthColor(info?: GenomicsFileInfo): string {
    if (!info) return 'text-[var(--text-muted)]';
    if (info.quickcheck_status === 'failed' || info.header_status === 'error') return 'text-red-300';
    if (info.index_status === 'missing' || info.index_status === 'stale') return 'text-amber-300';
    if (info.quickcheck_status === 'passed') return 'text-emerald-300';
    return 'text-[var(--text-secondary)]';
  }

  function quickcheckLabel(info?: GenomicsFileInfo): string {
    if (info?.quickcheck_status === 'passed') return 'Snabbkontroll OK';
    if (info?.quickcheck_status === 'failed') return 'Snabbkontroll fel';
    if (info?.quickcheck_status === 'unavailable') return 'Snabbkontroll saknas';
    return '';
  }
</script>

<div class="flex-1 min-h-0 flex flex-col overflow-hidden bg-[var(--bg-base)] text-[var(--text-primary)]">
  <div class="px-4 py-3 border-b border-[var(--border)] bg-[var(--bg-surface)] flex flex-wrap items-center justify-between gap-3">
    <div>
      <div class="text-sm font-semibold">Genomikarbetsyta</div>
      <div class="text-[11px] text-[var(--text-muted)]">Lokala filer · {files.length} filer · {Object.keys(infoByPath).length} analyserade</div>
    </div>
    <div class="flex items-center gap-2 text-xs">
      <button
        class="px-2.5 py-1 rounded border border-[var(--border)] hover:bg-[var(--bg-hover)] disabled:opacity-50"
        disabled={isAnalyzing || !!indexingPath || !files.length}
        on:click={() => analyze(files, 'next')}
      >{metadataButtonLabel}</button>
      {#if isAnalyzing}<span class="text-amber-300">Läser {analysisCount}/{analysisTotal}...</span>{/if}
    </div>
  </div>

  {#if files.length > 200 && !isAnalyzing && Object.keys(infoByPath).length === 0}
    <p class="px-4 py-2 text-xs text-amber-300">Stort projekt: klicka ”Läs metadata” för att läsa upp till 500 filhuvuden åt gången. Provnamnen nedan är tills vidare gissade från filnamnen.</p>
  {/if}
  {#if analysisError || actionError}
    <p class="px-4 py-2 text-xs text-red-300" role="alert">{analysisError || actionError}</p>
  {/if}

  <div class="px-4 pt-3 flex items-center gap-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] text-xs">
    <button class="px-3 py-1.5 rounded-t {tab === 'samples' ? 'bg-[var(--accent-subtle)] text-[var(--accent)] font-semibold' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'}" on:click={() => tab = 'samples'}>Prover ({samples.rows.length})</button>
    <button class="px-3 py-1.5 rounded-t {tab === 'health' ? 'bg-[var(--accent-subtle)] text-[var(--accent)] font-semibold' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'}" on:click={() => tab = 'health'}>Filhälsa</button>
    <button class="px-3 py-1.5 rounded-t {tab === 'compatibility' ? 'bg-[var(--accent-subtle)] text-[var(--accent)] font-semibold' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'}" on:click={() => tab = 'compatibility'}>Jämför referenser ({selectedPaths.size})</button>
  </div>

  {#if tab === 'samples'}
    <div class="flex-1 overflow-auto p-4 space-y-2">
      <p class="text-[11px] text-[var(--text-muted)] mb-2">Kopplingar via filnamn är förslag. Provnamn i BAM/VCF-huvuden gäller för just den filen. Filer med flera prov visas separat.</p>
      {#each visibleSamples as row (row.name)}
        <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-surface)] px-3 py-2 text-xs">
          <div class="flex flex-wrap items-center gap-2">
            <strong class="text-[var(--text-primary)]">{row.name}</strong>
            <span class="text-[10px] {row.evidence === 'header' ? 'text-emerald-300' : 'text-amber-300'}">{row.evidence === 'header' ? 'minst en fil med provnamn i header' : 'filnamnsförslag'}</span>
            {#each row.issues as issue}<span class="px-1.5 py-0.5 rounded bg-amber-900/40 text-amber-300 text-[10px]">{issue}</span>{/each}
          </div>
          <div class="grid grid-cols-4 gap-2 mt-2 text-[11px] text-[var(--text-secondary)]">
            <span>R1 <b>{row.r1.length}</b></span><span>R2 <b>{row.r2.length}</b></span>
            <span>BAM/CRAM <b>{row.alignments.length}</b></span><span>VCF/BCF <b>{row.variants.length}</b></span>
          </div>
          <div class="flex flex-wrap gap-1 mt-2">
            {#each [...row.r1, ...row.r2, ...row.otherReads, ...row.alignments, ...row.variants] as item (item.path)}
              <button class="px-1.5 py-0.5 rounded bg-[var(--bg-panel)] hover:bg-[var(--bg-hover)] text-[10px] font-mono truncate max-w-52" title={`${item.path} · ${infoByPath[item.path]?.sample_source === 'header' ? 'provnamn i header' : 'filnamnsförslag'}`} on:click={() => onSelectPreview(item)}>{item.name}</button>
            {/each}
          </div>
        </div>
      {:else}
        <p class="text-xs text-[var(--text-muted)]">Inga provfiler i sökningen.</p>
      {/each}
      {#if samples.rows.length > sampleLimit}<button class="text-xs text-[var(--accent)] underline" on:click={() => sampleLimit += 100}>Visa fler prover ({samples.rows.length - sampleLimit} kvar)</button>{/if}
      {#if samples.shared.length}
        <div class="rounded-lg border border-[var(--border)] p-3 text-xs">
          <strong>Filer utan entydigt prov ({samples.shared.length})</strong>
          <div class="flex flex-wrap gap-1 mt-2">
            {#each samples.shared as item (item.path)}<button class="underline text-sky-300 text-[11px]" title={item.path} on:click={() => onSelectPreview(item)}>{item.name}</button>{/each}
          </div>
        </div>
      {/if}
    </div>
  {:else if tab === 'health'}
    <div class="flex-1 overflow-auto p-4">
      <div class="flex flex-wrap justify-between gap-2 mb-3 text-[11px]">
        <span class="text-[var(--text-muted)]">Indexstatus bygger på filernas ändringstid. Snabbkontrollen läser BAM/CRAM-filernas början och slut, inte hela innehållet.</span>
        <button class="px-2.5 py-1 rounded border border-[var(--border)] hover:bg-[var(--bg-hover)] disabled:opacity-50" disabled={isAnalyzing || !!indexingPath || !visibleHealthFiles.some((f) => kindFromName(f.name) === 'alignment')} on:click={quickcheckVisible}>Snabbkontrollera visade BAM/CRAM</button>
      </div>
      <div class="space-y-1">
        {#each visibleHealthFiles as item (item.path)}
          {@const info = infoByPath[item.path]}
          <div class="rounded border border-[var(--border)] bg-[var(--bg-surface)] px-2 py-1.5 flex items-center gap-2 text-[11px]">
            <input type="checkbox" checked={selectedPaths.has(item.path)} on:change={() => toggleSelected(item.path)} aria-label={`Välj ${item.name} för referensjämförelse`} />
            <button class="font-mono truncate flex-1 text-left hover:underline" title={item.path} on:click={() => onSelectPreview(item)}>{item.name}</button>
            <button class="text-[var(--text-muted)] hidden sm:inline truncate max-w-36 hover:underline" title={item.path} on:click={() => onNavigateFolder(item.path.substring(0, item.path.lastIndexOf('/')) || '/')}>{item.path.substring(0, item.path.lastIndexOf('/'))}</button>
            <span class="shrink-0 {healthColor(info)}" title={info?.quickcheck_message || info?.header_message || info?.index_path || ''}>{healthLabel(info)}</span>
            {#if info && info.quickcheck_status !== 'not_run'}
              <span class="shrink-0 text-[10px] {healthColor(info)}" title={info.quickcheck_message || ''}>{quickcheckLabel(info)}</span>
            {/if}
            {#if info && (info.index_status === 'missing' || info.index_status === 'stale')}
              <button class="shrink-0 px-1.5 py-0.5 rounded bg-amber-700/30 text-amber-300 hover:bg-amber-700/50 disabled:opacity-50" disabled={!!indexingPath || isAnalyzing} on:click={() => createIndex(item)}>{indexingPath === item.path ? 'Bygger...' : info.index_status === 'stale' ? 'Bygg om index' : 'Skapa index'}</button>
            {/if}
          </div>
        {:else}
          <p class="text-xs text-[var(--text-muted)]">Inga filer i sökningen.</p>
        {/each}
      </div>
      {#if visibleFiles.length > healthLimit}<button class="mt-3 text-xs text-[var(--accent)] underline" on:click={() => healthLimit += 100}>Visa fler filer ({visibleFiles.length - healthLimit} kvar)</button>{/if}
    </div>
  {:else}
    <div class="flex-1 overflow-auto p-4 grid gap-4 lg:grid-cols-2 content-start">
      <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-xs">
        <strong>Välj filer att jämföra</strong>
        <p class="text-[11px] text-[var(--text-muted)] my-2">Välj minst två BAM/CRAM-, VCF/BCF-, BED- eller FASTA-filer. FASTQ saknar referensprofil.</p>
        <div class="max-h-96 overflow-auto space-y-1">
          {#each selectableFiles.slice(0, compareLimit) as item (item.path)}
            <label class="flex items-center gap-2 p-1 rounded hover:bg-[var(--bg-hover)] cursor-pointer">
              <input type="checkbox" checked={selectedPaths.has(item.path)} on:change={() => toggleSelected(item.path)} />
              <span class="truncate font-mono" title={item.path}>{item.name}</span>
            </label>
          {/each}
        </div>
        {#if selectableFiles.length > compareLimit}<button class="mt-2 text-[var(--accent)] underline" on:click={() => compareLimit += 100}>Visa fler filer ({selectableFiles.length - compareLimit} kvar)</button>{/if}
      </div>
      <div class="rounded-lg border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-xs">
        <strong class="{comparison.status === 'conflict' ? 'text-red-300' : comparison.status === 'warning' ? 'text-amber-300' : comparison.status === 'no_detected_conflict' ? 'text-emerald-300' : 'text-[var(--text-primary)]'}">{comparison.headline}</strong>
        {#if selectedPaths.size > selectedInfos.length}<p class="mt-2 text-amber-300">{selectedPaths.size - selectedInfos.length} valda filer saknar analyserad metadata.</p>{/if}
        <ul class="mt-2 space-y-2 text-[var(--text-secondary)]">
          {#each comparison.details as detail}<li>• {detail}</li>{/each}
        </ul>
        {#if selectedInfos.length}
          <div class="mt-3 border-t border-[var(--border)] pt-2 space-y-1">
            {#each selectedInfos as info (info.path)}
              <div class="font-mono text-[10px] truncate" title={info.path}>{info.path.split('/').pop()}: {info.build || 'okänt bygge'} · {info.contigs.length} contiger{info.contigs_truncated ? ' (första 1 000)' : ''}</div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
