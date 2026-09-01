<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    stagedTracks,
    selectedLocus,
    selectedGenome,
    isRsnapServerRunning,
    rsnapServerPid,
    isIgvConnected,
    isGenomicsHubOpen,
    removeTrackFromHub,
    clearTracksInHub,
    pollGenomicsStatuses,
    configuredGenomes,
    loadGenomes,
    genomeMismatchInfo,
  } from '../stores/genomicsStore';
  import {
    launchRsnap,
    startRsnapServer,
    stopRsnapServer,
    sendToIgv,
    saveConfiguredGenome,
  } from '../invoke';
  import {
    Dna,
    ExternalLink,
    Server,
    Radio,
    Play,
    Square,
    CheckSquare,
    Trash2,
    X,
    Sparkles,
    Check,
    AlertCircle,
    AlertTriangle,
    Loader2,
    Layers,
    Activity,
    Settings,
    FolderGit2,
    FileText,
  } from 'lucide-svelte';
  import type { GenomeRefInfo } from '../types';

  let connectToServer = false;
  let serverAddress = 'localhost:5555';
  let isLaunchingRsnap = false;
  let isSendingToIgv = false;
  let igvStatusMsg = '';
  let pollInterval: any = null;
  let isConfigModalOpen = false;
  let editingGenome: GenomeRefInfo | null = null;

  const quickGenes: { name: string; locus: string; desc: string }[] = [
    { name: 'EGFR', locus: 'chr7:55152000-55153000', desc: 'Exon 19/20' },
    { name: 'TP53', locus: 'chr17:7668402-7687550', desc: 'Tumörsuppressor' },
    { name: 'BRAF', locus: 'chr7:140753336-140753337', desc: 'V600E' },
    { name: 'KRAS', locus: 'chr12:25245350-25245351', desc: 'G12/G13' },
    { name: 'BRCA1', locus: 'chr17:43044295-43125483', desc: 'DNA-repair' },
    { name: 'BRCA2', locus: 'chr13:32315474-32400266', desc: 'DNA-repair' },
    { name: 'MYC', locus: 'chr8:127735434-127742951', desc: 'Onkogen' },
  ];

  $: if ($isGenomicsHubOpen) {
    loadGenomes();
    pollGenomicsStatuses();
    if (!pollInterval) {
      pollInterval = setInterval(pollGenomicsStatuses, 3000);
    }
  } else {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });

  $: activeGenomeInfo = $configuredGenomes.find((g) => g.id === $selectedGenome) || $configuredGenomes[0];

  async function handleLaunchRsnap() {
    if ($stagedTracks.length === 0) {
      alert('Lägg till minst en BAM/CRAM eller VCF-fil att visualisera.');
      return;
    }
    isLaunchingRsnap = true;
    try {
      const paths = $stagedTracks.map((t) => t.path);
      await launchRsnap(
        paths,
        $selectedLocus.trim() || undefined,
        $selectedGenome,
        activeGenomeInfo?.fasta_path || undefined,
        activeGenomeInfo?.gtf_path || undefined,
        connectToServer || $isRsnapServerRunning,
        connectToServer ? serverAddress : undefined,
      );
    } catch (err: any) {
      alert(`Kunde inte starta rsnap viewer: ${err}`);
    } finally {
      isLaunchingRsnap = false;
    }
  }

  async function handleToggleRsnapServer() {
    try {
      if ($isRsnapServerRunning) {
        await stopRsnapServer();
        isRsnapServerRunning.set(false);
        rsnapServerPid.set(null);
      } else {
        const info = await startRsnapServer(undefined, $selectedGenome);
        isRsnapServerRunning.set(info.is_running);
        rsnapServerPid.set(info.pid || null);
        connectToServer = true;
      }
    } catch (err: any) {
      alert(`rsnap server fel: ${err}`);
    }
  }

  async function handleSendToIgv() {
    if ($stagedTracks.length === 0) {
      alert('Lägg till minst en fil att skicka till IGV.');
      return;
    }
    isSendingToIgv = true;
    igvStatusMsg = '';
    try {
      const paths = $stagedTracks.map((t) => t.path);
      const res = await sendToIgv(
        paths,
        $selectedLocus.trim() || undefined,
        $selectedGenome,
        60151,
      );
      igvStatusMsg = res.message || 'Spår skickade till IGV!';
      setTimeout(() => {
        igvStatusMsg = '';
      }, 4000);
    } catch (err: any) {
      alert(`IGV fel: ${err}`);
    } finally {
      isSendingToIgv = false;
    }
  }

  function openEditGenome(genome: GenomeRefInfo) {
    editingGenome = { ...genome };
    isConfigModalOpen = true;
  }

  async function handleSaveGenome() {
    if (!editingGenome) return;
    try {
      const updated = await saveConfiguredGenome(editingGenome);
      configuredGenomes.set(updated);
      isConfigModalOpen = false;
      editingGenome = null;
    } catch (err: any) {
      alert(`Kunde inte spara genomkonfiguration: ${err}`);
    }
  }
