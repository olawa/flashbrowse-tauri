<script lang="ts">
  import {
    trashItems,
    revealInOs,
    openInDefault,
    launchRsnap,
    createZipArchive,
    sendToIgv,
    generateRsnapSnapshot,
    runQcBatch,
    cancelQc,
    type QcProgress,
  } from '../invoke';
  import { listen } from '@tauri-apps/api/event';
  import { addMultipleToStash } from '../stores/stash';
  import { reloadPane, activePaneId, transferBetweenPanes, isDualPane, leftPane, rightPane } from '../stores/navigation';
  import { addTracksToHub, isGenomicsHubOpen } from '../stores/genomicsStore';
  import { get } from 'svelte/store';
  import { saveMultipleItems, downloadDirectory, isSavingFile, getSSHServerFolderName } from '../stores/downloadStore';
  import type { FileItem } from '../types';
  import {
    Files,
    Trash2,
    Layers,
    Copy,
    Check,
    Folder,
    Dna,
    Table,
    Code,
    Image as ImageIcon,
    FileText,
    Archive,
    X,
    FolderOpen,
    ExternalLink,
    AlertTriangle,
    Camera,
    ArrowRightLeft,
    Download,
    Radio,
    Sparkles,
    Activity,
  } from 'lucide-svelte';

  export let items: FileItem[] = [];
  export let onDeselectItem: ((path: string) => void) | null = null;
  export let onClearSelection: (() => void) | null = null;

  let copied = false;
  let stashedDone = false;
  let isTrashing = false;
  let isZipping = false;
  let zippedDone = false;
  let filterCategory: string | null = null;

  async function handleCompressAll() {
    if (items.length === 0 || isZipping) return;
    isZipping = true;
    try {
      const paths = items.map((i) => i.path);
      await createZipArchive(paths);
      zippedDone = true;
      setTimeout(() => (zippedDone = false), 2500);
      reloadPane($activePaneId);
    } catch (e: any) {
      alert(`Kunde inte komprimera filer: ${e}`);
    } finally {
      isZipping = false;
    }
  }

  $: totalCount = items.length;
  $: dirCount = items.filter((i) => i.is_dir).length;
  $: fileCount = items.filter((i) => !i.is_dir).length;
  $: totalBytes = items.reduce((acc, i) => acc + (i.is_dir ? 0 : i.size_bytes), 0);
  $: formattedTotalSize = formatBytes(totalBytes);

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  // Categories breakdown
  $: categories = (() => {
    let bam = 0, vcf = 0, fastq = 0, tables = 0, code = 0, docs = 0, img = 0, arc = 0;
    for (const item of items) {
      if (item.is_dir) continue;
      const ext = item.extension.toLowerCase();
      const name = item.name.toLowerCase();
      if (ext === 'bam' || ext === 'cram' || ext === 'sam' || name.endsWith('.bam') || name.endsWith('.cram')) bam++;
      else if (ext === 'vcf' || ext === 'bcf' || name.endsWith('.vcf.gz')) vcf++;
      else if (ext === 'fq' || ext === 'fastq' || name.endsWith('.fastq.gz')) fastq++;
      else if (['tsv', 'csv', 'tab', 'xlsx', 'xls'].includes(ext)) tables++;
      else if (['rs', 'py', 'ts', 'js', 'sh', 'c', 'cpp', 'swift', 'r', 'json'].includes(ext)) code++;
      else if (['txt', 'md', 'pdf', 'log', 'doc', 'docx'].includes(ext)) docs++;
      else if (['png', 'jpg', 'jpeg', 'webp', 'svg', 'gif'].includes(ext)) img++;
      else if (['zip', 'tar', 'gz', 'tgz', 'bz2'].includes(ext)) arc++;
    }

    const list = [];
    if (dirCount > 0) list.push({ id: 'dir', label: `${dirCount} Mappar`, icon: Folder, color: 'text-amber-400 bg-amber-950/40 border-amber-800/60' });
    if (bam > 0) list.push({ id: 'bam', label: `${bam} BAM/CRAM`, icon: Dna, color: 'text-emerald-400 bg-emerald-950/40 border-emerald-800/60' });
    if (vcf > 0) list.push({ id: 'vcf', label: `${vcf} VCF`, icon: Dna, color: 'text-purple-400 bg-purple-950/40 border-purple-800/60' });
    if (fastq > 0) list.push({ id: 'fastq', label: `${fastq} FASTQ`, icon: Dna, color: 'text-cyan-400 bg-cyan-950/40 border-cyan-800/60' });
    if (tables > 0) list.push({ id: 'tables', label: `${tables} Tabeller`, icon: Table, color: 'text-blue-400 bg-blue-950/40 border-blue-800/60' });
    if (code > 0) list.push({ id: 'code', label: `${code} Kod/Script`, icon: Code, color: 'text-yellow-400 bg-yellow-950/40 border-yellow-800/60' });
    if (docs > 0) list.push({ id: 'docs', label: `${docs} Dokument`, icon: FileText, color: 'text-slate-300 bg-slate-800/40 border-slate-700/60' });
    if (img > 0) list.push({ id: 'img', label: `${img} Bilder`, icon: ImageIcon, color: 'text-pink-400 bg-pink-950/40 border-pink-800/60' });
    if (arc > 0) list.push({ id: 'arc', label: `${arc} Arkiv`, icon: Archive, color: 'text-orange-400 bg-orange-950/40 border-orange-800/60' });
    return list;
  })();

  $: filteredItems = items.filter((item) => {
    if (!filterCategory) return true;
    if (filterCategory === 'dir') return item.is_dir;
    if (item.is_dir) return false;
    const ext = item.extension.toLowerCase();
    const name = item.name.toLowerCase();
    if (filterCategory === 'bam') return ext === 'bam' || ext === 'cram' || ext === 'sam' || name.endsWith('.bam') || name.endsWith('.cram');
    if (filterCategory === 'vcf') return ext === 'vcf' || ext === 'bcf' || name.endsWith('.vcf.gz');
    if (filterCategory === 'fastq') return ext === 'fq' || ext === 'fastq' || name.endsWith('.fastq.gz');
    if (filterCategory === 'tables') return ['tsv', 'csv', 'tab', 'xlsx', 'xls'].includes(ext);
    if (filterCategory === 'code') return ['rs', 'py', 'ts', 'js', 'sh', 'c', 'cpp', 'swift', 'r', 'json'].includes(ext);
    if (filterCategory === 'docs') return ['txt', 'md', 'pdf', 'log', 'doc', 'docx'].includes(ext);
    if (filterCategory === 'img') return ['png', 'jpg', 'jpeg', 'webp', 'svg', 'gif'].includes(ext);
    if (filterCategory === 'arc') return ['zip', 'tar', 'gz', 'tgz', 'bz2'].includes(ext);
    return true;
  });

  $: bamPaths = items
    .filter((i) => !i.is_dir && (i.extension === 'bam' || i.extension === 'cram' || i.name.endsWith('.bam') || i.name.endsWith('.cram')))
    .map((i) => i.path);

  $: activePane = $activePaneId === 'left' ? $leftPane : $rightPane;
  $: isSSH = activePane.isSSH;
  $: sshFolderName = isSSH && activePane.sshHost ? getSSHServerFolderName(activePane.sshHost) : '';
  $: downloadTargetLabel = isSSH && sshFolderName ? `Downloads/${sshFolderName}` : 'Mac';
  $: downloadButtonTitle = isSSH && sshFolderName
    ? `Spara alla markerade filer permanent till Downloads/${sshFolderName}`
    : `Spara permanent lokal kopia av alla markerade filer till ${$downloadDirectory || '~/Downloads'}`;

  let isSavedAll = false;

  async function handleSaveAllToDownloads() {
    if (items.length === 0) return;
    const store = $activePaneId === 'left' ? leftPane : rightPane;
    const paneState = get(store);
    const res = await saveMultipleItems(
      paneState.isSSH,
      paneState.sshHost,
      items.map((i) => i.path)
    );
    if (res.success) {
      isSavedAll = true;
      setTimeout(() => (isSavedAll = false), 2500);
    }
  }

  async function handleTransfer() {
    if (items.length === 0) return;
    const fromPane = $activePaneId;
    const toPane = fromPane === 'left' ? 'right' : 'left';
    await transferBetweenPanes(fromPane, toPane, items.map((i) => i.path));
  }

  async function handleTrash() {
    if (items.length === 0 || isTrashing) return;
    isTrashing = true;
    try {
      const paths = items.map((i) => i.path);
      await trashItems(paths);
      if (onClearSelection) onClearSelection();
      reloadPane($activePaneId);
    } catch (e) {
      console.error('Failed to trash items:', e);
    } finally {
      isTrashing = false;
    }
  }

  function handleStash() {
    if (items.length === 0) return;
    addMultipleToStash(items);
    stashedDone = true;
    setTimeout(() => (stashedDone = false), 1500);
  }

  async function copyPaths() {
    if (items.length === 0) return;
    try {
      const text = items.map((i) => i.path).join('\n');
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {}
  }

  $: genomicsItems = items.filter((i) => {
    if (i.is_dir) return false;
    const ext = i.extension.toLowerCase();
    const name = i.name.toLowerCase();
    return (
      ext === 'bam' || ext === 'cram' || ext === 'sam' ||
      ext === 'vcf' || ext === 'bcf' || name.endsWith('.vcf.gz') ||
      ext === 'bed' || ext === 'bw' || ext === 'bigwig'
    );
  });

  // Batch QC: rs-qc picks its module per file - rna for spliced alignments,
  // dna for the rest, fastq for reads - so a mixed selection needs no sorting
  // by hand.
  $: qcCandidates = items.filter((i) => {
    if (i.is_dir) return false;
    return /\.(bam|cram)$/i.test(i.path) || /\.(fastq|fq)(\.gz)?$/i.test(i.path);
  });

  let qcProgress: QcProgress | null = null;
  let qcMessage = '';
  let qcId: string | null = null;
  let qcListenerStarted = false;

  function startQcListener() {
    if (qcListenerStarted) return;
    qcListenerStarted = true;
    listen<QcProgress>('qc-progress', (event) => {
      if (event.payload.id !== qcId) return;
      qcProgress = event.payload.done ? null : event.payload;
      if (event.payload.done && event.payload.failures.length > 0) {
        qcMessage = event.payload.failures.join('\n');
      }
    }).catch(console.error);
  }

  async function handleRunQcBatch() {
    if (qcCandidates.length === 0) return;
    startQcListener();
    qcId = `qc${Date.now()}`;
    qcMessage = '';
    try {
      const summary = await runQcBatch(qcId, qcCandidates.map((i) => i.path));
      qcMessage = qcMessage ? `${summary}\n${qcMessage}` : summary;
    } catch (e: any) {
      qcMessage = String(e);
    } finally {
      qcProgress = null;
      qcId = null;
    }
  }

  async function handleCancelQc() {
    if (qcId) await cancelQc(qcId);
  }

  // Multi-sample snapshot: rsnap stacks one panel per BAM over the same region.
  $: alignmentItems = items.filter((i) => {
    if (i.is_dir) return false;
    const ext = i.extension.toLowerCase();
    return ext === 'bam' || ext === 'cram';
  });

  let snapshotRegion = '';
  let snapshotB64: string | null = null;
  let snapshotError = '';
  let isGeneratingSnapshot = false;

  async function handleMultiSnapshot() {
    const region = snapshotRegion.trim();
    if (alignmentItems.length === 0 || !region) return;

    isGeneratingSnapshot = true;
    snapshotError = '';
    snapshotB64 = null;
    try {
      // No genome id: the backend reads the build from the first BAM header.
      snapshotB64 = await generateRsnapSnapshot(
        alignmentItems.map((i) => i.path),
        region
      );
    } catch (e: any) {
      snapshotError = String(e);
    } finally {
      isGeneratingSnapshot = false;
    }
  }

  async function handleOpenRsnap() {
    if (genomicsItems.length === 0) return;
    try {
      await launchRsnap(genomicsItems.map((i) => i.path).slice(0, 10));
    } catch (e) {
      console.warn('Could not launch rsnap:', e);
    }
  }

  function handleOpenGenomicsHub() {
    if (genomicsItems.length === 0) return;
    addTracksToHub(genomicsItems);
    isGenomicsHubOpen.set(true);
  }

  let isSendingIgv = false;
  async function handleSendGenomicsToIgv() {
    if (genomicsItems.length === 0) return;
    isSendingIgv = true;
    try {
      const res = await sendToIgv(genomicsItems.map((i) => i.path));
      alert(res.message || 'Skickat till IGV!');
    } catch (e: any) {
      alert(`IGV fel: ${e}`);
    } finally {
      isSendingIgv = false;
    }
  }

  function getFileIcon(item: FileItem) {
    if (item.is_dir) return { icon: Folder, color: 'text-amber-400' };
    const ext = item.extension.toLowerCase();
    const name = item.name.toLowerCase();
    if (ext === 'bam' || ext === 'cram' || ext === 'sam' || name.endsWith('.bam') || name.endsWith('.cram')) return { icon: Dna, color: 'text-emerald-400' };
    if (ext === 'vcf' || ext === 'bcf' || name.endsWith('.vcf.gz')) return { icon: Dna, color: 'text-purple-400' };
    if (ext === 'fq' || ext === 'fastq' || name.endsWith('.fastq.gz')) return { icon: Dna, color: 'text-cyan-400' };
    if (['tsv', 'csv', 'tab', 'xlsx', 'xls'].includes(ext)) return { icon: Table, color: 'text-blue-400' };
    if (['rs', 'py', 'ts', 'js', 'sh', 'c', 'cpp', 'swift', 'r', 'json'].includes(ext)) return { icon: Code, color: 'text-yellow-400' };
    if (['png', 'jpg', 'jpeg', 'webp', 'svg', 'gif'].includes(ext)) return { icon: ImageIcon, color: 'text-pink-400' };
    if (['zip', 'tar', 'gz', 'tgz', 'bz2'].includes(ext)) return { icon: Archive, color: 'text-orange-400' };
    return { icon: FileText, color: 'text-slate-300' };
  }
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden bg-[#0d0f14] text-slate-200 text-xs select-none">
  <!-- Header Card -->
  <div class="p-3.5 bg-[#151922] border-b border-[#252d3d] flex items-center justify-between gap-3 shrink-0">
    <div class="flex items-center gap-2.5 min-w-0">
      <div class="w-8 h-8 rounded-xl bg-blue-500/20 text-blue-400 flex items-center justify-center shrink-0 border border-blue-500/30">
        <Files size={18} />
      </div>
      <div>
        <div class="flex items-center gap-2 flex-wrap">
          <span class="font-bold text-sm text-white">{totalCount} objekt markerade</span>
          <span class="px-2 py-0.5 rounded-full bg-[#222838] text-slate-300 font-mono text-[11px] font-semibold border border-[#2d374d]">
            {formattedTotalSize}
          </span>
        </div>
        <div class="text-[11px] text-slate-400 font-mono mt-0.5">
          {fileCount} {fileCount === 1 ? 'fil' : 'filer'}{#if dirCount > 0}, {dirCount} {dirCount === 1 ? 'mapp' : 'mappar'}{/if}
        </div>
      </div>
    </div>

    <!-- Clear Selection -->
    {#if onClearSelection}
      <button
        class="p-1.5 rounded-lg bg-[#0e1015] hover:bg-white/10 border border-[#252d3d] text-slate-400 hover:text-white transition-colors"
        on:click={onClearSelection}
        title="Avmarkera alla filer (Esc)"
      >
        <X size={14} />
      </button>
    {/if}
  </div>

  <!-- Primary Batch Action Buttons -->
  <div class="p-3 bg-[#11141b] border-b border-[#252d3d] flex items-center gap-2 flex-wrap shrink-0">
    <!-- Save All permanently to Downloads -->
    <button
      class="px-3 py-1.5 rounded-lg bg-emerald-950/60 hover:bg-emerald-900/80 text-emerald-300 border border-emerald-800 font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm {isSavedAll ? 'bg-emerald-600 text-white' : ''}"
      on:click={handleSaveAllToDownloads}
      disabled={$isSavingFile}
      title={downloadButtonTitle}
    >
      {#if $isSavingFile}
        <div class="w-3 h-3 border-2 border-emerald-400 border-t-transparent rounded-full animate-spin"></div>
        <span>Sparar...</span>
      {:else if isSavedAll}
        <Check size={13} class="text-white" />
        <span>Sparade alla!</span>
      {:else}
        <Download size={13} class="text-emerald-400" />
        <span>Spara alla till {downloadTargetLabel} ({totalCount})</span>
      {/if}
    </button>

    <!-- Transfer / Copy to other pane -->
    {#if $isDualPane}
      <button
        class="px-3 py-1.5 rounded-lg bg-cyan-950/60 hover:bg-cyan-900/80 text-cyan-300 border border-cyan-800 font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm"
        on:click={handleTransfer}
        title="Överför/kopiera markerade filer till den andra panelen (F5)"
      >
        <ArrowRightLeft size={13} class="text-cyan-400" />
        <span>Överför ({totalCount})</span>
        <kbd class="ml-1 px-1 py-0.2 rounded bg-black/40 text-[9px] font-mono opacity-70">F5</kbd>
      </button>
    {/if}

    <!-- Trash / Delete (Cmd+Backspace) -->
    <button
      class="px-3 py-1.5 rounded-lg bg-red-950/60 hover:bg-red-900/80 text-red-300 border border-red-800 font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm disabled:opacity-50"
      on:click={handleTrash}
      disabled={isTrashing}
      title="Flytta markerade objekt till Papperskorgen (Cmd+Backspace)"
    >
      <Trash2 size={13} class="text-red-400" />
      <span>Kasta ({totalCount})</span>
      <kbd class="ml-1 px-1 py-0.2 rounded bg-black/40 text-[9px] font-mono opacity-70">⌘⌫</kbd>
    </button>

    <!-- Stash Shelf -->
    <button
      class="px-3 py-1.5 rounded-lg bg-amber-950/50 hover:bg-amber-900/70 text-amber-300 border border-amber-800/80 font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm {stashedDone ? 'border-emerald-500 text-emerald-300' : ''}"
      on:click={handleStash}
      title="Lägg alla markerade filer i Samlingsfacket (Stash Shelf)"
    >
      {#if stashedDone}
        <Check size={13} class="text-emerald-400" />
        <span>Stashade!</span>
      {:else}
        <Layers size={13} class="text-amber-400" />
        <span>Stash</span>
      {/if}
    </button>

    <!-- Compress All to Zip -->
    <button
      class="px-3 py-1.5 rounded-lg bg-orange-950/50 hover:bg-orange-900/70 text-orange-300 border border-orange-800/80 font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm {zippedDone ? 'border-emerald-500 text-emerald-300' : ''}"
      on:click={handleCompressAll}
      disabled={isZipping}
      title="Komprimera alla markerade filer till ett .zip-arkiv"
    >
      {#if isZipping}
        <div class="w-3 h-3 border-2 border-orange-400 border-t-transparent rounded-full animate-spin"></div>
        <span>Zippar...</span>
      {:else if zippedDone}
        <Check size={13} class="text-emerald-400" />
        <span>Zippat!</span>
      {:else}
        <Archive size={13} class="text-orange-400" />
        <span>Komprimera (.zip)</span>
      {/if}
    </button>

    <!-- Copy Absolute Paths -->
    <button
      class="px-3 py-1.5 rounded-lg bg-[#181d27] hover:bg-[#222836] text-slate-300 hover:text-white border border-[#252d3d] font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm"
      on:click={copyPaths}
      title="Kopiera absoluta sökvägar till urklipp"
    >
      {#if copied}
        <Check size={13} class="text-emerald-400" />
        <span>Kopierade!</span>
      {:else}
        <Copy size={13} />
        <span>Kopiera sökvägar</span>
      {/if}
    </button>

    <!-- Genomics Hub & Viewer Launch (if Genomics tracks present) -->
    {#if genomicsItems.length > 0}
      <button
        class="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm"
        on:click={handleOpenRsnap}
        title="Öppna markerade spår i rsnap Desktop Viewer"
      >
        <ExternalLink size={13} />
        <span>rsnap ({genomicsItems.length})</span>
      </button>

      <button
        class="px-3 py-1.5 rounded-lg bg-blue-600/90 hover:bg-blue-500 text-white font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm"
        on:click={handleSendGenomicsToIgv}
        disabled={isSendingIgv}
        title="Skicka markerade spår till IGV desktop (port 60151)"
      >
        <Radio size={13} />
        <span>IGV ({genomicsItems.length})</span>
      </button>

      <button
        class="px-3 py-1.5 rounded-lg bg-[#202738] hover:bg-[#2c364c] text-emerald-300 hover:text-white border border-[#323e57] font-semibold text-xs transition-colors flex items-center gap-1.5 shadow-sm"
        on:click={handleOpenGenomicsHub}
        title="Öppna Genomics Track Hub (hantera spår, server och IGV)"
      >
        <Sparkles size={13} class="text-amber-400" />
        <span>Genomics Hub</span>
      </button>
    {/if}
  </div>

  <!-- Batch QC over everything selected that rs-qc can read -->
  {#if qcCandidates.length > 1}
    <div class="p-3 bg-[#11141b] border-b border-[#252d3d] shrink-0 space-y-2">
      <div class="flex items-center gap-2 flex-wrap">
        <span class="flex items-center gap-1.5 text-[11px] font-semibold text-purple-300 shrink-0">
          <Activity size={13} class="text-purple-400" />
          Kör QC ({qcCandidates.length} filer)
        </span>

        {#if qcProgress}
          <div class="flex-1 min-w-[12rem] flex items-center gap-2">
            <div class="flex-1 h-1.5 rounded-full bg-purple-500/20 overflow-hidden">
              <div
                class="h-full bg-purple-400 transition-[width] duration-200"
                style="width: {(qcProgress.files_done / Math.max(1, qcProgress.files_total)) * 100}%"
              ></div>
            </div>
            <span class="text-[10px] font-mono text-purple-200 tabular-nums shrink-0">
              {qcProgress.files_done}/{qcProgress.files_total} · {qcProgress.module}
            </span>
            <button
              class="px-2 py-0.5 rounded border border-purple-500/50 hover:bg-purple-500/20 text-purple-200 text-[10px] shrink-0"
              on:click={handleCancelQc}
            >
              Avbryt
            </button>
          </div>
        {:else}
          <button
            class="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white font-semibold text-xs transition-colors flex items-center gap-1.5"
            on:click={handleRunQcBatch}
            title="rs-qc väljer modul per fil: rna för spliced alignments, dna för övriga, fastq för läsningar. Resultaten skrivs bredvid filerna."
          >
            <Activity size={13} />
            <span>Kör rs-qc</span>
          </button>
        {/if}
      </div>

      {#if qcProgress?.current_file}
        <div class="text-[10px] font-mono text-slate-400 truncate">{qcProgress.current_file}</div>
      {/if}
      {#if qcMessage}
        <pre class="text-[10.5px] text-slate-300 bg-[#0c0d10] border border-[#252d3d] rounded p-2 whitespace-pre-wrap break-words m-0 font-mono max-h-32 overflow-auto">{qcMessage}</pre>
      {/if}
    </div>
  {/if}

  <!-- Multi-sample rsnap snapshot: all selected alignments, one region -->
  {#if alignmentItems.length > 1}
    <div class="p-3 bg-[#11141b] border-b border-[#252d3d] shrink-0 space-y-2">
      <div class="flex items-center gap-2 flex-wrap">
        <span class="flex items-center gap-1.5 text-[11px] font-semibold text-emerald-300 shrink-0">
          <Camera size={13} class="text-emerald-400" />
          Gemensam snapshot ({alignmentItems.length} prover)
        </span>
        <input
          type="text"
          bind:value={snapshotRegion}
          on:keydown={(e) => e.key === 'Enter' && handleMultiSnapshot()}
          placeholder="Gen eller region — t.ex. TP53, BRCA1 exon11, chr17:7565097-7590856"
          class="flex-1 min-w-[14rem] bg-[var(--bg-panel)] text-[11px] text-[var(--text-primary)] px-2 py-1 rounded border border-[var(--border)] focus:border-[var(--accent)] focus:outline-none font-mono placeholder:text-[var(--text-muted)]"
        />
        <button
          class="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white font-semibold text-xs transition-colors flex items-center gap-1.5 shrink-0"
          on:click={handleMultiSnapshot}
          disabled={isGeneratingSnapshot || !snapshotRegion.trim()}
        >
          {#if isGeneratingSnapshot}
            <div class="w-3 h-3 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
            <span>Renderar…</span>
          {:else}
            <Camera size={13} />
            <span>Rendera</span>
          {/if}
        </button>
      </div>

      {#if snapshotError}
        <pre class="text-[10.5px] text-red-300 bg-red-950/40 border border-red-900/60 rounded p-2 whitespace-pre-wrap break-words m-0 font-mono">{snapshotError}</pre>
      {/if}

      {#if snapshotB64}
        <img
          src="data:image/png;base64,{snapshotB64}"
          alt="rsnap-snapshot för {alignmentItems.length} prover"
          class="w-full rounded border border-[var(--border)] bg-black"
        />
      {/if}
    </div>
  {/if}

  <!-- Category Breakdown Chips -->
  {#if categories.length > 0}
    <div class="px-3 py-2 bg-[#0e1015] border-b border-[#252d3d] flex items-center gap-1.5 flex-wrap shrink-0">
      <button
        class="px-2 py-0.5 rounded-full text-[10.5px] font-medium transition-colors border {filterCategory === null ? 'bg-blue-600 text-white border-blue-500 font-bold' : 'bg-[#181d27] text-slate-400 border-[#252d3d] hover:text-white'}"
        on:click={() => (filterCategory = null)}
      >
        Alla ({totalCount})
      </button>
      {#each categories as cat}
        <button
          class="px-2 py-0.5 rounded-full text-[10.5px] font-medium transition-colors border flex items-center gap-1 {filterCategory === cat.id ? 'ring-2 ring-white ' + cat.color : cat.color + ' opacity-75 hover:opacity-100'}"
          on:click={() => (filterCategory = filterCategory === cat.id ? null : cat.id)}
        >
          <svelte:component this={cat.icon} size={11} />
          <span>{cat.label}</span>
        </button>
      {/each}
    </div>
  {/if}

  <!-- Selected Items List -->
  <div class="flex-1 overflow-y-auto p-2 divide-y divide-[#1f2533]/50">
    {#each filteredItems as item}
      {@const iconInfo = getFileIcon(item)}
      <div class="py-1.5 px-2 rounded hover:bg-[#161a24] flex items-center justify-between gap-2 group transition-colors">
        <!-- File Name and Icon -->
        <div class="flex items-center gap-2 min-w-0 flex-1">
          <svelte:component this={iconInfo.icon} size={14} class="{iconInfo.color} shrink-0" />
          <span class="truncate font-sans {item.is_dir ? 'font-semibold text-white' : 'text-slate-200'}" title={item.path}>
            {item.name}
          </span>
        </div>

        <!-- Size & Actions -->
        <div class="flex items-center gap-2 shrink-0">
          <span class="font-mono text-[10.5px] text-slate-400">
            {item.is_dir ? '--' : item.formatted_size}
          </span>

          <!-- Reveal in OS -->
          <button
            class="opacity-0 group-hover:opacity-100 p-0.5 rounded text-slate-400 hover:text-white transition-opacity"
            on:click={() => revealInOs(item.path)}
            title="Visa i Finder"
          >
            <FolderOpen size={12} />
          </button>

          <!-- Deselect individual item -->
          {#if onDeselectItem}
            <button
              class="opacity-0 group-hover:opacity-100 p-0.5 rounded text-slate-400 hover:text-red-400 transition-opacity"
              on:click={() => onDeselectItem && onDeselectItem(item.path)}
              title="Avmarkera denna fil"
            >
              <X size={12} />
            </button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>
