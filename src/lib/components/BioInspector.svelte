<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    getBamHeader,
    getBamAlignments,
    generateRsnapSnapshot,
    launchRsnap,
    runRsQc,
  } from '../invoke';
  import { inspectorScroll } from '../stores/navigation';
  import type { FileItem, BamHeaderData, SamRecord } from '../types';
  import {
    Dna,
    Camera,
    Activity,
    ExternalLink,
    RefreshCw,
    Search,
    Copy,
    Check,
    ChevronDown,
    Filter,
    Layers,
    X,
    Table,
    FileText,
  } from 'lucide-svelte';

  export let item: FileItem;

  let bamHeader: BamHeaderData | null = null;
  let isLoadingHeader = false;
  let headerError = '';

  // Drawers
  let showContigDrawer = false;
  let showSnapshotDrawer = false;
  let showQcDrawer = false;
  let contigSearch = '';

  // SAM Stream Buffer
  let bufferedRecords: SamRecord[] = [];
  let isLoadingAlignments = false;
  let isLoadingMore = false;
  let alignmentsError = '';
  let alignRegion = '';
  let currentOffset = 0;
  let batchSize = 60;
  let hasMore = true;
  let alignViewMode: 'table' | 'raw' = 'table';
  let copiedRecordIndex: number | null = null;
  let copiedAll = false;

  let streamScrollEl: HTMLElement;
  let lastScrollPulse = 0;

  // Snapshot & QC State
  let snapshotRegion = 'chr1:1000000-1005000';
  let snapshotB64: string | null = null;
  let isGeneratingSnapshot = false;
  let snapshotError = '';
  let qcReport: string | null = null;
  let isRunningQc = false;
  let qcError = '';

  $: if (item) {
    initFile(item.path);
  }

  // Remote two-finger scroll listener from File Table
  $: if ($inspectorScroll.pulse && $inspectorScroll.pulse !== lastScrollPulse) {
    lastScrollPulse = $inspectorScroll.pulse;
    if (streamScrollEl) {
      streamScrollEl.scrollBy({ top: $inspectorScroll.deltaY, left: 0, behavior: 'auto' });
    }
  }

  async function initFile(path: string) {
    headerError = '';
    alignmentsError = '';
    showContigDrawer = false;
    showSnapshotDrawer = false;
    showQcDrawer = false;
    snapshotB64 = null;
    qcReport = null;
    currentOffset = 0;
    bufferedRecords = [];
    hasMore = true;

    await Promise.all([loadHeader(path), fetchAlignments(path, 0, true)]);
  }

  async function loadHeader(path: string) {
    isLoadingHeader = true;
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

  async function fetchAlignments(path: string, offset: number, isInitial = false) {
    if (isInitial) {
      isLoadingAlignments = true;
    } else {
      if (isLoadingMore || !hasMore) return;
      isLoadingMore = true;
    }

    try {
      const res = await getBamAlignments(path, alignRegion.trim() || undefined, batchSize, offset);
      if (isInitial) {
        bufferedRecords = res.records;
      } else {
        bufferedRecords = [...bufferedRecords, ...res.records];
      }
      currentOffset = offset + res.records.length;
      hasMore = res.has_more;
    } catch (e: any) {
      alignmentsError = String(e);
    } finally {
      isLoadingAlignments = false;
      isLoadingMore = false;
    }
  }

  function handleStreamScroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (!el || isLoadingMore || !hasMore || isLoadingAlignments) return;

    const threshold = 400;
    const distanceToBottom = el.scrollHeight - el.scrollTop - el.clientHeight;

    if (distanceToBottom < threshold) {
      fetchAlignments(item.path, currentOffset, false);
    }
  }

  function handleLocusSearch() {
    currentOffset = 0;
    bufferedRecords = [];
    hasMore = true;
    fetchAlignments(item.path, 0, true);
  }

  function handleClearLocus() {
    alignRegion = '';
    currentOffset = 0;
    bufferedRecords = [];
    hasMore = true;
    fetchAlignments(item.path, 0, true);
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

  async function copyAllBuffered() {
    if (bufferedRecords.length === 0) return;
    try {
      const allText = bufferedRecords.map((r) => r.raw_line).join('\n');
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
  <!-- 1. COMPACT GENOMICS HEADER STRIP (Height ~60px) -->
  <div class="px-3 py-2 bg-[#151922] border-b border-[#252d3d] flex flex-col gap-1 shrink-0">
    <div class="flex items-center justify-between gap-2">
      <div class="flex items-center gap-2 min-w-0">
        <div class="w-6 h-6 rounded bg-emerald-500/20 text-emerald-400 flex items-center justify-center shrink-0">
          <Dna size={14} />
        </div>
        <span class="font-bold text-xs text-white truncate max-w-[180px]" title={item.path}>{item.name}</span>
        
        {#if bamHeader?.detected_reference}
          <span class="px-1.5 py-0.2 rounded bg-emerald-500/20 text-emerald-300 font-mono text-[10px] font-bold border border-emerald-500/40 shrink-0">
            {bamHeader.detected_reference}
          </span>
        {/if}
        
        {#if bamHeader?.has_index}
          <span class="px-1 py-0.2 rounded bg-emerald-950 text-emerald-400 font-mono text-[9px] border border-emerald-800 shrink-0">
            {bamHeader.index_type || 'BAI'}
          </span>
        {/if}
      </div>

      <!-- Quick Action Buttons -->
      <div class="flex items-center gap-1 shrink-0">
        <button
          class="px-2 py-0.5 rounded bg-[#1a1f2c] hover:bg-white/10 text-slate-300 text-[10.5px] border border-[#252d3d] transition-colors {showContigDrawer ? 'bg-emerald-500/20 text-emerald-300 font-semibold' : ''}"
          on:click={() => (showContigDrawer = !showContigDrawer)}
          title="Visa alla contigs / kromosomer"
        >
          <span>Contigs ({bamHeader?.total_contigs || 0})</span>
        </button>

        <button
          class="px-2 py-0.5 rounded bg-[#1a1f2c] hover:bg-white/10 text-slate-300 text-[10.5px] border border-[#252d3d] transition-colors {showSnapshotDrawer ? 'bg-amber-500/20 text-amber-300 font-semibold' : ''}"
          on:click={() => (showSnapshotDrawer = !showSnapshotDrawer)}
          title="Skapa rsnap snapshot"
        >
          <Camera size={11} class="text-amber-400 inline mr-0.5" />
          <span>Snapshot</span>
        </button>

        <button
          class="px-2 py-0.5 rounded bg-[#1a1f2c] hover:bg-white/10 text-slate-300 text-[10.5px] border border-[#252d3d] transition-colors {showQcDrawer ? 'bg-purple-500/20 text-purple-300 font-semibold' : ''}"
          on:click={() => (showQcDrawer = !showQcDrawer)}
          title="Kör rs-qc kvalitetsrapport"
        >
          <Activity size={11} class="text-purple-400 inline mr-0.5" />
          <span>rs-qc</span>
        </button>

        <button
          class="px-2 py-0.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-[10.5px] shadow transition-colors flex items-center gap-1 ml-0.5"
          on:click={handleLaunchViewer}
          title="Öppna desktop viewer i rsnap"
        >
          <ExternalLink size={11} />
          <span>rsnap</span>
        </button>
      </div>
    </div>

    <!-- Metadata Details line -->
    {#if bamHeader}
      <div class="flex items-center gap-2 text-[10.5px] font-mono text-slate-400 truncate">
        {#each bamHeader.read_groups.slice(0, 2) as rg}
          <span class="text-slate-300 truncate"><strong class="text-emerald-400">SM:</strong> {rg.sample || rg.id}</span>
          <span class="text-slate-600">•</span>
        {/each}
        {#each bamHeader.programs.slice(0, 1) as pg}
          <span class="truncate"><strong class="text-amber-400">PG:</strong> {pg.name || pg.id}</span>
          <span class="text-slate-600">•</span>
        {/each}
        <span>{bamHeader.total_contigs} contigs ({bamHeader.formatted_genome_length})</span>
      </div>
    {/if}
  </div>

  <!-- 2. OPTIONAL DRAWERS (Contigs, Snapshot, rs-qc) -->
  {#if showContigDrawer && bamHeader}
    <div class="p-3 bg-[#151922] border-b border-[#252d3d] space-y-2 shrink-0 max-h-48 overflow-auto">
      <div class="flex items-center justify-between">
        <span class="font-bold text-xs text-slate-200">Kromosomer / Contigs ({bamHeader.total_contigs})</span>
        <div class="flex items-center gap-1">
          <input
            type="text"
            placeholder="Sök contig..."
            bind:value={contigSearch}
            class="bg-[#0e1015] border border-[#252d3d] rounded px-2 py-0.5 text-[11px] font-mono text-white focus:outline-none w-28"
          />
          <button on:click={() => (showContigDrawer = false)} class="p-1 text-slate-400 hover:text-white">
            <X size={12} />
          </button>
        </div>
      </div>
      <div class="flex items-center gap-1.5 flex-wrap">
        {#each filteredContigs.slice(0, 24) as c}
          <button
            class="px-2 py-0.5 rounded bg-[#0e1015] hover:bg-emerald-500/20 text-slate-300 hover:text-emerald-300 font-mono text-[10.5px] border border-[#222837] transition-colors"
            on:click={() => {
              alignRegion = `${c.name}:1-100000`;
              showContigDrawer = false;
              handleLocusSearch();
            }}
          >
            {c.name} ({c.formatted_length})
          </button>
        {/each}
      </div>
    </div>
  {/if}

  {#if showSnapshotDrawer}
    <div class="p-3 bg-[#151922] border-b border-[#252d3d] space-y-2 shrink-0">
      <div class="flex items-center justify-between">
        <span class="font-bold text-xs text-amber-300 flex items-center gap-1">
          <Camera size={12} /> rsnap Snapshot
        </span>
        <button on:click={() => (showSnapshotDrawer = false)} class="text-slate-400 hover:text-white">
          <X size={12} />
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
          {isGeneratingSnapshot ? 'Genererar...' : 'Skapa'}
        </button>
      </div>
      {#if snapshotB64}
        <div class="p-1 rounded bg-black border border-[#252d3d]">
          <img src="data:image/png;base64,{snapshotB64}" alt="snapshot" class="w-full object-contain rounded" />
        </div>
      {/if}
    </div>
  {/if}

  {#if showQcDrawer}
    <div class="p-3 bg-[#151922] border-b border-[#252d3d] space-y-2 shrink-0">
      <div class="flex items-center justify-between">
        <span class="font-bold text-xs text-purple-300 flex items-center gap-1">
          <Activity size={12} /> rs-qc Alignment Diagnostic
        </span>
        <button on:click={() => (showQcDrawer = false)} class="text-slate-400 hover:text-white">
          <X size={12} />
        </button>
      </div>
      {#if !qcReport}
        <button
          class="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white font-semibold text-xs shadow transition-colors"
          disabled={isRunningQc}
          on:click={handleRunQc}
        >
          {isRunningQc ? 'Kör analys...' : 'Starta rs-qc'}
        </button>
      {:else}
        <div class="p-2 rounded bg-[#0c0d10] border border-[#252d3d] max-h-40 overflow-auto font-mono text-[10.5px]">
          <pre class="m-0 whitespace-pre-wrap">{qcReport}</pre>
        </div>
      {/if}
    </div>
  {/if}

  <!-- 3. SAMTOOLS VIEW CONTROLS TOOLBAR -->
  <div class="px-3 py-1.5 bg-[#151922] border-b border-[#252d3d] flex items-center justify-between gap-2 shrink-0">
    <div class="flex items-center gap-1.5 flex-1 min-w-[200px]">
      <span class="font-bold text-[11px] text-white font-mono shrink-0">samtools view</span>
      <input
        type="text"
        placeholder="Locus (t.ex. chr1:10000-50000)..."
        bind:value={alignRegion}
        on:keydown={(e) => e.key === 'Enter' && handleLocusSearch()}
        class="bg-[#0e1015] border border-[#252d3d] rounded px-2 py-0.5 text-[11px] font-mono text-white focus:outline-none focus:border-emerald-400 flex-1 max-w-[220px]"
      />
      <button
        class="px-2 py-0.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-[10.5px] transition-colors"
        on:click={handleLocusSearch}
      >
        Visa
      </button>
      {#if alignRegion}
        <button
          class="p-0.5 text-slate-400 hover:text-white"
          on:click={handleClearLocus}
          title="Rensa filter och visa från start"
        >
          <X size={11} />
        </button>
      {/if}
    </div>

    <!-- Right: View Mode Toggle & Read Counter & Copy -->
    <div class="flex items-center gap-2 shrink-0">
      <span class="font-mono text-[10.5px] text-emerald-400">
        {bufferedRecords.length} reads
      </span>

      <div class="flex items-center bg-[#0e1015] rounded p-0.5 border border-[#252d3d]">
        <button
          class="px-1.5 py-0.2 rounded text-[10px] font-medium transition-colors {alignViewMode === 'table' ? 'bg-emerald-500/20 text-emerald-300 font-bold' : 'text-slate-400 hover:text-white'}"
          on:click={() => (alignViewMode = 'table')}
        >
          Tabell
        </button>
        <button
          class="px-1.5 py-0.2 rounded text-[10px] font-medium transition-colors {alignViewMode === 'raw' ? 'bg-emerald-500/20 text-emerald-300 font-bold' : 'text-slate-400 hover:text-white'}"
          on:click={() => (alignViewMode = 'raw')}
        >
          less -S
        </button>
      </div>

      <button
        class="p-1 rounded bg-[#0e1015] hover:bg-white/10 border border-[#252d3d] text-slate-300 hover:text-white transition-colors"
        on:click={copyAllBuffered}
        title="Kopiera alla laddade SAM-rader"
      >
        {#if copiedAll}
          <Check size={11} class="text-emerald-400" />
        {:else}
          <Copy size={11} />
        {/if}
      </button>
    </div>
  </div>

  <!-- 4. LIVE SAM READ STREAM TABLE -->
  <div
    bind:this={streamScrollEl}
    on:scroll={handleStreamScroll}
    class="flex-1 overflow-y-auto overflow-x-auto select-text font-mono text-xs"
  >
    {#if isLoadingAlignments && bufferedRecords.length === 0}
      <div class="p-8 text-center text-slate-400 flex items-center justify-center gap-2">
        <RefreshCw size={14} class="animate-spin text-emerald-400" />
        <span>Kör samtools view...</span>
      </div>
    {:else if alignmentsError && bufferedRecords.length === 0}
      <div class="p-4 m-3 rounded-xl bg-red-950/30 border border-red-800 text-red-400 space-y-1 font-mono text-xs">
        <span class="font-bold">Fel vid samtools view:</span>
        <p>{alignmentsError}</p>
      </div>
    {:else if bufferedRecords.length > 0}
      {#if alignViewMode === 'table'}
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
            {#each bufferedRecords as r, i}
              <tr class="border-b border-[#1b202c] hover:bg-emerald-500/5 transition-colors group">
                <td class="p-1.5 pl-3 text-slate-600 text-[10px]">{i + 1}</td>
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
      {:else}
        <!-- Raw Text Mode (less -S) -->
        <div class="p-3 font-mono text-[11px] text-slate-300 leading-tight overflow-x-auto">
          <pre class="m-0 whitespace-pre">{bufferedRecords.map((r) => r.raw_line).join('\n')}</pre>
        </div>
      {/if}

      <!-- Bottom Stream Loader -->
      <div class="py-3 text-center text-slate-500 font-mono text-[11px]">
        {#if isLoadingMore}
          <div class="flex items-center justify-center gap-2 text-emerald-400">
            <RefreshCw size={12} class="animate-spin" />
            <span>Strömmar fler reads ({currentOffset + 1}–{currentOffset + batchSize})...</span>
          </div>
        {:else if hasMore}
          <span class="opacity-60 text-[10px]">↓ Tvåfingerscrolla för att strömma fler reads</span>
        {:else}
          <span class="text-slate-600 text-[10px]">✓ Slut på filen</span>
        {/if}
      </div>
    {:else}
      <div class="p-8 text-center text-slate-500">
        Inga alignments hittades för {alignRegion || 'denna fil'}.
      </div>
    {/if}
  </div>
</div>
