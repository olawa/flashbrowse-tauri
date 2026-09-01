<script lang="ts">
  import { stashItems, isStashOpen, removeFromStash, clearStash } from '../stores/stash';
  import { launchRsnap, runRsQc, sendToIgv } from '../invoke';
  import { addTracksToHub, isGenomicsHubOpen } from '../stores/genomicsStore';
  import { activePaneId, leftPane, rightPane } from '../stores/navigation';
  import {
    Layers,
    X,
    Trash2,
    Copy,
    Check,
    ExternalLink,
    ChevronDown,
    ChevronUp,
    FileText,
    Folder,
    FileCode,
    Dna,
    Activity,
    Radio,
    Sparkles,
  } from 'lucide-svelte';
  import type { FileItem } from '../types';

  let copied = false;
  let isRunningQc = false;
  let qcReportModal = '';

  $: hasBams = $stashItems.some(
    (i) => i.extension.toLowerCase() === 'bam' || i.name.endsWith('.bam') || i.name.endsWith('.cram')
  );

  function getFileIcon(item: FileItem) {
    if (item.is_dir) return Folder;
    const ext = item.extension.toLowerCase();
    if (['bam', 'cram', 'sam', 'vcf', 'bcf', 'bed'].includes(ext)) return Dna;
    if (['rs', 'py', 'js', 'ts', 'c', 'sh'].includes(ext)) return FileCode;
    return FileText;
  }

  async function openAllInRsnap() {
    const paths = $stashItems.map((i) => i.path);
    if (paths.length === 0) return;
    try {
      await launchRsnap(paths);
    } catch (e: any) {
      alert(`Kunde inte starta rsnap: ${e}`);
    }
  }

  function handleOpenGenomicsHub() {
    if ($stashItems.length === 0) return;
    addTracksToHub($stashItems);
    isGenomicsHubOpen.set(true);
  }

  async function sendStashToIgv() {
    const paths = $stashItems.map((i) => i.path);
    if (paths.length === 0) return;
    try {
      const res = await sendToIgv(paths);
      alert(res.message || 'Skickat till IGV!');
    } catch (e: any) {
      alert(`IGV fel: ${e}`);
    }
  }

  async function runQcOnStash() {
    const bams = $stashItems.filter(
      (i) => i.extension.toLowerCase() === 'bam' || i.name.endsWith('.bam') || i.name.endsWith('.cram')
    );
    if (bams.length === 0) {
      alert('Inga BAM/CRAM-filer hittades i samlingsfacket.');
      return;
    }
    isRunningQc = true;
    qcReportModal = '';
    try {
      const reports: string[] = [];
      for (const bam of bams) {
        const report = await runRsQc(bam.path);
        reports.push(`======================================================================\n📊 BAM: ${bam.name} (${bam.path})\n======================================================================\n${report}`);
      }
      qcReportModal = reports.join('\n\n');
    } catch (e: any) {
      alert(`rs-qc fel: ${e}`);
    } finally {
      isRunningQc = false;
    }
  }

  async function copyAllPaths() {
    const text = $stashItems.map((i) => i.path).join('\n');
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (err) {
      console.warn('Clipboard write failed:', err);
    }
  }
</script>