</script>

{#if $isGenomicsHubOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm select-none"
    on:click={() => isGenomicsHubOpen.set(false)}
  >
    <div
      class="w-[740px] max-h-[90vh] flex flex-col bg-[#11141b] border border-[#252d3d] rounded-2xl shadow-2xl overflow-hidden font-sans text-slate-200"
      on:click|stopPropagation
    >
      <!-- Header -->
      <div class="px-5 py-3.5 bg-[#171c26] border-b border-[#252d3d] flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-lg bg-emerald-950 border border-emerald-800/80 flex items-center justify-center text-emerald-400">
            <Dna size={18} />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <span class="font-bold text-sm text-white">Genomics Viewer & Server Hub</span>
              <span class="px-2 py-0.5 rounded-full bg-emerald-950 text-emerald-300 font-mono text-[10px] font-bold border border-emerald-800/60">
                {$stagedTracks.length} {$stagedTracks.length === 1 ? 'spår' : 'spår'}
              </span>
            </div>
            <span class="text-[11px] text-slate-400">Styr rsnap (Viewer & Server) och IGV Desktop direkt från Flashbrowse</span>
          </div>
        </div>

        <button
          class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-[#222938] transition-colors"
          on:click={() => isGenomicsHubOpen.set(false)}
        >
          <X size={16} />
        </button>
      </div>

      <!-- Main Body Scroll Area -->
      <div class="flex-1 overflow-y-auto p-5 space-y-4">
        <!-- Genom Mismatch Varning -->
        {#if $genomeMismatchInfo.hasMismatch}
          <div class="p-3 rounded-xl bg-amber-950/60 border border-amber-600/80 text-amber-200 flex items-start gap-2.5 shadow-md">
            <AlertTriangle size={18} class="text-amber-400 shrink-0 mt-0.5" />
            <div class="text-xs space-y-0.5">
              <div class="font-bold text-amber-300">Varning: Flera olika genomversioner upptäckta bland dina spår!</div>
              <div class="text-[11.5px] text-amber-200/90 leading-relaxed">
                Dina valda spår innehåller både <strong>{$genomeMismatchInfo.builds.map((b) => b.toUpperCase()).join(' och ')}</strong>. Att visualisera spår från olika koordinatsystem i samma vy ger felaktiga resultat.
              </div>
            </div>
          </div>
        {/if}

        <!-- 1. Staged Tracks List -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold uppercase tracking-wider text-slate-400 flex items-center gap-1.5">
              <Layers size={13} class="text-emerald-400" />
              Aktiva Spår ({$stagedTracks.length})
            </span>
            {#if $stagedTracks.length > 0}
              <button
                class="text-[11px] text-red-400 hover:text-red-300 flex items-center gap-1 transition-colors"
                on:click={clearTracksInHub}
              >
                <Trash2 size={11} /> Töm alla
              </button>
            {/if}
          </div>

          {#if $stagedTracks.length === 0}
            <div class="p-6 rounded-xl border border-dashed border-[#2b354c] bg-[#141822] text-center text-xs text-slate-400 space-y-1">
              <p class="font-medium text-slate-300">Inga spår är laddade ännu</p>
              <p class="text-[11px] text-slate-500">
                Markera BAM, CRAM, VCF eller BED-filer i Flashbrowse och klicka "Skicka till rsnap / IGV", eller dra filer hit.
              </p>
            </div>
          {:else}
            <div class="max-h-48 overflow-y-auto rounded-xl border border-[#252d3d] bg-[#0d1017] divide-y divide-[#1d2331]">
              {#each $stagedTracks as track}
                <div class="px-3 py-2 flex items-center justify-between gap-3 text-xs">
                  <div class="flex items-center gap-2.5 min-w-0 flex-1">
                    {#if track.kind === 'bam'}
                      <span class="px-1.5 py-0.5 rounded bg-emerald-950 text-emerald-300 font-mono text-[9.5px] font-bold border border-emerald-800 shrink-0">BAM</span>
                    {:else if track.kind === 'vcf'}
                      <span class="px-1.5 py-0.5 rounded bg-purple-950 text-purple-300 font-mono text-[9.5px] font-bold border border-purple-800 shrink-0">VCF</span>
                    {:else if track.kind === 'bed'}
                      <span class="px-1.5 py-0.5 rounded bg-cyan-950 text-cyan-300 font-mono text-[9.5px] font-bold border border-cyan-800 shrink-0">BED</span>
                    {:else}
                      <span class="px-1.5 py-0.5 rounded bg-slate-800 text-slate-300 font-mono text-[9.5px] shrink-0">FIL</span>
                    {/if}

                    <div class="flex flex-col min-w-0 flex-1">
                      <div class="flex items-center gap-2">
                        <span class="font-medium text-white truncate">{track.name}</span>
                        {#if track.detected_build && track.detected_build !== 'unknown'}
                          <span class="px-1.5 py-0.2 rounded font-mono text-[9px] font-bold border {track.detected_build === 'hg38' ? 'bg-emerald-950 text-emerald-300 border-emerald-800' : 'bg-blue-950 text-blue-300 border-blue-800'}">
                            {track.detected_build.toUpperCase()}
                          </span>
                        {/if}
                      </div>
                      <span class="font-mono text-[10px] text-slate-500 truncate" title={track.path}>{track.path}</span>
                    </div>
                  </div>

                  <div class="flex items-center gap-2 shrink-0">
                    {#if track.formatted_size}
                      <span class="font-mono text-[10px] text-slate-400">{track.formatted_size}</span>
                    {/if}
                    <button
                      class="p-1 text-slate-500 hover:text-red-400 rounded transition-colors"
                      on:click={() => removeTrackFromHub(track.path)}
                      title="Ta bort från spårlista"
                    >
                      <X size={12} />
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- 2. Genome & Reference Configuration -->
        <div class="p-3.5 rounded-xl bg-[#151922] border border-[#252d3d] space-y-2.5">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span class="text-xs font-bold text-slate-300 flex items-center gap-1.5">
                <Dna size={14} class="text-emerald-400" /> Referensgenom & Gener:
              </span>
            </div>

            <div class="flex items-center gap-2">
              <select
                bind:value={$selectedGenome}
                class="bg-[#1e2433] text-xs text-white rounded px-2.5 py-1 border border-[#303a4e] focus:border-emerald-500 focus:outline-none font-semibold font-mono"
              >
                {#each $configuredGenomes as g}
                  <option value={g.id}>{g.name} {g.is_available ? '✓' : '⚠️'}</option>
                {/each}
              </select>

              {#if activeGenomeInfo}
                <button
                  class="p-1.5 rounded-lg bg-[#1e2433] hover:bg-[#283247] text-slate-300 hover:text-white border border-[#303a4e] transition-colors"
                  on:click={() => openEditGenome(activeGenomeInfo)}
                  title="Konfigurera sökvägar till FASTA och GTF"
                >
                  <Settings size={13} />
                </button>
              {/if}
            </div>
          </div>

          <!-- Active Genome Path Details -->
          {#if activeGenomeInfo}
            <div class="p-2.5 rounded-lg bg-[#0c0e14] border border-[#202736] space-y-1.5 text-[11px] font-mono">
              <div class="flex items-center justify-between gap-2">
                <span class="text-slate-400 shrink-0">FASTA (.fa/.fasta):</span>
                <span class="text-emerald-300 truncate text-right flex-1" title={activeGenomeInfo.fasta_path || 'Ingen'}>
                  {activeGenomeInfo.fasta_path || '⚠️ Inte konfigurerad'}
                </span>
                {#if activeGenomeInfo.fai_path}
                  <span class="px-1.5 py-0.2 rounded bg-emerald-950 text-emerald-400 text-[9px] font-bold border border-emerald-800 shrink-0">FAI ✓</span>
                {/if}
              </div>

              <div class="flex items-center justify-between gap-2">
                <span class="text-slate-400 shrink-0">Gener (GTF/GFF/BED):</span>
                <span class="text-purple-300 truncate text-right flex-1" title={activeGenomeInfo.gtf_path || 'Ingen'}>
                  {activeGenomeInfo.gtf_path || '⚠️ Ingen gen-annotation'}
                </span>
                {#if activeGenomeInfo.gtf_path}
                  <span class="px-1.5 py-0.2 rounded bg-purple-950 text-purple-400 text-[9px] font-bold border border-purple-800 shrink-0">GTF ✓</span>
                {/if}
              </div>
            </div>
          {/if}

          <!-- Locus input -->
          <div class="space-y-1.5 pt-1">
            <div class="flex items-center justify-between">
              <span class="text-xs font-bold text-slate-300">Locus / Genkoordinater:</span>
              <span class="text-[10.5px] text-slate-500">t.ex. chr7:55152000-55153000 eller EGFR</span>
            </div>

            <input
              type="text"
              bind:value={$selectedLocus}
              placeholder="t.ex. chr7:55152000-55153000 eller EGFR"
              class="w-full bg-[#0c0e14] text-xs text-white px-3 py-2 rounded-lg border border-[#252d3d] focus:border-emerald-400 focus:outline-none font-mono tracking-wide"
            />

            <!-- Quick Gene Chips -->
            <div class="flex items-center gap-1.5 flex-wrap pt-0.5">
              <span class="text-[10.5px] text-slate-500 font-medium">Snabba gener:</span>
              {#each quickGenes as g}
                <button
                  class="px-2 py-0.5 rounded-md bg-[#1d2331] hover:bg-emerald-900/40 text-slate-300 hover:text-emerald-300 text-[10.5px] font-mono border border-[#2e374d] hover:border-emerald-700 transition-colors"
                  on:click={() => selectedLocus.set(g.locus)}
                  title="{g.name} ({g.desc}): {g.locus}"
                >
                  {g.name}
                </button>
              {/each}
            </div>
          </div>
        </div>

        <!-- 3. Two Launch Engines: rsnap & IGV -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3.5">
          <!-- rsnap Panel -->
          <div class="p-3.5 rounded-xl bg-[#141822] border border-[#252d3d] flex flex-col justify-between space-y-3">
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <span class="font-bold text-xs text-emerald-400 flex items-center gap-1.5">
                    <Activity size={14} /> rsnap Viewer & Server
                  </span>
                </div>
                <!-- Server Status Pill -->
                <div class="flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold {$isRsnapServerRunning ? 'bg-emerald-950 text-emerald-300 border border-emerald-800' : 'bg-[#1e2433] text-slate-400 border border-[#2c3548]'}">
                  <div class="w-1.5 h-1.5 rounded-full {$isRsnapServerRunning ? 'bg-emerald-400 animate-pulse' : 'bg-slate-500'}"></div>
                  {$isRsnapServerRunning ? `Server Port 5555` : 'Server Stoppad'}
                </div>
              </div>

              <p class="text-[11px] text-slate-400 leading-relaxed">
                Supersnabb egui-native alignment viewer och rendering via bakgrundsserver.
              </p>

              <!-- Server Controller -->
              <div class="pt-1 flex items-center justify-between">
                <button
                  class="px-2.5 py-1 rounded bg-[#1e2433] hover:bg-[#273043] text-slate-300 hover:text-white text-[11px] font-medium border border-[#303a4e] flex items-center gap-1.5 transition-colors"
                  on:click={handleToggleRsnapServer}
                >
                  <Server size={12} class={$isRsnapServerRunning ? 'text-amber-400' : 'text-emerald-400'} />
                  <span>{$isRsnapServerRunning ? 'Stoppa rsnap server' : 'Starta rsnap server'}</span>
                </button>

                {#if $isRsnapServerRunning}
                  <span class="text-[10px] font-mono text-emerald-400">PID: {$rsnapServerPid || 'OK'}</span>
                {/if}
              </div>
            </div>

            <!-- Launch Viewer Button -->
            <button
              class="w-full py-2 px-3 rounded-lg bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 text-white font-bold text-xs shadow-lg flex items-center justify-center gap-2 transition-colors disabled:opacity-50"
              disabled={isLaunchingRsnap || $stagedTracks.length === 0}
              on:click={handleLaunchRsnap}
            >
              {#if isLaunchingRsnap}
                <Loader2 size={14} class="animate-spin" />
                <span>Startar rsnap...</span>
              {:else}
                <ExternalLink size={14} />
                <span>Öppna i rsnap Desktop Viewer</span>
              {/if}
            </button>
          </div>

          <!-- IGV Desktop Panel -->
          <div class="p-3.5 rounded-xl bg-[#141822] border border-[#252d3d] flex flex-col justify-between space-y-3">
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <span class="font-bold text-xs text-blue-400 flex items-center gap-1.5">
                  <Radio size={14} /> IGV Desktop Bridge
                </span>
                <!-- IGV Status Pill -->
                <div class="flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold {$isIgvConnected ? 'bg-blue-950 text-blue-300 border border-blue-800' : 'bg-[#1e2433] text-slate-400 border border-[#2c3548]'}">
                  <div class="w-1.5 h-1.5 rounded-full {$isIgvConnected ? 'bg-blue-400' : 'bg-slate-500'}"></div>
                  {$isIgvConnected ? 'IGV Ansluten (60151)' : 'IGV Ej Aktiv'}
                </div>
              </div>

              <p class="text-[11px] text-slate-400 leading-relaxed">
                Skickar spår och navigerar automatiskt i ditt öppna IGV Desktop-fönster via REST API port 60151.
              </p>

              {#if igvStatusMsg}
                <div class="p-2 rounded bg-blue-950/60 border border-blue-800 text-blue-300 text-[11px] font-mono flex items-center gap-1.5">
                  <Check size={12} class="text-blue-400 shrink-0" />
                  <span>{igvStatusMsg}</span>
                </div>
              {/if}
            </div>

            <!-- Send to IGV Button -->
            <button
              class="w-full py-2 px-3 rounded-lg bg-blue-600 hover:bg-blue-500 active:bg-blue-700 text-white font-bold text-xs shadow-lg flex items-center justify-center gap-2 transition-colors disabled:opacity-50"
              disabled={isSendingToIgv || $stagedTracks.length === 0}
              on:click={handleSendToIgv}
            >
              {#if isSendingToIgv}
                <Loader2 size={14} class="animate-spin" />
                <span>Skickar till IGV...</span>
              {:else}
                <Radio size={14} />
                <span>Skicka till IGV Desktop</span>
              {/if}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Genome Configuration Sub-Modal -->
{#if isConfigModalOpen && editingGenome}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm select-none">
    <div class="w-[600px] bg-[#171c26] border border-[#2b354c] rounded-2xl shadow-2xl p-5 space-y-4 text-slate-200">
      <div class="flex items-center justify-between border-b border-[#252d3d] pb-3">
        <div class="flex items-center gap-2">
          <Settings size={18} class="text-emerald-400" />
          <span class="font-bold text-sm text-white">Konfigurera {editingGenome.name}</span>
        </div>
        <button
          class="p-1 rounded-lg text-slate-400 hover:text-white"
          on:click={() => (isConfigModalOpen = false)}
        >
          <X size={16} />
        </button>
      </div>

      <div class="space-y-3 text-xs">
        <div class="space-y-1">
          <label class="font-semibold text-slate-300">FASTA referenssekvens (.fa / .fasta):</label>
          <input
            type="text"
            bind:value={editingGenome.fasta_path}
            placeholder="/Users/.../GRCh38.fasta"
            class="w-full bg-[#0d1017] border border-[#252d3d] rounded-lg px-3 py-2 text-white font-mono text-[11px] focus:outline-none focus:border-emerald-400"
          />
          <span class="text-[10px] text-slate-500 block">Kräver en matchande .fai indexfil i samma katalog</span>
        </div>

        <div class="space-y-1">
          <label class="font-semibold text-slate-300">FAI indexfil (.fai):</label>
          <input
            type="text"
            bind:value={editingGenome.fai_path}
            placeholder="/Users/.../GRCh38.fasta.fai"
            class="w-full bg-[#0d1017] border border-[#252d3d] rounded-lg px-3 py-2 text-white font-mono text-[11px] focus:outline-none focus:border-emerald-400"
          />
        </div>

        <div class="space-y-1">
          <label class="font-semibold text-slate-300">Gen-annotationer (GTF / GFF / BED):</label>
          <input
            type="text"
            bind:value={editingGenome.gtf_path}
            placeholder="/Users/.../gencode.v46.annotation.sorted.gtf.gz"
            class="w-full bg-[#0d1017] border border-[#252d3d] rounded-lg px-3 py-2 text-white font-mono text-[11px] focus:outline-none focus:border-emerald-400"
          />
          <span class="text-[10px] text-slate-500 block">Om filen slutar på .gtf.gz krävs en .tbi indexfil (skapas med tabix -p gff)</span>
        </div>
      </div>

      <div class="flex items-center justify-end gap-2 pt-2 border-t border-[#252d3d]">
        <button
          class="px-3 py-1.5 rounded-lg border border-[#303a4e] text-xs hover:bg-[#202738]"
          on:click={() => (isConfigModalOpen = false)}
        >
          Avbryt
        </button>
        <button
          class="px-4 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs shadow"
          on:click={handleSaveGenome}
        >
          Spara ändringar
        </button>
      </div>
    </div>
  </div>
{/if}
