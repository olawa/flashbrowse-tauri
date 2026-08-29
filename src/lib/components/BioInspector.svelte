<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getBamHeader,
    getBamAlignments,
    generateRsnapSnapshot,
    launchRsnap,
    runRsQc,
  } from '../invoke';
  import { inspectorScroll } from '../stores/navigation';
  import type { FileItem, BamHeaderData, SamViewResult, SamRecord } from '../types';
  import {
    Dna,
    Camera,
    Activity,
    FileText,
    ExternalLink,
    RefreshCw,
    Search,
    ListFilter,
    ChevronLeft,
    ChevronRight,
    Copy,
    Check,
    Terminal,
    Eye,
    Layers,
  } from 'lucide-svelte';

  export let item: FileItem;

  let bamHeader: BamHeaderData | null = null;
  let isLoadingHeader = false;
  let headerError = '';

  let activeTab: 'alignments' | 'header' | 'rsnap' | 'rsqc' | 'raw' = 'alignments';

  // SAM Alignments (samtools view) state
  let samResult: SamViewResult | null = null;
  let isLoadingAlignments = false;
  let alignmentsError = '';
  let alignRegion = '';
  let alignOffset = 0;
  let alignLimit = 50;
  let alignViewMode: 'table' | 'raw' = 'table';
  let copiedRecordIndex: number | null = null;
  let copiedAll = false;
  let alignScrollContainer: HTMLElement;
  let lastScrollPulse = 0;

  // rsnap Snapshot state
  let snapshotRegion = 'chr1:1000000-1005000';
  let snapshotB64: string | null = null;
  let isGeneratingSnapshot = false;
  let snapshotError = '';

  // rs-qc state
  let qcReport: string | null = null;
  let isRunningQc = false;
  let qcError = '';

  // Contig filter
  let contigSearch = '';

  $: if (item) {
    alignOffset = 0;
    loadHeader(item.path);
    loadAlignments(item.path, alignRegion, 0, alignLimit);
  }

  // Remote two-finger scroll listener
  $: if ($inspectorScroll.pulse && $inspectorScroll.pulse !== lastScrollPulse) {
    lastScrollPulse = $inspectorScroll.pulse;
    if (alignScrollContainer && (activeTab === 'alignments' || activeTab === 'raw')) {
      alignScrollContainer.scrollBy({ top: $inspectorScroll.deltaY, left: 0, behavior: 'auto' });
    }
  }

  async function loadHeader(path: string) {
    isLoadingHeader = true;
    headerError = '';
    snapshotB64 = null;
    qcReport = null;
    try {
      bamHeader = await getBamHeader(path);
      if (bamHeader && bamHeader.contigs.length > 0) {
        const first = bamHeader.contigs[0];
        const end = Math.min(first.length, 50000);
        snapshotRegion = `${first.name}:10000-${end}`;
      }
    } catch (e: any) {
      headerError = String(e);
    } finally {
      isLoadingHeader = false;
    }
  }

  async function loadAlignments(path: string, region = '', offset = 0, limit = 50) {
    isLoadingAlignments = true;
    alignmentsError = '';
    try {
      const res = await getBamAlignments(path, region.trim() || undefined, limit, offset);
      samResult = res;
    } catch (e: any) {
      alignmentsError = String(e);
    } finally {
      isLoadingAlignments = false;
    }
  }

  function handleLocusSearch() {
    alignOffset = 0;
    loadAlignments(item.path, alignRegion, 0, alignLimit);
  }

  function handleClearLocus() {
    alignRegion = '';
    alignOffset = 0;
    loadAlignments(item.path, '', 0, alignLimit);
  }

  function handleNextPage() {
    if (samResult && samResult.has_more) {
      alignOffset += alignLimit;
      loadAlignments(item.path, alignRegion, alignOffset, alignLimit);
    }
  }

  function handlePrevPage() {
    if (alignOffset > 0) {
      alignOffset = Math.max(0, alignOffset - alignLimit);
      loadAlignments(item.path, alignRegion, alignOffset, alignLimit);
    }
  }

  async function copyRawRecord(raw: string, index: number) {
    try {
      await navigator.clipboard.writeText(raw);
      copiedRecordIndex = index;
      setTimeout(() => {
        copiedRecordIndex = null;
      }, 1500);
    } catch {}
  }

  async function copyAllVisible() {
    if (!samResult) return;
    try {
      await navigator.clipboard.writeText(samResult.raw_output);
      copiedAll = true;
      setTimeout(() => {
        copiedAll = false;
      }, 1500);
    } catch {}
  }

  async function handleGenerateSnapshot() {
    if (!snapshotRegion.trim()) return;
    isGeneratingSnapshot = true;
    snapshotError = '';
    try {
      const b64 = await generateRsnapSnapshot(
        item.path,
        snapshotRegion.trim(),
        bamHeader?.reference_matched_path
      );
      snapshotB64 = b64;
    } catch (e: any) {
      snapshotError = String(e);
    } finally {
      isGeneratingSnapshot = false;
    }
  }

  async function handleLaunchViewer() {
    try {
      await launchRsnap(
        [item.path],
        snapshotRegion.trim() || alignRegion.trim() || undefined,
        bamHeader?.reference_matched_path
      );
    } catch (e: any) {
      alert(`Kunde inte starta rsnap: ${e}`);
    }
  }

  async function handleRunQc() {
    isRunningQc = true;
    qcError = '';
    try {
      qcReport = await runRsQc(item.path);
    } catch (e: any) {
      qcError = String(e);
    } finally {
      isRunningQc = false;
    }
  }

  $: filteredContigs = (bamHeader?.contigs || []).filter((c) => {
    if (!contigSearch.trim()) return true;
    return c.name.toLowerCase().includes(contigSearch.toLowerCase());
  });

  function getMapqColor(mapq: number): string {
    if (mapq >= 60) return 'text-emerald-400 font-bold';
    if (mapq >= 30) return 'text-cyan-400 font-semibold';
    if (mapq >= 10) return 'text-amber-400';
    return 'text-rose-400 opacity-80';
  }
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden bg-[#0d0f14] text-slate-200 text-xs select-none">
  <!-- Top Genomics Header Ribbon -->
  <div class="px-3 py-2 bg-[#151922] border-b border-[#252d3d] flex items-center justify-between gap-2 shrink-0">
    <div class="flex items-center gap-2 min-w-0">
      <div class="w-6 h-6 rounded bg-emerald-500/20 text-emerald-400 flex items-center justify-center shrink-0">
        <Dna size={14} />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1.5 flex-wrap">
          <span class="font-bold text-xs text-white break-all leading-snug select-text" title={item.path}>{item.name}</span>
          {#if bamHeader?.has_index}
            <span class="px-1.5 py-0.2 rounded bg-emerald-950 text-emerald-400 text-[10px] font-mono font-bold border border-emerald-800 shrink-0">
              {bamHeader.index_type || 'INDEXED'}
            </span>
          {:else}
            <span class="px-1.5 py-0.2 rounded bg-amber-950 text-amber-400 text-[10px] font-mono border border-amber-800 shrink-0">
              NO INDEX
            </span>
          {/if}
        </div>
        {#if bamHeader}
          <div class="text-[11px] text-slate-400 font-mono flex items-center gap-1.5 mt-0.5">
            <span class="text-emerald-300 font-semibold">{bamHeader.detected_reference}</span>
            <span>•</span>
            <span>{bamHeader.total_contigs} contigs ({bamHeader.formatted_genome_length})</span>
          </div>
        {/if}
      </div>
    </div>

    <button
      class="flex items-center gap-1 px-2.5 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs shadow-md shrink-0 transition-colors"
      on:click={handleLaunchViewer}
      title="Öppna interaktiv rsnap desktop viewer"
    >
      <ExternalLink size={12} />
      <span>rsnap</span>
    </button>
  </div>

  <!-- Bio Tabs Bar -->
  <div class="flex items-center gap-1 px-3 pt-1.5 border-b border-[#252d3d] bg-[#11141b] text-xs shrink-0 overflow-x-auto scrollbar-none">
    <button
      class="flex items-center gap-1.5 px-3 py-1.5 border-b-2 font-medium transition-colors {activeTab === 'alignments' ? 'border-emerald-400 text-emerald-300 font-bold bg-white/5' : 'border-transparent text-slate-400 hover:text-white'}"
      on:click={() => (activeTab = 'alignments')}
    >
      <ListFilter size={12} />
      <span>Alignments (samtools view)</span>
    </button>

    <button
      class="flex items-center gap-1.5 px-3 py-1.5 border-b-2 font-medium transition-colors {activeTab === 'header' ? 'border-emerald-400 text-emerald-300 font-bold bg-white/5' : 'border-transparent text-slate-400 hover:text-white'}"
      on:click={() => (activeTab = 'header')}
    >
      <Dna size={12} />
      <span>Header & Contigs</span>
    </button>

    <button
      class="flex items-center gap-1.5 px-3 py-1.5 border-b-2 font-medium transition-colors {activeTab === 'rsnap' ? 'border-emerald-400 text-emerald-300 font-bold bg-white/5' : 'border-transparent text-slate-400 hover:text-white'}"
      on:click={() => (activeTab = 'rsnap')}
    >
      <Camera size={12} />
      <span>rsnap Snapshot</span>
    </button>

    <button
      class="flex items-center gap-1.5 px-3 py-1.5 border-b-2 font-medium transition-colors {activeTab === 'rsqc' ? 'border-purple-400 text-purple-300 font-bold bg-white/5' : 'border-transparent text-slate-400 hover:text-white'}"
      on:click={() => (activeTab = 'rsqc')}
    >
      <Activity size={12} />
      <span>rs-qc</span>
    </button>

    <button
      class="flex items-center gap-1.5 px-3 py-1.5 border-b-2 font-medium transition-colors {activeTab === 'raw' ? 'border-amber-400 text-amber-300 font-bold bg-white/5' : 'border-transparent text-slate-400 hover:text-white'}"
      on:click={() => (activeTab = 'raw')}
    >
      <FileText size={12} />
      <span>Rå SAM</span>
    </button>
  </div>

  <!-- Tab Content -->
  <div bind:this={alignScrollContainer} class="flex-1 overflow-auto p-3 text-xs select-text">
    {#if activeTab === 'alignments'}
      <!-- SAM Alignments (samtools view | less) Mode -->
      <div class="h-full flex flex-col space-y-2">
        <!-- Control Bar: Locus Filter & View Mode & Pagination -->
        <div class="p-2.5 rounded-xl bg-[#151922] border border-[#252d3d] flex items-center justify-between gap-2 flex-wrap shrink-0">
          <div class="flex items-center gap-1.5 flex-1 min-w-[220px]">
            <input
              type="text"
              placeholder="Filtrera locus (t.ex. chr1:10000-50000)..."
              bind:value={alignRegion}
              on:keydown={(e) => e.key === 'Enter' && handleLocusSearch()}
              class="flex-1 bg-[#0e1015] border border-[#252d3d] rounded-lg px-2.5 py-1 text-xs font-mono text-white focus:outline-none focus:border-emerald-400"
            />
            <button
              class="px-2.5 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs transition-colors flex items-center gap-1 shadow"
              on:click={handleLocusSearch}
              title="Kör samtools view med angiven region"
            >
              <Search size={11} />
              <span>Visa</span>
            </button>
            {#if alignRegion}
              <button
                class="px-2 py-1 rounded-lg bg-[#252d3d] hover:bg-white/10 text-slate-300 text-xs"
                on:click={handleClearLocus}
                title="Rensa filter och visa från start"
              >
                Hela filen
              </button>
            {/if}
          </div>

          <!-- View Mode & Pagination Controls -->
          <div class="flex items-center gap-2 shrink-0">
            <!-- View Mode toggle -->
            <div class="flex items-center bg-[#0e1015] rounded-lg p-0.5 border border-[#252d3d]">
              <button
                class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors {alignViewMode === 'table' ? 'bg-emerald-500/20 text-emerald-300 font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (alignViewMode = 'table')}
              >
                Tabell
              </button>
              <button
                class="px-2 py-0.5 rounded text-[11px] font-medium transition-colors {alignViewMode === 'raw' ? 'bg-emerald-500/20 text-emerald-300 font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (alignViewMode = 'raw')}
              >
                less -S (Rå)
              </button>
            </div>

            <!-- Copy All Button -->
            <button
              class="p-1.5 rounded-lg bg-[#1a1f2c] hover:bg-white/10 border border-[#252d3d] text-slate-300 hover:text-white transition-colors"
              on:click={copyAllVisible}
              title="Kopiera alla synliga SAM-rader"
            >
              {#if copiedAll}
                <Check size={12} class="text-emerald-400" />
              {:else}
                <Copy size={12} />
              {/if}
            </button>

            <!-- Paginator -->
            <div class="flex items-center gap-1 font-mono text-[11px] text-slate-400">
              <button
                class="p-1 rounded bg-[#1a1f2c] hover:bg-white/10 border border-[#252d3d] disabled:opacity-30 transition-colors"
                disabled={alignOffset === 0 || isLoadingAlignments}
                on:click={handlePrevPage}
                title="Föregående sida"
              >
                <ChevronLeft size={12} />
              </button>
              <span class="px-1 text-slate-300">{alignOffset + 1}–{alignOffset + (samResult?.records.length || 0)}</span>
              <button
                class="p-1 rounded bg-[#1a1f2c] hover:bg-white/10 border border-[#252d3d] disabled:opacity-30 transition-colors"
                disabled={!samResult?.has_more || isLoadingAlignments}
                on:click={handleNextPage}
                title="Nästa sida"
              >
                <ChevronRight size={12} />
              </button>
            </div>
          </div>
        </div>

        {#if isLoadingAlignments}
          <div class="flex-1 flex items-center justify-center text-slate-400 gap-2 p-8">
            <RefreshCw size={14} class="animate-spin text-emerald-400" />
            <span>Kör samtools view...</span>
          </div>
        {:else if alignmentsError}
          <div class="p-4 rounded-xl bg-red-950/30 border border-red-800 text-red-400 space-y-2">
            <span class="font-bold block">Kunde inte hämta alignments:</span>
            <p class="text-xs font-mono">{alignmentsError}</p>
          </div>
        {:else if samResult && samResult.records.length > 0}
          {#if alignViewMode === 'table'}
            <!-- Rich SAM Table View -->
            <div class="flex-1 border border-[#252d3d] rounded-xl bg-[#0e1015] overflow-x-auto">
              <table class="w-full text-left font-mono text-[11px] border-collapse min-w-[700px]">
                <thead>
                  <tr class="border-b border-[#252d3d] bg-[#1a1f2c] text-slate-400 text-[10px] uppercase tracking-wider sticky top-0 z-10">
                    <th class="p-1.5 pl-3 w-8">#</th>
                    <th class="p-1.5 w-36">QNAME</th>
                    <th class="p-1.5 w-16">FLAG</th>
                    <th class="p-1.5 w-28">POS</th>
                    <th class="p-1.5 w-14">MAPQ</th>
                    <th class="p-1.5 w-20">CIGAR</th>
                    <th class="p-1.5 w-14">TLEN</th>
                    <th class="p-1.5 min-w-[200px]">SEQ</th>
                    <th class="p-1.5 w-32">TAGS</th>
                    <th class="p-1.5 text-right pr-3 w-10"></th>
                  </tr>
                </thead>
                <tbody>
                  {#each samResult.records as r, i}
                    <tr class="border-b border-[#1b202c] hover:bg-emerald-500/5 transition-colors group">
                      <!-- Index -->
                      <td class="p-1.5 pl-3 text-slate-500 text-[10px]">{alignOffset + i + 1}</td>

                      <!-- QNAME -->
                      <td class="p-1.5 font-medium text-slate-200 truncate max-w-[140px]" title={r.qname}>
                        {r.qname}
                      </td>

                      <!-- FLAG -->
                      <td class="p-1.5">
                        <span
                          class="px-1.5 py-0.5 rounded bg-blue-950 text-blue-300 font-bold text-[10px] border border-blue-800/60 cursor-help"
                          title={r.flag_desc.join(', ')}
                        >
                          {r.flag}
                        </span>
                      </td>

                      <!-- POS -->
                      <td class="p-1.5 text-emerald-300 font-semibold truncate">
                        {r.rname}:{r.pos.toLocaleString()}
                      </td>

                      <!-- MAPQ -->
                      <td class="p-1.5 {getMapqColor(r.mapq)}">
                        {r.mapq}
                      </td>

                      <!-- CIGAR -->
                      <td class="p-1.5 text-purple-300 truncate" title={r.cigar}>
                        {r.cigar}
                      </td>

                      <!-- TLEN -->
                      <td class="p-1.5 text-slate-400">
                        {r.tlen !== 0 ? `${r.tlen} bp` : '--'}
                      </td>

                      <!-- SEQ (Colored Nucleotides) -->
                      <td class="p-1.5 font-mono text-[10.5px] truncate max-w-[240px]" title={r.seq}>
                        {#each r.seq.slice(0, 45) as char}
                          {#if char === 'A'}
                            <span class="text-emerald-400 font-bold">A</span>
                          {:else if char === 'C'}
                            <span class="text-sky-400 font-bold">C</span>
                          {:else if char === 'G'}
                            <span class="text-amber-400 font-bold">G</span>
                          {:else if char === 'T'}
                            <span class="text-rose-400 font-bold">T</span>
                          {:else}
                            <span class="text-slate-500">{char}</span>
                          {/if}
                        {/each}
                        {#if r.seq.length > 45}
                          <span class="text-slate-600">...</span>
                        {/if}
                      </td>

                      <!-- TAGS -->
                      <td class="p-1.5 text-[10px] text-slate-400 truncate max-w-[130px]" title={r.tags.join(' ')}>
                        {r.tags.join(' ')}
                      </td>

                      <!-- Action / Copy -->
                      <td class="p-1.5 text-right pr-3">
                        <button
                          class="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-white/10 text-slate-400 hover:text-white transition-opacity"
                          on:click={() => copyRawRecord(r.raw_line, i)}
                          title="Kopiera SAM-rad"
                        >
                          {#if copiedRecordIndex === i}
                            <Check size={11} class="text-emerald-400" />
                          {:else}
                            <Copy size={11} />
                          {/if}
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {:else}
            <!-- Raw SAM text mode (`less -S`) -->
            <div class="flex-1 p-3 rounded-xl bg-[#0c0d10] border border-[#252d3d] font-mono text-[11px] text-slate-300 leading-tight overflow-auto">
              <pre class="m-0 whitespace-pre">{samResult.raw_output}</pre>
            </div>
          {/if}
        {:else}
          <div class="p-12 text-center text-slate-500 space-y-2">
            <ListFilter size={32} class="mx-auto opacity-30" />
            <p>Inga alignment-rader hittades för det angivna sökområdet.</p>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'header'}
      <!-- BAM Header Structured View -->
      <div class="space-y-4">
        <!-- Reference Card -->
        <div class="p-3.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2.5">
          <div class="flex items-center justify-between">
            <span class="font-bold text-xs uppercase tracking-wider text-emerald-400">Detekterat Genombygge</span>
            <span class="px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 font-bold text-[11px]">
              {bamHeader?.detected_reference || 'UNKNOWN'}
            </span>
          </div>

          {#if bamHeader?.reference_matched_path}
            <div class="text-[11px] text-slate-300 font-mono bg-[#0e1015] p-2 rounded border border-[#222837] flex items-center justify-between">
              <span class="truncate" title={bamHeader.reference_matched_path}>Matchad FASTA: {bamHeader.reference_matched_path}</span>
              <span class="text-emerald-400 text-[10px] font-bold">AUTO-LÄNKAD</span>
            </div>
          {/if}

          <!-- Programs (@PG) -->
          {#if bamHeader && bamHeader.programs.length > 0}
            <div class="pt-2 border-t border-[#252d3d] space-y-1.5">
              <span class="text-[11px] font-bold text-slate-300 block">Aligner & Pipeline Program (@PG)</span>
              {#each bamHeader.programs as pg}
                <div class="p-2 rounded bg-[#0e1015] border border-[#222837] space-y-1 font-mono text-[11px]">
                  <div class="flex items-center justify-between font-bold text-slate-200">
                    <span>{pg.name || pg.id}</span>
                    <span class="text-slate-400 font-normal">v{pg.version || 'unknown'}</span>
                  </div>
                  {#if pg.command_line}
                    <p class="text-[10px] text-slate-400 break-words leading-tight bg-black/40 p-1.5 rounded">
                      {pg.command_line}
                    </p>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}

          <!-- Read Groups (@RG) -->
          {#if bamHeader && bamHeader.read_groups.length > 0}
            <div class="pt-2 border-t border-[#252d3d] space-y-1.5">
              <span class="text-[11px] font-bold text-slate-300 block">Read Groups & Prov (@RG)</span>
              {#each bamHeader.read_groups as rg}
                <div class="flex items-center justify-between p-2 rounded bg-[#0e1015] border border-[#222837] text-[11px] font-mono">
                  <div>
                    <span class="text-emerald-400 font-bold">Sample (SM): {rg.sample || rg.id}</span>
                    {#if rg.library}
                      <span class="text-slate-400 block text-[10px]">Lib: {rg.library}</span>
                    {/if}
                  </div>
                  <span class="px-2 py-0.5 rounded bg-blue-950 text-blue-300 text-[10px] font-bold">
                    {rg.platform || 'ILLUMINA'}
                  </span>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Contigs / Chromosomes (@SQ) Table -->
        {#if bamHeader}
          <div class="p-3 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2">
            <div class="flex items-center justify-between">
              <span class="font-bold text-xs uppercase tracking-wider text-slate-300">
                Kromosomer / Contigs ({bamHeader.total_contigs})
              </span>
              <div class="relative">
                <Search size={11} class="absolute left-2 top-1.5 text-slate-500" />
                <input
                  type="text"
                  bind:value={contigSearch}
                  placeholder="Sök contig..."
                  class="bg-[#0e1015] border border-[#252d3d] rounded pl-5 pr-2 py-0.5 text-[11px] text-white focus:outline-none focus:border-emerald-400 w-28"
                />
              </div>
            </div>

            <div class="max-h-56 overflow-auto border border-[#252d3d] rounded-lg bg-[#0e1015]">
              <table class="w-full text-left font-mono text-[11px] border-collapse">
                <thead>
                  <tr class="border-b border-[#252d3d] bg-[#1a1f2c] text-slate-400 text-[10px]">
                    <th class="p-1.5 pl-3">Contig (SN)</th>
                    <th class="p-1.5 text-right pr-3">Längd (LN)</th>
                    <th class="p-1.5 text-right pr-3">Handling</th>
                  </tr>
                </thead>
                <tbody>
                  {#each filteredContigs as c}
                    <tr class="border-b border-[#1f2533] hover:bg-white/5">
                      <td class="p-1.5 pl-3 font-semibold text-emerald-300">{c.name}</td>
                      <td class="p-1.5 text-right pr-3 text-slate-400">{c.formatted_length}</td>
                      <td class="p-1.5 text-right pr-3">
                        <button
                          class="px-1.5 py-0.5 rounded bg-emerald-500/20 hover:bg-emerald-500 text-emerald-300 hover:text-white text-[10px]"
                          on:click={() => {
                            alignRegion = `${c.name}:1-100000`;
                            activeTab = 'alignments';
                            handleLocusSearch();
                          }}
                        >
                          Visa samtools
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'rsnap'}
      <!-- rsnap Snapshot Live Viewer -->
      <div class="space-y-3">
        <div class="p-3 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2">
          <label for="snapshot-region-input" class="block font-bold text-xs text-slate-200">Genomisk Region (Locus)</label>
          <div class="flex items-center gap-2">
            <input
              id="snapshot-region-input"
              type="text"
              bind:value={snapshotRegion}
              placeholder="t.ex. chr1:1000000-1005000 eller chr17:41196312-41277500"
              class="flex-1 bg-[#0e1015] border border-[#252d3d] rounded-lg px-3 py-1.5 text-xs font-mono text-white focus:outline-none focus:border-emerald-400"
              on:keydown={(e) => e.key === 'Enter' && handleGenerateSnapshot()}
            />
            <button
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs shadow-md transition-colors disabled:opacity-50"
              disabled={isGeneratingSnapshot}
              on:click={handleGenerateSnapshot}
            >
              <Camera size={13} class={isGeneratingSnapshot ? 'animate-spin' : ''} />
              <span>{isGeneratingSnapshot ? 'Genererar...' : 'Skapa Snapshot'}</span>
            </button>
          </div>

          <div class="flex items-center justify-between text-[11px] text-slate-400">
            <span>Tryck Enter eller klicka för att rendera snapshot via rsnap</span>
            <button
              class="text-emerald-400 hover:underline flex items-center gap-1 font-semibold"
              on:click={handleLaunchViewer}
            >
              <span>Öppna i interaktiv rsnap GUI</span>
              <ExternalLink size={11} />
            </button>
          </div>
        </div>

        {#if snapshotError}
          <div class="p-3 rounded-xl bg-red-950/30 border border-red-800 text-red-400 text-xs font-mono">
            {snapshotError}
          </div>
        {:else if snapshotB64}
          <div class="p-2 rounded-xl bg-black border border-[#252d3d] flex flex-col items-center space-y-2">
            <img
              src="data:image/png;base64,{snapshotB64}"
              alt="rsnap read alignment snapshot"
              class="w-full object-contain rounded-lg shadow-2xl"
            />
            <div class="w-full flex items-center justify-between px-2 text-[11px] text-slate-400 font-mono">
              <span>Region: {snapshotRegion}</span>
              <span class="text-emerald-400 font-semibold">Renderad med rsnap (~200ms)</span>
            </div>
          </div>
        {:else}
          <div class="p-12 rounded-xl border border-dashed border-[#252d3d] text-center space-y-2 text-slate-500">
            <Camera size={32} class="mx-auto opacity-30" />
            <p class="text-xs">Ange en genomisk koordinat ovan och klicka <strong>Skapa Snapshot</strong> för att förhandsvisa read alignments direkt här.</p>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'rsqc'}
      <!-- rs-qc Alignment Metrics View -->
      <div class="space-y-3">
        <div class="p-3 rounded-xl bg-[#151922] border border-[#252d3d] flex items-center justify-between">
          <div>
            <h4 class="font-bold text-xs text-purple-300">Rapid Sequencing QC (rs-qc align)</h4>
            <p class="text-[11px] text-slate-400 mt-0.5">Mappar, pairing, insert size distribution och cfDNA-diagnostik</p>
          </div>
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white font-semibold text-xs shadow-md transition-colors disabled:opacity-50"
            disabled={isRunningQc}
            on:click={handleRunQc}
          >
            <Activity size={13} class={isRunningQc ? 'animate-spin' : ''} />
            <span>{isRunningQc ? 'Analyserar...' : 'Kör rs-qc'}</span>
          </button>
        </div>

        {#if qcError}
          <div class="p-3 rounded-xl bg-red-950/30 border border-red-800 text-red-400 text-xs font-mono">
            {qcError}
          </div>
        {:else if qcReport}
          <div class="p-4 rounded-xl bg-[#0c0d10] border border-[#252d3d] font-mono text-xs text-slate-200 leading-relaxed overflow-auto">
            <pre class="m-0 whitespace-pre-wrap">{qcReport}</pre>
          </div>
        {:else}
          <div class="p-12 rounded-xl border border-dashed border-[#252d3d] text-center space-y-2 text-slate-500">
            <Activity size={32} class="mx-auto opacity-30 text-purple-400" />
            <p class="text-xs">Klicka på <strong>Kör rs-qc</strong> för att generera alignment-metrik och kvalitetsrapport.</p>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'raw'}
      <!-- Raw SAM Header -->
      <div class="p-3 rounded-xl bg-[#0c0d10] border border-[#252d3d] font-mono text-[11px] text-slate-300 leading-tight overflow-auto max-h-[70vh]">
        <pre class="m-0 whitespace-pre-wrap">{bamHeader?.raw_header || ''}</pre>
      </div>
    {/if}
  </div>
</div>