{#if $isStashOpen && $stashItems.length > 0}
  <div class="border-t border-[var(--border)] bg-[#12151b] text-xs select-none shadow-2xl flex flex-col max-h-48 transition-all">
    <!-- Stash Header Bar -->
    <div class="flex items-center justify-between px-3 py-1.5 bg-[#171b22] border-b border-[#262d3d]">
      <div class="flex items-center gap-2">
        <Layers size={13} class="text-[var(--accent)]" />
        <span class="font-bold text-slate-200">Samlingsfack (Stash Shelf)</span>
        <span class="px-1.5 py-0.2 rounded-full bg-[var(--accent)]/20 text-[var(--accent)] font-mono text-[10px] font-bold">
          {$stashItems.length}
        </span>
      </div>

      <div class="flex items-center gap-1.5">
        {#if hasBams}
          <button
            class="flex items-center gap-1 px-2 py-0.5 rounded bg-emerald-500/20 hover:bg-emerald-500 text-emerald-300 hover:text-white border border-emerald-500/40 text-[11px] font-semibold transition-colors"
            on:click={openAllInRsnap}
            title="Öppna alla stashed filer i rsnap viewer"
          >
            <Dna size={11} />
            <span>rsnap</span>
          </button>

          <button
            class="flex items-center gap-1 px-2 py-0.5 rounded bg-blue-500/20 hover:bg-blue-500 text-blue-300 hover:text-white border border-blue-500/40 text-[11px] font-semibold transition-colors"
            on:click={sendStashToIgv}
            title="Skicka stashed spår till IGV desktop"
          >
            <Radio size={11} />
            <span>IGV</span>
          </button>

          <button
            class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#2a3449] hover:bg-[#394764] text-emerald-300 hover:text-white border border-[#3e4f71] text-[11px] font-semibold transition-colors"
            on:click={handleOpenGenomicsHub}
            title="Öppna Genomics Track Hub med alla stashed spår"
          >
            <Sparkles size={11} class="text-amber-400" />
            <span>Hub</span>
          </button>

          <button
            class="flex items-center gap-1 px-2 py-0.5 rounded bg-purple-500/20 hover:bg-purple-500 text-purple-300 hover:text-white border border-purple-500/40 text-[11px] font-semibold transition-colors"
            on:click={runQcOnStash}
            disabled={isRunningQc}
            title="Kör rs-qc align på BAM-fil"
          >
            <Activity size={11} class={isRunningQc ? 'animate-spin' : ''} />
            <span>{isRunningQc ? 'Kör rs-qc...' : 'rs-qc'}</span>
          </button>
        {/if}

        <button
          class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#202530] hover:bg-[#2c3342] text-slate-300 border border-[#2f384a] text-[11px] transition-colors"
          on:click={copyAllPaths}
          title="Kopiera alla sökvägar rad för rad"
        >
          {#if copied}
            <Check size={11} class="text-green-400" />
            <span>Kopierat!</span>
          {:else}
            <Copy size={11} />
            <span>Kopiera</span>
          {/if}
        </button>

        <button
          class="flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-red-500/20 text-slate-400 hover:text-red-400 text-[11px] transition-colors"
          on:click={clearStash}
          title="Rensa hela stashen"
        >
          <Trash2 size={11} />
          <span>Rensa</span>
        </button>

        <button
          class="p-1 rounded hover:bg-white/10 text-slate-400 hover:text-white"
          on:click={() => isStashOpen.set(false)}
          title="Minimera stashen"
        >
          <ChevronDown size={13} />
        </button>
      </div>
    </div>

    <!-- Stash Items Horizontal Carousel / Grid -->
    <div class="p-2 overflow-x-auto flex items-center gap-2 bg-[#0d0e12]">
      {#each $stashItems as item (item.path)}
        <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-[#191d26] border border-[#2a3244] hover:border-[var(--accent)] text-slate-200 text-xs shrink-0 group transition-all">
          <svelte:component this={getFileIcon(item)} size={13} class="text-[var(--accent)] shrink-0" />
          <span class="font-medium truncate max-w-[160px]" title={item.path}>{item.name}</span>
          <span class="text-[10px] text-slate-500 font-mono">{item.formatted_size}</span>
          <button
            class="opacity-50 group-hover:opacity-100 hover:text-red-400 p-0.5 transition-opacity"
            on:click|stopPropagation={() => removeFromStash(item.path)}
            title="Ta bort från stash"
          >
            <X size={11} />
          </button>
        </div>
      {/each}
    </div>
  </div>
{/if}

<!-- rs-qc Modal Output -->
{#if qcReportModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-6">
    <div class="w-[680px] max-h-[80vh] flex flex-col bg-[#14171d] border border-[#262d3d] rounded-2xl shadow-2xl overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 bg-[#191d24] border-b border-[#262d3d]">
        <div class="flex items-center gap-2 text-purple-400 font-bold">
          <Activity size={16} />
          <span>rs-qc Alignment QC Rapport</span>
        </div>
        <button
          class="p-1 rounded hover:bg-white/10 text-slate-400 hover:text-white"
          on:click={() => (qcReportModal = '')}
        >
          <X size={16} />
        </button>
      </div>
      <div class="flex-1 overflow-auto p-4 font-mono text-xs text-slate-200 bg-[#0c0d10] leading-relaxed select-text">
        <pre class="m-0 whitespace-pre-wrap">{qcReportModal}</pre>
      </div>
      <div class="px-4 py-2 bg-[#191d24] border-t border-[#262d3d] flex justify-end">
        <button
          class="px-4 py-1.5 rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-xs font-semibold"
          on:click={() => (qcReportModal = '')}
        >
          Stäng
        </button>
      </div>
    </div>
  </div>
{/if}
