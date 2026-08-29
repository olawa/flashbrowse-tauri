<script lang="ts">
  import {
    getBamHeader,
    getBamAlignments,
    generateRsnapSnapshot,
    launchRsnap,
    runRsQc,
  } from '../invoke';
  import type { FileItem, BamHeaderData, SamViewResult, SamRecord } from '../types';
  import {
    Dna,
    Camera,
    Activity,
    ExternalLink,
    RefreshCw,
    Search,
    ListFilter,
    ChevronLeft,
    ChevronRight,
    Copy,
    Check,
    FileText,
    Layers,
    X,
  } from 'lucide-svelte';

  export let item: FileItem;

  // Tabs: 'header' is the primary default!
  let activeTab: 'header' | 'alignments' | 'rsnap' | 'rsqc' = 'header';

  let bamHeader: BamHeaderData | null = null;
  let isLoadingHeader = false;
  let headerError = '';

  // SAM Alignments (samtools view) state
  let samResult: SamViewResult | null = null;
  let isLoadingAlignments = false;
  let alignmentsError = '';
  let alignRegion = '';
  let alignOffset = 0;
  let alignLimit = 500;
  let alignViewMode: 'raw' | 'table' = 'raw';
  let copiedRecordIndex: number | null = null;
  let copiedAll = false;

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
    if (activeTab === 'alignments') {
      loadAlignments(item.path, alignRegion, alignOffset, alignLimit);
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
      samResult = await getBamAlignments(path, region.trim() || undefined, limit, offset);
    } catch (e: any) {
      alignmentsError = String(e);
      samResult = null;
    } finally {
      isLoadingAlignments = false;
    }
  }

  function handleTabChange(tab: 'header' | 'alignments' | 'rsnap' | 'rsqc') {
    activeTab = tab;
    if (tab === 'alignments' && (!samResult || samResult.offset !== alignOffset)) {
      loadAlignments(item.path, alignRegion, alignOffset, alignLimit);
    }
  }

  function handleSearchLocus() {
    alignOffset = 0;
    loadAlignments(item.path, alignRegion, 0, alignLimit);
  }

  function handleClearLocus() {
    alignRegion = '';
    alignOffset = 0;
    loadAlignments(item.path, '', 0, alignLimit);
  }

  function handleNextPage() {
    alignOffset += alignLimit;
    loadAlignments(item.path, alignRegion, alignOffset, alignLimit);
  }

  function handlePrevPage() {
    if (alignOffset >= alignLimit) {
      alignOffset -= alignLimit;
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
    if (!samResult || samResult.records.length === 0) return;
    try {
      const allText = samResult.records.map((r) => r.raw_line).join('\n');
      await navigator.clipboard.writeText(allText);
      copiedAll = true;
      setTimeout(() => {
        copiedAll = false;
      }, 1500);
    } catch {}
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

  function getMapqColor(mapq: number): string {
    if (mapq >= 60) return 'text-emerald-400 font-bold';
    if (mapq >= 30) return 'text-cyan-400 font-semibold';
    if (mapq >= 10) return 'text-amber-400';
    return 'text-rose-400 opacity-80';
  }

  $: filteredContigs = (bamHeader?.contigs || []).filter((c) => {
    if (!contigSearch.trim()) return true;
    return c.name.toLowerCase().includes(contigSearch.toLowerCase());
  });
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden bg-[#0d0f14] text-slate-200 text-xs select-none">
  <!-- Top Navigation Header with Tabs (Header is default) -->
  <div class="px-3 py-2 bg-[#151922] border-b border-[#252d3d] flex flex-col gap-2 shrink-0">
    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <div class="w-6 h-6 rounded bg-emerald-500/20 text-emerald-400 flex items-center justify-center shrink-0">
          <Dna size={14} />
        </div>
        <span class="font-bold text-xs text-white truncate max-w-[200px]" title={item.path}>{item.name}</span>
        {#if bamHeader?.detected_reference}
          <span class="px-1.5 py-0.2 rounded bg-emerald-500/20 text-emerald-300 font-mono text-[10px] font-bold border border-emerald-500/40 shrink-0">
            {bamHeader.detected_reference}
          </span>
        {/if}
        {#if bamHeader?.has_index}
          <span class="px-1.5 py-0.2 rounded bg-emerald-950 text-emerald-400 font-mono text-[9.5px] border border-emerald-800 shrink-0 font-bold">
            {bamHeader.index_type || 'BAI'}
          </span>
        {:else}
          <span class="px-1.5 py-0.2 rounded bg-amber-950 text-amber-400 font-mono text-[9.5px] border border-amber-800 shrink-0">
            NO INDEX
          </span>
        {/if}
      </div>

      <!-- rsnap Desktop Launch button -->
      <button
        class="flex items-center gap-1 px-2.5 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs shadow-md shrink-0 transition-colors"
        on:click={handleLaunchViewer}
        title="Öppna desktop viewer i rsnap"
      >
        <ExternalLink size={12} />
        <span>rsnap</span>
      </button>
    </div>

    <!-- Tab Bar -->
    <div class="flex items-center gap-1 border-b border-[#252d3d] pb-1 pt-0.5">
      <button
        class="flex items-center gap-1.5 px-3 py-1 rounded font-medium transition-colors text-xs {activeTab === 'header' ? 'bg-emerald-500/20 text-emerald-300 font-bold border-b-2 border-emerald-400' : 'text-slate-400 hover:text-white'}"
        on:click={() => handleTabChange('header')}
      >
        <Dna size={13} />
        <span>Header ({bamHeader?.total_contigs || 0} contigs)</span>
      </button>

      <button
        class="flex items-center gap-1.5 px-3 py-1 rounded font-medium transition-colors text-xs {activeTab === 'alignments' ? 'bg-emerald-500/20 text-emerald-300 font-bold border-b-2 border-emerald-400' : 'text-slate-400 hover:text-white'}"
        on:click={() => handleTabChange('alignments')}
      >
        <ListFilter size={13} />
        <span>Alignments (samtools view)</span>
      </button>

      <button
        class="flex items-center gap-1.5 px-3 py-1 rounded font-medium transition-colors text-xs {activeTab === 'rsnap' ? 'bg-amber-500/20 text-amber-300 font-bold border-b-2 border-amber-400' : 'text-slate-400 hover:text-white'}"
        on:click={() => handleTabChange('rsnap')}
      >
        <Camera size={13} />
        <span>rsnap Snapshot</span>
      </button>

      <button
        class="flex items-center gap-1.5 px-3 py-1 rounded font-medium transition-colors text-xs {activeTab === 'rsqc' ? 'bg-purple-500/20 text-purple-300 font-bold border-b-2 border-purple-400' : 'text-slate-400 hover:text-white'}"
        on:click={() => handleTabChange('rsqc')}
      >
        <Activity size={13} />
        <span>rs-qc</span>
      </button>
    </div>
  </div>

  <!-- TAB BODY -->
  <div class="flex-1 overflow-y-auto overflow-x-hidden p-3 space-y-4">
    <!-- TAB 1: HEADER (Primary Default) -->
    {#if activeTab === 'header'}
      {#if isLoadingHeader}
        <div class="flex items-center justify-center p-12 text-slate-400 gap-2">
          <RefreshCw size={16} class="animate-spin text-emerald-400" />
          <span>Läser in BAM/CRAM header...</span>
        </div>
      {:else if headerError}
        <div class="p-4 rounded-xl bg-red-950/30 border border-red-800 text-red-400 space-y-2">
          <span class="font-bold">Kunde inte läsa BAM-header:</span>
          <p class="font-mono text-xs">{headerError}</p>
        </div>
      {:else if bamHeader}
        <!-- Reference & General Info Card -->
        <div class="p-3.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2.5">
          <div class="flex items-center justify-between flex-wrap gap-2">
            <div class="flex items-center gap-2">
              <span class="font-bold text-slate-300 text-xs">Referens:</span>
              <span class="px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 font-bold font-mono text-xs">
                {bamHeader.detected_reference}
              </span>
            </div>
            <div class="text-[11px] font-mono text-slate-400">
              {bamHeader.total_contigs} contigs • Totalt {bamHeader.formatted_genome_length}
            </div>
          </div>

          {#if bamHeader.reference_matched_path}
            <div class="text-[11px] text-slate-400 font-mono flex items-center gap-1.5 pt-1 border-t border-[#252d3d]/50">
              <span class="text-emerald-400 font-semibold shrink-0">FASTA:</span>
              <span class="truncate text-slate-300" title={bamHeader.reference_matched_path}>{bamHeader.reference_matched_path}</span>
            </div>
          {/if}
        </div>

        <!-- Read Groups (@RG) -->
        {#if bamHeader.read_groups.length > 0}
          <div class="p-3.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2">
            <span class="font-bold text-xs text-slate-300 flex items-center gap-1.5">
              <span>Read Groups (@RG)</span>
              <span class="text-[10px] px-1.5 py-0.2 rounded-full bg-[#252d3d] text-slate-400 font-mono">
                {bamHeader.read_groups.length}
              </span>
            </span>
            <div class="divide-y divide-[#252d3d]/40">
              {#each bamHeader.read_groups as rg}
                <div class="py-1.5 flex items-center justify-between gap-2 font-mono text-[11px]">
                  <div class="flex items-center gap-2 min-w-0">
                    <span class="text-emerald-400 font-bold">{rg.id}</span>
                    {#if rg.sample}
                      <span class="text-slate-300">SM: <strong class="text-white">{rg.sample}</strong></span>
                    {/if}
                    {#if rg.library}
                      <span class="text-slate-400">LB: {rg.library}</span>
                    {/if}
                  </div>
                  {#if rg.platform}
                    <span class="px-1.5 py-0.2 rounded bg-blue-950 text-blue-300 text-[10px] font-bold border border-blue-800">
                      {rg.platform}
                    </span>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Programs / Pipeline (@PG) -->
        {#if bamHeader.programs.length > 0}
          <div class="p-3.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2">
            <span class="font-bold text-xs text-slate-300 flex items-center gap-1.5">
              <span>Aligners & Pipeline (@PG)</span>
              <span class="text-[10px] px-1.5 py-0.2 rounded-full bg-[#252d3d] text-slate-400 font-mono">
                {bamHeader.programs.length}
              </span>
            </span>
            <div class="divide-y divide-[#252d3d]/40">
              {#each bamHeader.programs as pg}
                <div class="py-1.5 space-y-0.5 font-mono text-[11px]">
                  <div class="flex items-center justify-between">
                    <span class="text-amber-400 font-bold">{pg.name || pg.id}</span>
                    <span class="text-slate-400">v{pg.version || '1.0'}</span>
                  </div>
                  {#if pg.command_line}
                    <div class="text-[10px] text-slate-500 font-mono truncate max-w-full" title={pg.command_line}>
                      {pg.command_line}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Contigs List -->
        <div class="p-3.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2.5">
          <div class="flex items-center justify-between gap-2">
            <span class="font-bold text-xs text-slate-300">Contigs / Sekvenser ({bamHeader.total_contigs})</span>
            <div class="relative">
              <input
                type="text"
                placeholder="Filtrera contig..."
                bind:value={contigSearch}
                class="bg-[#0e1015] border border-[#252d3d] rounded px-2 py-0.5 text-[11px] font-mono text-white focus:outline-none focus:border-emerald-400 w-36"
              />
            </div>
          </div>

          <div class="grid grid-cols-2 sm:grid-cols-3 gap-1.5 max-h-56 overflow-y-auto">
            {#each filteredContigs as c}
              <button
                class="p-1.5 rounded bg-[#0e1015] hover:bg-emerald-500/15 border border-[#222837] text-left transition-colors flex items-center justify-between group"
                on:click={() => {
                  alignRegion = `${c.name}:1-100000`;
                  handleTabChange('alignments');
                }}
                title="Klicka för att inspektera i samtools view"
              >
                <span class="font-mono font-semibold text-[11px] text-slate-200 group-hover:text-emerald-300">{c.name}</span>
                <span class="font-mono text-[10px] text-slate-500">{c.formatted_length}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

    <!-- TAB 2: ALIGNMENTS (samtools view) -->
    {:else if activeTab === 'alignments'}
      <!-- Toolbar: Locus search, Paginator, View Mode -->
      <div class="p-2.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2">
        <div class="flex items-center justify-between flex-wrap gap-2">
          <!-- Region input -->
          <div class="flex items-center gap-1.5 flex-1 min-w-[220px]">
            <input
              type="text"
              placeholder="Genomiskt locus (t.ex. chr1:10000-50000)..."
              bind:value={alignRegion}
              on:keydown={(e) => e.key === 'Enter' && handleSearchLocus()}
              class="flex-1 bg-[#0e1015] border border-[#252d3d] rounded-lg px-2.5 py-1 text-xs font-mono text-white focus:outline-none focus:border-emerald-400"
            />
            <button
              class="px-3 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs transition-colors"
              on:click={handleSearchLocus}
            >
              Visa
            </button>
            {#if alignRegion}
              <button
                class="p-1 text-slate-400 hover:text-white"
                on:click={handleClearLocus}
                title="Rensa filter och visa hela filen"
              >
                <X size={13} />
              </button>
            {/if}
          </div>

          <!-- Controls: View Mode & Paginator -->
          <div class="flex items-center gap-2">
            <!-- View Mode -->
            <div class="flex items-center bg-[#0e1015] rounded-md p-0.5 border border-[#252d3d]">
              <button
                class="px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors {alignViewMode === 'table' ? 'bg-emerald-500/20 text-emerald-300 font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (alignViewMode = 'table')}
              >
                Tabell
              </button>
              <button
                class="px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors {alignViewMode === 'raw' ? 'bg-emerald-500/20 text-emerald-300 font-bold' : 'text-slate-400 hover:text-white'}"
                on:click={() => (alignViewMode = 'raw')}
              >
                less -S
              </button>
            </div>

            <!-- Copy All -->
            <button
              class="p-1.5 rounded bg-[#0e1015] hover:bg-white/10 border border-[#252d3d] text-slate-300 hover:text-white transition-colors"
              on:click={copyAllVisible}
              title="Kopiera alla synliga SAM-rader"
            >
              {#if copiedAll}
                <Check size={13} class="text-emerald-400" />
              {:else}
                <Copy size={13} />
              {/if}
            </button>

            <!-- Paginator -->
            <div class="flex items-center gap-1.5">
              <button
                class="px-2 py-1 rounded bg-[#0e1015] hover:bg-white/10 border border-[#252d3d] text-slate-300 disabled:opacity-40 flex items-center gap-1 text-[11px] font-medium"
                disabled={alignOffset === 0}
                on:click={handlePrevPage}
                title="Föregående {alignLimit} rader"
              >
                <ChevronLeft size={13} />
                <span>-{alignLimit}</span>
              </button>
              <span class="font-mono text-[11px] text-emerald-400 font-bold px-1">
                Rad {alignOffset + 1}–{alignOffset + (samResult?.records.length || 0)}
              </span>
              <button
                class="px-2 py-1 rounded bg-[#0e1015] hover:bg-white/10 border border-[#252d3d] text-slate-300 disabled:opacity-40 flex items-center gap-1 text-[11px] font-medium"
                disabled={!samResult?.has_more}
                on:click={handleNextPage}
                title="Nästa {alignLimit} rader"
              >
                <span>+{alignLimit}</span>
                <ChevronRight size={13} />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Alignments Content -->
      {#if isLoadingAlignments}
        <div class="flex items-center justify-center p-12 text-slate-400 gap-2">
          <RefreshCw size={16} class="animate-spin text-emerald-400" />
          <span>Kör samtools view...</span>
        </div>
      {:else if alignmentsError}
        <div class="p-4 rounded-xl bg-red-950/30 border border-red-800 text-red-400 space-y-1 font-mono text-xs">
          <span class="font-bold">Fel vid samtools view:</span>
          <p>{alignmentsError}</p>
        </div>
      {:else if samResult && samResult.records.length > 0}
        {#if alignViewMode === 'table'}
          <div class="border border-[#252d3d] rounded-xl bg-[#0e1015] overflow-x-auto shadow-inner">
            <table class="w-full text-left font-mono text-[11px] border-collapse min-w-[650px]">
              <thead>
                <tr class="border-b border-[#252d3d] bg-[#161a24] text-slate-400 text-[10px] uppercase tracking-wider sticky top-0 z-10">
                  <th class="p-1.5 pl-3 w-8">#</th>
                  <th class="p-1.5 w-32">QNAME</th>
                  <th class="p-1.5 w-12">FLAG</th>
                  <th class="p-1.5 w-24">POS</th>
                  <th class="p-1.5 w-12">MAPQ</th>
                  <th class="p-1.5 w-16">CIGAR</th>
                  <th class="p-1.5 w-12">TLEN</th>
                  <th class="p-1.5 min-w-[180px]">SEQ</th>
                  <th class="p-1.5 w-24">TAGS</th>
                  <th class="p-1.5 text-right pr-2 w-8"></th>
                </tr>
              </thead>
              <tbody>
                {#each samResult.records as r, i}
                  <tr class="border-b border-[#1b202c] hover:bg-emerald-500/5 transition-colors group">
                    <td class="p-1.5 pl-3 text-slate-600 text-[10px]">{alignOffset + i + 1}</td>
                    <td class="p-1.5 font-medium text-slate-200 truncate max-w-[130px]" title={r.qname}>{r.qname}</td>
                    <td class="p-1.5">
                      <span
                        class="px-1.5 py-0.2 rounded bg-blue-950 text-blue-300 font-bold text-[10px] border border-blue-800/60 cursor-help"
                        title={r.flag_desc.join(', ')}
                      >
                        {r.flag}
                      </span>
                    </td>
                    <td class="p-1.5 text-emerald-300 font-semibold truncate">{r.rname}:{r.pos.toLocaleString()}</td>
                    <td class="p-1.5 {getMapqColor(r.mapq)}">{r.mapq}</td>
                    <td class="p-1.5 text-purple-300 truncate" title={r.cigar}>{r.cigar}</td>
                    <td class="p-1.5 text-slate-400">{r.tlen !== 0 ? `${r.tlen}` : '--'}</td>
                    <!-- Colored nucleotides -->
                    <td class="p-1.5 font-mono text-[10.5px] truncate max-w-[200px]" title={r.seq}>
                      {#each r.seq.slice(0, 38) as char}
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
                      {#if r.seq.length > 38}
                        <span class="text-slate-600">...</span>
                      {/if}
                    </td>
                    <td class="p-1.5 text-[10px] text-slate-500 truncate max-w-[100px]" title={r.tags.join(' ')}>{r.tags.join(' ')}</td>
                    <td class="p-1.5 text-right pr-2">
                      <button
                        class="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-white/10 text-slate-400 hover:text-white transition-opacity"
                        on:click={() => copyRawRecord(r.raw_line, i)}
                        title="Kopiera rad"
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
          <!-- Raw Text View (less -S) -->
          <div class="p-3.5 rounded-xl bg-[#0c0d10] border border-[#252d3d] font-mono text-[11px] text-slate-300 leading-normal overflow-auto select-text shadow-inner">
            <pre class="m-0 whitespace-pre font-mono">{samResult.raw_output}</pre>
          </div>
        {/if}
      {:else}
        <div class="p-8 text-center text-slate-500">
          Inga alignments hittades i {alignRegion || 'denna BAM-fil'}.
        </div>
      {/if}

    <!-- TAB 3: RSNAP SNAPSHOT -->
    {:else if activeTab === 'rsnap'}
      <div class="p-3.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-3">
        <div class="flex items-center justify-between">
          <span class="font-bold text-xs text-amber-300 flex items-center gap-1.5">
            <Camera size={13} /> rsnap Snapshot Renderer
          </span>
          <button
            class="px-2.5 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs flex items-center gap-1"
            on:click={handleLaunchViewer}
          >
            <ExternalLink size={12} />
            <span>Öppna desktop viewer</span>
          </button>
        </div>

        <div class="flex items-center gap-2">
          <input
            type="text"
            bind:value={snapshotRegion}
            placeholder="t.ex. chr1:1000000-1005000"
            class="flex-1 bg-[#0e1015] border border-[#252d3d] rounded-lg px-2.5 py-1 text-xs font-mono text-white focus:outline-none"
            on:keydown={(e) => e.key === 'Enter' && handleGenerateSnapshot()}
          />
          <button
            class="px-3 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs shadow transition-colors disabled:opacity-50"
            disabled={isGeneratingSnapshot}
            on:click={handleGenerateSnapshot}
          >
            {isGeneratingSnapshot ? 'Genererar...' : 'Skapa bild'}
          </button>
        </div>

        {#if snapshotError}
          <div class="p-3 rounded-lg bg-red-950/40 border border-red-800 text-red-300 font-mono text-xs">
            {snapshotError}
          </div>
        {:else if snapshotB64}
          <div class="p-1 rounded-lg bg-black border border-[#252d3d] overflow-hidden">
            <img src="data:image/png;base64,{snapshotB64}" alt="snapshot" class="w-full object-contain rounded" />
          </div>
        {/if}
      </div>

    <!-- TAB 4: RS-QC -->
    {:else if activeTab === 'rsqc'}
      <div class="p-3.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-3">
        <div class="flex items-center justify-between">
          <span class="font-bold text-xs text-purple-300 flex items-center gap-1.5">
            <Activity size={13} /> rs-qc Alignment Diagnostic
          </span>
          {#if !qcReport}
            <button
              class="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white font-semibold text-xs shadow transition-colors"
              disabled={isRunningQc}
              on:click={handleRunQc}
            >
              {isRunningQc ? 'Kör diagnos...' : 'Kör rs-qc analys'}
            </button>
          {/if}
        </div>

        {#if qcError}
          <div class="p-3 rounded-lg bg-red-950/40 border border-red-800 text-red-300 font-mono text-xs">
            {qcError}
          </div>
        {:else if qcReport}
          <div class="p-3 rounded-xl bg-[#0c0d10] border border-[#252d3d] font-mono text-[11px] text-slate-300 overflow-auto max-h-96">
            <pre class="m-0 whitespace-pre-wrap">{qcReport}</pre>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
