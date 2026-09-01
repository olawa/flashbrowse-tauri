<script lang="ts">
  import {
    getBamHeader,
    getBamAlignments,
    generateRsnapSnapshot,
    launchRsnap,
    runRsQc,
    sendToIgv,
  } from '../invoke';
  import {
    addTracksToHub,
    isGenomicsHubOpen,
    selectedLocus,
    isRsnapServerRunning,
    isIgvConnected,
  } from '../stores/genomicsStore';
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
    Radio,
    Sparkles,
    ZoomIn,
    ZoomOut,
    Sliders,
  } from 'lucide-svelte';

  export let item: FileItem;

  // Tabs: 'rsnap' is the premier live interactive visualizer tab!
  let activeTab: 'rsnap' | 'header' | 'alignments' | 'rsqc' = 'rsnap';

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

  // rsnap Live Viewer state
  let snapshotRegion = 'chr7:55152000-55153000';
  let snapshotB64: string | null = null;
  let isGeneratingSnapshot = false;
  let snapshotError = '';
  let copiedSnapshot = false;

  const quickGenes: { name: string; locus: string; desc: string }[] = [
    { name: 'EGFR', locus: 'chr7:55152000-55153000', desc: 'Exon 19/20' },
    { name: 'TP53', locus: 'chr17:7668402-7687550', desc: 'Tumörsuppressor' },
    { name: 'BRAF', locus: 'chr7:140753336-140753337', desc: 'V600E' },
    { name: 'KRAS', locus: 'chr12:25245350-25245351', desc: 'G12/G13' },
    { name: 'BRCA1', locus: 'chr17:43044295-43125483', desc: 'DNA-repair' },
    { name: 'BRCA2', locus: 'chr13:32315474-32400266', desc: 'DNA-repair' },
    { name: 'MYC', locus: 'chr8:127735434-127742951', desc: 'Onkogen' },
  ];

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
    } else if (activeTab === 'rsnap') {
      handleGenerateSnapshot();
    }
  }

  $: if ($selectedLocus && $selectedLocus !== snapshotRegion) {
    snapshotRegion = $selectedLocus;
  }

  async function loadHeader(path: string) {
    isLoadingHeader = true;
    headerError = '';
    qcReport = null;
    try {
      bamHeader = await getBamHeader(path);
      if (bamHeader && bamHeader.contigs.length > 0) {
        if (!snapshotRegion || snapshotRegion === 'chr1:1000000-1005000') {
          const first = bamHeader.contigs[0];
          const end = Math.min(first.length, 50000);
          snapshotRegion = `${first.name}:10000-${end}`;
        }
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
    } else if (tab === 'rsnap' && !snapshotB64) {
      handleGenerateSnapshot();
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

  function copyRawRecord(line: string, index: number) {
    navigator.clipboard.writeText(line);
    copiedRecordIndex = index;
    setTimeout(() => {
      if (copiedRecordIndex === index) copiedRecordIndex = null;
    }, 1500);
  }

  function copyAllVisible() {
    if (!samResult) return;
    const allText = samResult.records.map((r) => r.raw_line).join('\n');
    navigator.clipboard.writeText(allText);
    copiedAll = true;
    setTimeout(() => (copiedAll = false), 2000);
  }

  async function handleLaunchViewer() {
    try {
      const genomeId = bamHeader?.detected_reference?.includes('38') ? 'hg38' :
                       bamHeader?.detected_reference?.includes('19') || bamHeader?.detected_reference?.includes('37') ? 'hg19' : undefined;
      await launchRsnap(
        [item.path],
        snapshotRegion.trim() || alignRegion.trim() || undefined,
        genomeId,
        bamHeader?.reference_matched_path,
        undefined,
        $isRsnapServerRunning,
      );
    } catch (e: any) {
      alert(`Kunde inte starta rsnap: ${e}`);
    }
  }

  function handleOpenHub() {
    addTracksToHub([item]);
    if (snapshotRegion.trim() || alignRegion.trim()) {
      selectedLocus.set(snapshotRegion.trim() || alignRegion.trim());
    }
    isGenomicsHubOpen.set(true);
  }

  let isSendingIgv = false;
  async function handleSendToIgvDirect() {
    isSendingIgv = true;
    try {
      const loc = snapshotRegion.trim() || alignRegion.trim() || undefined;
      const genomeId = bamHeader?.detected_reference?.includes('19') || bamHeader?.detected_reference?.includes('37') ? 'hg19' : 'hg38';
      const res = await sendToIgv([item.path], loc, genomeId, 60151);
      alert(res.message || 'Skickat till IGV!');
    } catch (err: any) {
      alert(`IGV fel: ${err}`);
    } finally {
      isSendingIgv = false;
    }
  }

  async function handleGenerateSnapshot() {
    if (!snapshotRegion.trim()) return;
    isGeneratingSnapshot = true;
    snapshotError = '';
    try {
      const genomeId = bamHeader?.detected_reference?.includes('38') ? 'hg38' :
                       bamHeader?.detected_reference?.includes('19') || bamHeader?.detected_reference?.includes('37') ? 'hg19' : undefined;
      const b64 = await generateRsnapSnapshot(
        item.path,
        snapshotRegion.trim(),
        genomeId,
        bamHeader?.reference_matched_path
      );
      snapshotB64 = b64;
    } catch (e: any) {
      snapshotError = String(e);
    } finally {
      isGeneratingSnapshot = false;
    }
  }

  function parseLocus(locusStr: string): { chr: string; start: number; end: number } | null {
    const clean = locusStr.trim().replace(/,/g, '');
    const match = clean.match(/^([^:]+):(\d+)[-_](\d+)$/);
    if (!match) return null;
    return {
      chr: match[1],
      start: parseInt(match[2], 10),
      end: parseInt(match[3], 10),
    };
  }

  function formatLocus(chr: string, start: number, end: number): string {
    const safeStart = Math.max(1, Math.round(start));
    const safeEnd = Math.max(safeStart + 50, Math.round(end));
    return `${chr}:${safeStart}-${safeEnd}`;
  }

  function panRegion(fraction: number) {
    const parsed = parseLocus(snapshotRegion);
    if (!parsed) return;
    const span = parsed.end - parsed.start;
    const shift = Math.round(span * fraction);
    snapshotRegion = formatLocus(parsed.chr, parsed.start + shift, parsed.end + shift);
    selectedLocus.set(snapshotRegion);
    handleGenerateSnapshot();
  }

  function zoomRegion(factor: number) {
    const parsed = parseLocus(snapshotRegion);
    if (!parsed) return;
    const mid = (parsed.start + parsed.end) / 2;
    const newSpan = (parsed.end - parsed.start) * factor;
    snapshotRegion = formatLocus(parsed.chr, mid - newSpan / 2, mid + newSpan / 2);
    selectedLocus.set(snapshotRegion);
    handleGenerateSnapshot();
  }

  function setSpan(spanBp: number) {
    const parsed = parseLocus(snapshotRegion);
    if (!parsed) return;
    const mid = (parsed.start + parsed.end) / 2;
    snapshotRegion = formatLocus(parsed.chr, mid - spanBp / 2, mid + spanBp / 2);
    selectedLocus.set(snapshotRegion);
    handleGenerateSnapshot();
  }

  function jumpToGene(locus: string) {
    snapshotRegion = locus;
    selectedLocus.set(snapshotRegion);
    handleGenerateSnapshot();
  }

  async function copySnapshotImage() {
    if (!snapshotB64) return;
    try {
      const res = await fetch(`data:image/png;base64,${snapshotB64}`);
      const blob = await res.blob();
      await navigator.clipboard.write([
        new ClipboardItem({ 'image/png': blob }),
      ]);
      copiedSnapshot = true;
      setTimeout(() => (copiedSnapshot = false), 2000);
    } catch (err) {
      console.warn('Copy image failed:', err);
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
    if (mapq >= 50) return 'text-emerald-400 font-bold';
    if (mapq >= 30) return 'text-sky-400';
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

      <!-- Action Buttons: rsnap, IGV, Hub -->
      <div class="flex items-center gap-1.5 shrink-0">
        <button
          class="flex items-center gap-1 px-2 py-1 rounded bg-blue-600/90 hover:bg-blue-500 text-white font-semibold text-xs shadow-md transition-colors"
          on:click={handleSendToIgvDirect}
          title="Skicka detta spår till IGV desktop (port 60151)"
          disabled={isSendingIgv}
        >
          <Radio size={12} />
          <span>IGV</span>
        </button>

        <button
          class="flex items-center gap-1 px-2 py-1 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs shadow-md transition-colors"
          on:click={handleLaunchViewer}
          title="Öppna desktop viewer i rsnap"
        >
          <ExternalLink size={12} />
          <span>rsnap</span>
        </button>

        <button
          class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#202738] hover:bg-[#2c364c] text-emerald-300 hover:text-white font-semibold text-xs border border-[#323e57] shadow transition-colors"
          on:click={handleOpenHub}
          title="Öppna Genomics Track Hub (hantera spår, server och IGV)"
        >
          <Sparkles size={12} class="text-amber-400" />
          <span>Hub</span>
        </button>
      </div>
    </div>

    <!-- Tab Bar -->
    <div class="flex items-center gap-1 border-b border-[#252d3d] pb-1 pt-0.5">
      <button
        class="flex items-center gap-1.5 px-3 py-1 rounded font-medium transition-colors text-xs {activeTab === 'rsnap' ? 'bg-amber-500/20 text-amber-300 font-bold border-b-2 border-amber-400' : 'text-slate-400 hover:text-white'}"
        on:click={() => handleTabChange('rsnap')}
      >
        <Camera size={13} />
        <span>🧬 rsnap Live Viewer</span>
      </button>

      <button
        class="flex items-center gap-1.5 px-3 py-1 rounded font-medium transition-colors text-xs {activeTab === 'header' ? 'bg-emerald-500/20 text-emerald-300 font-bold border-b-2 border-emerald-400' : 'text-slate-400 hover:text-white'}"
        on:click={() => handleTabChange('header')}
      >
        <Dna size={13} />
        <span>Header ({bamHeader?.total_contigs || 0})</span>
      </button>

      <button
        class="flex items-center gap-1.5 px-3 py-1 rounded font-medium transition-colors text-xs {activeTab === 'alignments' ? 'bg-emerald-500/20 text-emerald-300 font-bold border-b-2 border-emerald-400' : 'text-slate-400 hover:text-white'}"
        on:click={() => handleTabChange('alignments')}
      >
        <ListFilter size={13} />
        <span>Alignments (SAM)</span>
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

    <!-- TAB: RSNAP LIVE VIEWER -->
    {:else if activeTab === 'rsnap'}
      <div class="flex-1 flex flex-col space-y-2.5">
        <!-- Live Toolbar -->
        <div class="p-3 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2.5">
          <!-- Top Row: Locus input + Jump + Actions -->
          <div class="flex items-center gap-2">
            <div class="flex-1 flex items-center bg-[#0e1015] border border-[#252d3d] rounded-lg px-2.5 py-1 text-xs font-mono text-white focus-within:border-amber-400">
              <span class="text-slate-500 text-[10.5px] mr-1.5">Locus:</span>
              <input
                type="text"
                bind:value={snapshotRegion}
                placeholder="chr1:1000000-1005000 eller gen (t.ex. EGFR)"
                class="flex-1 bg-transparent text-white focus:outline-none font-mono text-xs"
                on:keydown={(e) => {
                  if (e.key === 'Enter') {
                    selectedLocus.set(snapshotRegion);
                    handleGenerateSnapshot();
                  }
                }}
              />
              {#if isGeneratingSnapshot}
                <RefreshCw size={13} class="animate-spin text-amber-400 shrink-0 ml-1" />
              {/if}
            </div>

            <button
              class="px-3 py-1.5 rounded-lg bg-amber-600 hover:bg-amber-500 text-white font-semibold text-xs shadow transition-colors disabled:opacity-50 flex items-center gap-1.5 shrink-0"
              disabled={isGeneratingSnapshot}
              on:click={() => {
                selectedLocus.set(snapshotRegion);
                handleGenerateSnapshot();
              }}
            >
              <Camera size={13} />
              <span>Uppdatera</span>
            </button>

            <button
              class="p-1.5 rounded-lg bg-[#1f2636] hover:bg-[#2b354a] text-slate-300 hover:text-white border border-[#2c374d] transition-colors"
              on:click={copySnapshotImage}
              title="Kopiera bild till urklipp"
              disabled={!snapshotB64}
            >
              {#if copiedSnapshot}
                <Check size={14} class="text-emerald-400" />
              {:else}
                <Copy size={14} />
              {/if}
            </button>

            <button
              class="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs transition-colors shrink-0 shadow"
              on:click={handleLaunchViewer}
              title="Öppna desktop viewer (egui fönster)"
            >
              <ExternalLink size={13} />
              <span class="hidden sm:inline">Externt fönster</span>
            </button>
          </div>

          <!-- Navigation Row: Pan & Zoom Controls -->
          <div class="flex items-center justify-between gap-2 flex-wrap pt-0.5 text-xs">
            <!-- Pan Buttons -->
            <div class="flex items-center gap-1">
              <span class="text-[10px] text-slate-400 font-semibold uppercase tracking-wider mr-1">Panorera:</span>
              <button
                class="px-2 py-0.5 rounded bg-[#1e2433] hover:bg-[#2a3449] text-slate-300 text-[11px] font-mono border border-[#2e394f]"
                on:click={() => panRegion(-0.5)}
                title="Panorera vänster 50%"
              >
                ◀◀ 50%
              </button>
              <button
                class="px-2 py-0.5 rounded bg-[#1e2433] hover:bg-[#2a3449] text-slate-300 text-[11px] font-mono border border-[#2e394f]"
                on:click={() => panRegion(-0.1)}
                title="Panorera vänster 10%"
              >
                ◀ 10%
              </button>
              <button
                class="px-2 py-0.5 rounded bg-[#1e2433] hover:bg-[#2a3449] text-slate-300 text-[11px] font-mono border border-[#2e394f]"
                on:click={() => panRegion(0.1)}
                title="Panorera höger 10%"
              >
                10% ▶
              </button>
              <button
                class="px-2 py-0.5 rounded bg-[#1e2433] hover:bg-[#2a3449] text-slate-300 text-[11px] font-mono border border-[#2e394f]"
                on:click={() => panRegion(0.5)}
                title="Panorera höger 50%"
              >
                50% ▶▶
              </button>
            </div>

            <!-- Zoom Buttons -->
            <div class="flex items-center gap-1">
              <span class="text-[10px] text-slate-400 font-semibold uppercase tracking-wider mr-1">Zooma:</span>
              <button
                class="px-2 py-0.5 rounded bg-[#1e2433] hover:bg-[#2a3449] text-amber-300 font-bold text-[11px] border border-[#2e394f] flex items-center gap-0.5"
                on:click={() => zoomRegion(0.5)}
                title="Zooma in 2x"
              >
                <ZoomIn size={12} /> 2x
              </button>
              <button
                class="px-2 py-0.5 rounded bg-[#1e2433] hover:bg-[#2a3449] text-amber-300 font-bold text-[11px] border border-[#2e394f] flex items-center gap-0.5"
                on:click={() => zoomRegion(2)}
                title="Zooma ut 2x"
              >
                <ZoomOut size={12} /> 2x
              </button>

              <div class="h-3.5 w-px bg-[#2e394f] mx-1"></div>

              <!-- Span Presets -->
              {#each [1000, 5000, 20000, 100000] as span}
                <button
                  class="px-1.5 py-0.5 rounded bg-[#161a24] hover:bg-[#242b3d] text-slate-400 hover:text-white text-[10px] font-mono border border-[#263044]"
                  on:click={() => setSpan(span)}
                >
                  {span >= 1000 ? `${span / 1000}kb` : `${span}bp`}
                </button>
              {/each}
            </div>
          </div>

          <!-- Quick Gene Chips -->
          <div class="flex items-center gap-1.5 flex-wrap pt-0.5 border-t border-[#202738]">
            <span class="text-[10px] text-slate-500 font-medium">Snabba gener:</span>
            {#each quickGenes as g}
              <button
                class="px-2 py-0.5 rounded-md bg-[#181d28] hover:bg-amber-950/40 text-slate-300 hover:text-amber-300 text-[10px] font-mono border border-[#273145] hover:border-amber-700 transition-colors"
                on:click={() => jumpToGene(g.locus)}
                title="{g.name} ({g.desc}): {g.locus}"
              >
                {g.name}
              </button>
            {/each}
          </div>
        </div>

        <!-- Live Snapshot Canvas Image Display -->
        {#if snapshotError}
          <div class="p-4 rounded-xl bg-red-950/40 border border-red-800 text-red-300 font-mono text-xs space-y-1">
            <div class="font-bold">Kunde inte generera rsnap vy:</div>
            <div>{snapshotError}</div>
          </div>
        {:else if isGeneratingSnapshot && !snapshotB64}
          <div class="p-16 flex flex-col items-center justify-center text-slate-400 gap-3 bg-[#0e1118] border border-[#252d3d] rounded-xl">
            <RefreshCw size={24} class="animate-spin text-amber-400" />
            <span class="font-mono text-xs">Renderar alignment-vy med rsnap...</span>
          </div>
        {:else if snapshotB64}
          <div class="relative rounded-xl bg-black border border-[#252d3d] overflow-hidden shadow-2xl group select-none">
            <img
              src="data:image/png;base64,{snapshotB64}"
              alt="rsnap alignment snapshot"
              class="w-full object-contain rounded transition-transform"
            />

            <!-- Top Floating Badge -->
            <div class="absolute top-2 left-2 px-2 py-1 rounded bg-black/75 backdrop-blur-md border border-white/10 text-white font-mono text-[10px] flex items-center gap-2 shadow-lg">
              <span class="font-bold text-amber-300">{snapshotRegion}</span>
              {#if bamHeader?.detected_reference}
                <span class="text-slate-400">({bamHeader.detected_reference})</span>
              {/if}
            </div>

            <!-- Quick on-canvas pan controls -->
            <div class="absolute inset-y-0 left-0 w-12 flex items-center justify-center opacity-0 group-hover:opacity-80 transition-opacity bg-gradient-to-r from-black/60 to-transparent">
              <button
                class="p-1.5 rounded-full bg-black/80 text-white hover:scale-110 transition-transform shadow-lg border border-white/20"
                on:click={() => panRegion(-0.25)}
                title="Panorera vänster"
              >
                <ChevronLeft size={16} />
              </button>
            </div>

            <div class="absolute inset-y-0 right-0 w-12 flex items-center justify-center opacity-0 group-hover:opacity-80 transition-opacity bg-gradient-to-l from-black/60 to-transparent">
              <button
                class="p-1.5 rounded-full bg-black/80 text-white hover:scale-110 transition-transform shadow-lg border border-white/20"
                on:click={() => panRegion(0.25)}
                title="Panorera höger"
              >
                <ChevronRight size={16} />
              </button>
            </div>
          </div>
        {:else}
          <div class="p-12 flex flex-col items-center justify-center text-slate-500 gap-2 bg-[#0e1118] border border-[#252d3d] rounded-xl text-center">
            <Camera size={28} class="text-slate-600 mb-1" />
            <span class="text-xs">Klicka "Uppdatera" eller välj en gen ovan för att ladda rsnap-vyn.</span>
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
