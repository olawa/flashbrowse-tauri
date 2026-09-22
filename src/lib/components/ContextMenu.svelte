<script lang="ts">
  import { onMount } from 'svelte';
  import {
    openInDefault,
    openFileWith,
    sshOpenFileLocally,
    revealInOs,
    trashItems,
    calculateDirSize,
    launchRsnap,
    runRsQc,
    createZipArchive,
    sendToIgv,
  } from '../invoke';
  import { openTerminalAt } from '../stores/terminal';
  import { refreshPane, leftPane, rightPane, transferBetweenPanes, isDualPane, navigatePane, activePaneId } from '../stores/navigation';
  import { addToStash } from '../stores/stash';
  import { castToSecondaryInspector } from '../stores/navigation';
  import { addTracksToHub, isGenomicsHubOpen } from '../stores/genomicsStore';
  import { activeIndexFilteredItems } from '../stores/indexStore';
  import { findRelatedBams, type RelatedBamResult } from '../invoke';
  import RelatedBamsModal from './RelatedBamsModal.svelte';
  import type { FileItem } from '../types';
  import {
    ExternalLink,
    FolderOpen,
    Copy,
    Trash2,
    Terminal as TerminalIcon,
    PieChart,
    Dna,
    Activity,
    Layers,
    Rocket,
    CheckCheck,
    X,
    ArrowRightLeft,
    Download,
    Archive,
    Radio,
    Sparkles,
    Table,
    FileText,
    FileCode,
    Code,
    ChevronRight,
    GitBranch,
  } from 'lucide-svelte';
  import { saveRemoteOrLocalItem, downloadDirectory, saveNotification, getSSHServerFolderName, getSSHDownloadDirectory } from '../stores/downloadStore';
  import {
    favoriteEditor,
    setFavoriteEditor,
    openInFavoriteEditor,
    SUPPORTED_EDITORS,
  } from '../stores/editorStore';

  export let item: FileItem;
  export let paneId: 'left' | 'right';
  export let x = 0;
  export let y = 0;
  export let onClose: () => void;

  let qcResultModal = '';
  let isOpenWithSubmenu = false;

  $: currentPaneState = paneId === 'left' ? $leftPane : $rightPane;
  $: isSSH = currentPaneState.isSSH || item.path.startsWith('ssh://');
  $: sshHost = currentPaneState.sshHost || (item.path.startsWith('ssh://') ? item.path.split('/')[2] : '');
  $: remotePath = item.path.startsWith('ssh://')
    ? item.path.replace(new RegExp(`^ssh://[^/]+`), '') || '/'
    : item.path;
  $: isMultiSelect = currentPaneState.selectedPaths.has(item.path) && currentPaneState.selectedPaths.size > 1;
  $: multiCount = isMultiSelect ? currentPaneState.selectedPaths.size : 1;

  const ext = item.extension.toLowerCase();
  const isBamOrCram = ext === 'bam' || ext === 'cram' || item.name.endsWith('.bam') || item.name.endsWith('.cram');
  const isGenomics = isBamOrCram || ['vcf', 'bcf', 'bed', 'bw', 'bigwig'].includes(ext) || item.name.endsWith('.vcf.gz');

  const isTable = ['xlsx', 'xls', 'csv', 'tsv', 'tab', 'ods'].includes(ext) || item.name.endsWith('.csv.gz') || item.name.endsWith('.tsv.gz');
  const isDocx = ['docx', 'doc', 'rtf', 'odt', 'pages'].includes(ext);
  const isPdf = ext === 'pdf';
  const isCode = ['py', 'rs', 'sh', 'bash', 'zsh', 'json', 'yaml', 'yml', 'toml', 'md', 'c', 'cpp', 'cc', 'h', 'hpp', 'swift', 'js', 'ts', 'jsx', 'tsx', 'svelte', 'vue', 'html', 'css', 'scss', 'r', 'smk', 'nf', 'wdl', 'txt', 'log', 'conf', 'cfg', 'ini', 'xml', 'sql'].includes(ext);

  async function handleOpenWith(appName?: string) {
    const host = sshHost;
    const path = remotePath;
    const name = item.name;
    const isRemote = isSSH;
    const isDir = item.is_dir;
    const fullPath = item.path;
    const targetDir = getSSHDownloadDirectory(host);
    onClose();

    if (isDir) {
      if (!isRemote) {
        await openFileWith(fullPath, appName);
      }
      return;
    }

    if (isRemote) {
      const appDisplay = appName || 'standardprogram';
      saveNotification.set({
        text: `⬇️ Hämtar ${name} för att öppna i ${appDisplay}...`,
        success: true,
      });
      try {
        const localPath = await sshOpenFileLocally(host, path, appName, targetDir);
        saveNotification.set({
          text: `🚀 Öppnade ${name} lokalt (${appDisplay})`,
          path: localPath,
          success: true,
        });
        setTimeout(() => saveNotification.set(null), 4000);
      } catch (err: any) {
        saveNotification.set({
          text: `❌ Kunde inte öppna: ${err?.message || err}`,
          success: false,
        });
        setTimeout(() => saveNotification.set(null), 5000);
      }
    } else {
      try {
        await openFileWith(fullPath, appName);
      } catch (err: any) {
        alert(`Kunde inte öppna med ${appName || 'standardprogram'}: ${err}`);
      }
    }
  }

  async function handleCompress() {
    try {
      const store = paneId === 'left' ? $leftPane : $rightPane;
      const paths = store.selectedPaths.has(item.path) && store.selectedPaths.size > 1
        ? Array.from(store.selectedPaths)
        : [item.path];
      await createZipArchive(paths);
      await refreshPane(paneId);
    } catch (e: any) {
      alert(`Kunde inte komprimera: ${e}`);
    }
    onClose();
  }

  async function handleCast() {
    await castToSecondaryInspector(item);
    onClose();
  }

  async function handleOpen() {
    if (!item.is_dir && isSSH) {
      await handleOpenWith(undefined);
    } else {
      await openInDefault(item.path);
      onClose();
    }
  }

  async function handleReveal() {
    await revealInOs(item.path);
    onClose();
  }

  async function handleOpenInEditor(editorName?: string) {
    if (editorName) {
      setFavoriteEditor(editorName);
    }
    const path = item.path;
    const isRemote = isSSH;
    const host = currentPaneState.sshHost;
    onClose();
    await openInFavoriteEditor(path, isRemote, host);
  }

  async function handleCopyPath() {
    try {
      await navigator.clipboard.writeText(item.path);
    } catch (err) {
      console.warn('Clipboard write failed:', err);
    }
    onClose();
  }

  async function handleOpenInTerminal() {
    activePaneId.set(paneId);
    const lastSlash = item.path.lastIndexOf('/');
    const dir = item.is_dir ? item.path : (lastSlash > 0 ? item.path.substring(0, lastSlash) : (item.path.startsWith('/') ? '/' : '~'));
    await openTerminalAt(dir);
    onClose();
  }

  async function handleTrash() {
    const store = paneId === 'left' ? $leftPane : $rightPane;
    const paths = store.selectedPaths.has(item.path) && store.selectedPaths.size > 1
      ? Array.from(store.selectedPaths)
      : [item.path];
    if (confirm(`Flytta ${paths.length} ${paths.length === 1 ? 'fil' : 'filer'} till papperskorgen?`)) {
      await trashItems(paths);
      await refreshPane(paneId);
    }
    onClose();
  }

  async function handleDu() {
    try {
      const summary = await calculateDirSize(item.path);
      alert(`Directory Size for ${summary.path}:\nTotal size: ${summary.formatted_total_size}\nFiles: ${summary.total_files}\nFolders: ${summary.total_dirs}`);
    } catch (e: any) {
      alert(`Failed to calculate size: ${e}`);
    }
    onClose();
  }

  async function handleRsnap() {
    try {
      const store = paneId === 'left' ? $leftPane : $rightPane;
      const paths = store.selectedPaths.has(item.path) && store.selectedPaths.size > 1
        ? Array.from(store.selectedPaths)
        : [item.path];
      await launchRsnap(paths);
    } catch (e: any) {
      alert(`rsnap fel: ${e}`);
    }
    onClose();
  }

  async function handleSendToIgv() {
    try {
      const store = paneId === 'left' ? $leftPane : $rightPane;
      const paths = store.selectedPaths.has(item.path) && store.selectedPaths.size > 1
        ? Array.from(store.selectedPaths)
        : [item.path];
      const res = await sendToIgv(paths);
      alert(res.message || 'Skickat till IGV!');
    } catch (e: any) {
      alert(`IGV fel: ${e}`);
    }
    onClose();
  }

  function handleOpenHub() {
    const store = paneId === 'left' ? $leftPane : $rightPane;
    const itemsToAdd = store.selectedPaths.has(item.path) && store.selectedPaths.size > 1
      ? store.items.filter((i) => store.selectedPaths.has(i.path))
      : [item];
    addTracksToHub(itemsToAdd);
    isGenomicsHubOpen.set(true);
    onClose();
  }

  // BAM files from the same reads: search the files the user can already see -
  // this pane's listing, plus the index when one is open - rather than walking
  // the disk, so the cost is bounded and predictable.
  let relatedResult: RelatedBamResult | null = null;
  let isFindingRelated = false;
  let relatedError = '';

  function relatedCandidates(): string[] {
    const store = paneId === 'left' ? $leftPane : $rightPane;
    const isAlignment = (p: string) => /\.(bam|cram)$/i.test(p);

    const fromPane = store.items.filter((i) => !i.is_dir && isAlignment(i.path)).map((i) => i.path);
    const fromIndex = $activeIndexFilteredItems.filter((i) => isAlignment(i.path)).map((i) => i.path);

    return Array.from(new Set([...fromPane, ...fromIndex]));
  }

  async function handleFindRelated() {
    isFindingRelated = true;
    relatedError = '';
    relatedResult = null;
    try {
      relatedResult = await findRelatedBams(item.path, relatedCandidates());
    } catch (e: any) {
      relatedError = String(e);
    } finally {
      isFindingRelated = false;
    }
  }

  function openRelated(path: string) {
    const dir = path.substring(0, path.lastIndexOf('/')) || '/';
    navigatePane(paneId, dir);
    relatedResult = null;
    onClose();
  }

  async function handleRsQc() {
    try {
      const result = await runRsQc(item.path);
      qcResultModal = `${result.report}\n\nFiler skrivna till: ${result.output_dir}`;
    } catch (e: any) {
      alert(`rs-qc fel: ${e}`);
      onClose();
    }
  }

  function handleAddToStash() {
    addToStash(item);
    onClose();
  }

  $: sameTypeCount = (() => {
    const store = paneId === 'left' ? $leftPane : $rightPane;
    if (item.is_dir) {
      return store.items.filter((i) => i.is_dir).length;
    }
    const ext = item.extension.toLowerCase();
    const name = item.name.toLowerCase();
    const isCompoundGz = name.endsWith('.vcf.gz') ? '.vcf.gz' :
                         name.endsWith('.fastq.gz') ? '.fastq.gz' :
                         name.endsWith('.fq.gz') ? '.fq.gz' :
                         name.endsWith('.tar.gz') ? '.tar.gz' : null;
    return store.items.filter((i) => {
      if (i.is_dir) return false;
      if (isCompoundGz) return i.name.toLowerCase().endsWith(isCompoundGz);
      return i.extension.toLowerCase() === ext;
    }).length;
  })();

  $: selectedGenomicsCount = (() => {
    const store = paneId === 'left' ? $leftPane : $rightPane;
    if (store.selectedPaths.has(item.path) && store.selectedPaths.size > 1) {
      return store.items.filter((i) => store.selectedPaths.has(i.path) && (
        ['bam', 'cram', 'sam', 'vcf', 'bcf', 'bed', 'bw', 'bigwig'].includes(i.extension.toLowerCase()) ||
        i.name.toLowerCase().endsWith('.vcf.gz')
      )).length;
    }
    return 1;
  })();

  function handleSelectSameType() {
    const store = paneId === 'left' ? leftPane : rightPane;
    store.update((s) => {
      let matchingPaths: string[] = [];
      if (item.is_dir) {
        matchingPaths = s.items.filter((i) => i.is_dir).map((i) => i.path);
      } else {
        const ext = item.extension.toLowerCase();
        const name = item.name.toLowerCase();
        const isCompoundGz = name.endsWith('.vcf.gz') ? '.vcf.gz' :
                             name.endsWith('.fastq.gz') ? '.fastq.gz' :
                             name.endsWith('.fq.gz') ? '.fq.gz' :
                             name.endsWith('.tar.gz') ? '.tar.gz' : null;

        matchingPaths = s.items
          .filter((i) => {
            if (i.is_dir) return false;
            if (isCompoundGz) return i.name.toLowerCase().endsWith(isCompoundGz);
            return i.extension.toLowerCase() === ext;
          })
          .map((i) => i.path);
      }
      return { ...s, selectedPaths: new Set(matchingPaths) };
    });
    onClose();
  }

  async function handleTransferToOtherPane() {
    const otherPane = paneId === 'left' ? 'right' : 'left';
    const store = paneId === 'left' ? $leftPane : $rightPane;
    const paths = store.selectedPaths.has(item.path) && store.selectedPaths.size > 1
      ? Array.from(store.selectedPaths)
      : [item.path];
    await transferBetweenPanes(paneId, otherPane, paths);
    onClose();
  }

  async function handleSaveToDownloads() {
    const store = paneId === 'left' ? $leftPane : $rightPane;
    await saveRemoteOrLocalItem(store.isSSH, store.sshHost, item.path);
    onClose();
  }

  let canClose = false;
  onMount(() => {
    const t = setTimeout(() => {
      canClose = true;
    }, 150);
    return () => clearTimeout(t);
  });

  function handleBackdrop(e: MouseEvent) {
    if (!canClose) return;
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  $: sshFolderName = currentPaneState.isSSH && currentPaneState.sshHost ? getSSHServerFolderName(currentPaneState.sshHost) : '';
  $: downloadButtonLabel = sshFolderName ? `Downloads/${sshFolderName}` : 'Downloads';
  $: downloadButtonTitle = sshFolderName 
    ? `Spara permanent till Downloads/${sshFolderName}` 
    : `Spara permanent lokal kopia till ${$downloadDirectory || '~/Downloads'}`;

  // Viewport clamping so menu never overflows off-screen
  let adjustedX = x;
  let adjustedY = y;
  $: if (typeof window !== 'undefined') {
    const menuWidth = 230;
    const menuHeight = 360;
    adjustedX = Math.max(8, Math.min(x, window.innerWidth - menuWidth - 8));
    if (y + menuHeight > window.innerHeight - 8) {
      adjustedY = Math.max(8, window.innerHeight - menuHeight - 8);
    } else {
      adjustedY = Math.max(8, y);
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- Full-screen invisible backdrop to capture and isolate pointer events exclusively to menu -->
<div
  class="fixed inset-0 z-40 bg-transparent cursor-default select-none"
  on:contextmenu|preventDefault|stopPropagation={handleBackdrop}
  on:mousedown|stopPropagation={handleBackdrop}
  on:click|stopPropagation={handleBackdrop}
></div>

<div
  class="fixed z-50 w-56 py-1 bg-[var(--bg-surface)] border border-[var(--border)] rounded-md shadow-2xl text-xs text-[var(--text-primary)] select-none backdrop-blur-md"
  style="top: {adjustedY}px; left: {adjustedX}px;"
  on:click|stopPropagation
  on:mousedown|stopPropagation
>
  <!-- Save permanently to Downloads -->
  <button
    class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-emerald-600 hover:text-[var(--text-primary)] text-left transition-colors font-medium text-emerald-400"
    on:click={handleSaveToDownloads}
    title={downloadButtonTitle}
  >
    <div class="flex items-center gap-2 min-w-0">
      <Download size={13} class="text-emerald-400 shrink-0" />
      <span class="truncate">Spara till {downloadButtonLabel}</span>
    </div>
    <span class="text-[9px] font-mono opacity-70">Lokal</span>
  </button>

  <!-- Transfer to other pane -->
  {#if $isDualPane}
    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-cyan-600 hover:text-[var(--text-primary)] text-left transition-colors font-medium text-cyan-400"
      on:click={handleTransferToOtherPane}
      title="Överför till motsatt panel"
    >
      <div class="flex items-center gap-2 min-w-0">
        <ArrowRightLeft size={13} class="text-cyan-400 shrink-0" />
        <span class="truncate">{isMultiSelect ? `Flytta ${multiCount} filer till ${paneId === 'left' ? 'höger' : 'vänster'}` : `Flytta till ${paneId === 'left' ? 'höger' : 'vänster'}`}</span>
      </div>
      <kbd class="text-[9px] font-mono opacity-70">F5</kbd>
    </button>
  {/if}

  <!-- Select all of same type -->
  <button
    class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--accent)] hover:text-[var(--text-primary)] text-left transition-colors font-medium text-emerald-400 hover:text-[var(--text-primary)]"
    on:click={handleSelectSameType}
  >
    <div class="flex items-center gap-2 min-w-0">
      <CheckCheck size={13} class="text-emerald-400 shrink-0" />
      <span class="truncate">Markera alla av samma typ</span>
    </div>
    <span class="text-[10px] font-mono opacity-70 ml-1 shrink-0">
      {item.is_dir ? 'mapp' : `.${item.extension || 'fil'}`} ({sameTypeCount})
    </span>
  </button>

  <!-- Compress to Zip -->
  <button
    class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-orange-600 hover:text-[var(--text-primary)] text-left transition-colors font-medium text-orange-400 hover:text-[var(--text-primary)]"
    on:click={handleCompress}
    title="Komprimera till zip-arkiv"
  >
    <div class="flex items-center gap-2 min-w-0">
      <Archive size={13} class="text-orange-400 shrink-0" />
      <span class="truncate">Komprimera (.zip)</span>
    </div>
    <span class="text-[9px] font-mono opacity-70">Arkiv</span>
  </button>

  <div class="h-px my-1 bg-[var(--border)]"></div>

  {#if isBamOrCram}
    <button
      class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-purple-600 hover:text-[var(--text-primary)] text-purple-400 font-medium text-left transition-colors"
      on:click={handleRsQc}
    >
      <Activity size={13} />
      <span>Kör rs-qc (Alignment QC)</span>
    </button>

    <button
      class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-emerald-600 hover:text-[var(--text-primary)] text-emerald-400 font-medium text-left transition-colors"
      on:click={handleFindRelated}
      title="Läser @PG-kommandoraden och @RG-läsgrupperna i headern och jämför med övriga BAM-filer i listan och indexet"
    >
      <GitBranch size={13} />
      <span>BAM-filer från samma FASTQ</span>
    </button>
  {/if}

  {#if isGenomics}
    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-emerald-600 hover:text-[var(--text-primary)] text-emerald-400 font-medium text-left transition-colors"
      on:click={handleRsnap}
      title="Öppna i rsnap Desktop Viewer"
    >
      <div class="flex items-center gap-2 min-w-0">
        <ExternalLink size={13} class="text-emerald-400 shrink-0" />
        <span class="truncate">Öppna i rsnap Viewer</span>
      </div>
      {#if selectedGenomicsCount > 1}
        <span class="text-[9.5px] font-mono px-1.5 py-0.2 rounded bg-emerald-950 text-emerald-300 border border-emerald-800 shrink-0">
          {selectedGenomicsCount} spår
        </span>
      {/if}
    </button>

    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-blue-600 hover:text-[var(--text-primary)] text-blue-400 font-medium text-left transition-colors"
      on:click={handleSendToIgv}
      title="Skicka till IGV Desktop (port 60151)"
    >
      <div class="flex items-center gap-2 min-w-0">
        <Radio size={13} class="text-blue-400 shrink-0" />
        <span class="truncate">Skicka till IGV Desktop</span>
      </div>
      {#if selectedGenomicsCount > 1}
        <span class="text-[9.5px] font-mono px-1.5 py-0.2 rounded bg-blue-950 text-blue-300 border border-blue-800 shrink-0">
          {selectedGenomicsCount} spår
        </span>
      {/if}
    </button>

    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--bg-active)] text-emerald-300 font-medium text-left transition-colors"
      on:click={handleOpenHub}
      title="Öppna i Genomics Hub..."
    >
      <div class="flex items-center gap-2 min-w-0">
        <Sparkles size={13} class="text-amber-400 shrink-0" />
        <span class="truncate">Öppna i Genomics Hub...</span>
      </div>
      {#if selectedGenomicsCount > 1}
        <span class="text-[9.5px] font-mono px-1.5 py-0.2 rounded bg-amber-950 text-amber-300 border border-amber-800 shrink-0">
          {selectedGenomicsCount} spår
        </span>
      {/if}
    </button>
    <div class="h-px my-1 bg-[var(--border)]"></div>
  {/if}

  <button
    class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-amber-600 hover:text-[var(--text-primary)] text-left text-amber-400 font-medium transition-colors"
    on:click={handleCast}
    title="Kasta fil till stort inspektörsfönster (⌘K, ⌥⏎ eller dra uppåt)"
  >
    <div class="flex items-center gap-2 min-w-0">
      <Rocket size={13} class="shrink-0" />
      <span class="truncate">Kasta till Stort Fönster</span>
    </div>
    <span class="text-[10px] font-mono opacity-80 shrink-0 ml-1">⌘K / ⌥⏎</span>
  </button>

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-[var(--text-primary)] text-left"
    on:click={handleAddToStash}
  >
    <Layers size={13} class="text-[var(--accent)]" />
    <span>Lägg i Samlingsfack (Stash)</span>
  </button>

  <div class="h-px my-1 bg-[var(--border)]"></div>

  {#if !item.is_dir}
    <!-- Smart contextual primary action based on file type -->
    {#if isTable}
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-emerald-600 hover:text-[var(--text-primary)] text-left transition-colors font-medium text-emerald-400 cursor-pointer"
        on:click={() => handleOpenWith('Microsoft Excel')}
        title="Öppna i Microsoft Excel"
      >
        <div class="flex items-center gap-2 min-w-0">
          <Table size={13} class="text-emerald-400 shrink-0" />
          <span class="truncate font-semibold">Öppna i Excel</span>
        </div>
        {#if isSSH}
          <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-emerald-950/80 border border-emerald-700/60 text-emerald-300">
            SSH ➔ Mac
          </span>
        {/if}
      </button>
    {:else if isDocx}
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-blue-600 hover:text-[var(--text-primary)] text-left transition-colors font-medium text-blue-400 cursor-pointer"
        on:click={() => handleOpenWith('Microsoft Word')}
        title="Öppna i Microsoft Word"
      >
        <div class="flex items-center gap-2 min-w-0">
          <FileText size={13} class="text-blue-400 shrink-0" />
          <span class="truncate font-semibold">Öppna i Word</span>
        </div>
        {#if isSSH}
          <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-blue-950/80 border border-blue-700/60 text-blue-300">
            SSH ➔ Mac
          </span>
        {/if}
      </button>
    {:else if isPdf}
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-rose-600 hover:text-[var(--text-primary)] text-left transition-colors font-medium text-rose-400 cursor-pointer"
        on:click={handleOpen}
        title="Öppna PDF"
      >
        <div class="flex items-center gap-2 min-w-0">
          <FileText size={13} class="text-rose-400 shrink-0" />
          <span class="truncate font-semibold">Öppna PDF</span>
        </div>
        {#if isSSH}
          <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-rose-950/80 border border-rose-700/60 text-rose-300">
            SSH ➔ Mac
          </span>
        {/if}
      </button>
    {:else if isCode}
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-sky-600 hover:text-[var(--text-primary)] text-left transition-colors font-medium text-sky-400 cursor-pointer"
        on:click={() => handleOpenInEditor()}
        title="Öppna filen i {$favoriteEditor} (Kortkommando: ⌘E)"
      >
        <div class="flex items-center gap-2 min-w-0">
          <Code size={13} class="text-sky-400 shrink-0" />
          <span class="truncate font-semibold">Öppna i {$favoriteEditor}</span>
        </div>
        <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-sky-950/80 border border-sky-700/60 text-sky-300 shrink-0">
          ⌘E
        </span>
      </button>
    {:else}
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--accent)] hover:text-[var(--text-primary)] text-left transition-colors font-medium text-[var(--text-primary)] cursor-pointer"
        on:click={handleOpen}
        title="Öppna filen"
      >
        <div class="flex items-center gap-2 min-w-0">
          <ExternalLink size={13} class="shrink-0" />
          <span class="truncate font-semibold">Öppna</span>
        </div>
        {#if isSSH}
          <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-amber-950/80 border border-amber-700/60 text-amber-300">
            SSH ➔ Mac
          </span>
        {/if}
      </button>
    {/if}

    <!-- Open with submenu -->
    <div class="relative">
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--bg-hover)] text-left transition-colors text-[var(--text-primary)] cursor-pointer"
        on:click={() => (isOpenWithSubmenu = !isOpenWithSubmenu)}
      >
        <div class="flex items-center gap-2 min-w-0">
          <ExternalLink size={13} class="text-[var(--text-secondary)] shrink-0" />
          <span>Öppna med...</span>
        </div>
        <ChevronRight size={12} class="text-[var(--text-secondary)] {isOpenWithSubmenu ? 'rotate-90' : ''} transition-transform" />
      </button>

      {#if isOpenWithSubmenu}
        <div class="bg-[var(--bg-panel)] border-y border-[var(--border)] py-1 pl-3 pr-2 text-[11px] flex flex-col gap-0.5">
          {#if isTable}
            <div class="px-2 py-0.5 text-[9.5px] font-bold text-[var(--text-muted)] uppercase tracking-wider">Kalkylark</div>
            <button
              class="flex items-center justify-between px-2 py-1 rounded hover:bg-emerald-600 hover:text-[var(--text-primary)] text-emerald-400 text-left transition-colors cursor-pointer"
              on:click={() => handleOpenWith('Microsoft Excel')}
            >
              <div class="flex items-center gap-2">
                <Table size={12} />
                <span>Microsoft Excel</span>
              </div>
              <span class="text-[9px] font-mono opacity-70">Excel</span>
            </button>
            <button
              class="flex items-center justify-between px-2 py-1 rounded hover:bg-emerald-700 hover:text-[var(--text-primary)] text-emerald-300 text-left transition-colors cursor-pointer"
              on:click={() => handleOpenWith('Numbers')}
            >
              <div class="flex items-center gap-2">
                <Table size={12} />
                <span>Apple Numbers</span>
              </div>
              <span class="text-[9px] font-mono opacity-70">Numbers</span>
            </button>
            <button
              class="flex items-center justify-between px-2 py-1 rounded hover:bg-slate-700 hover:text-[var(--text-primary)] text-[var(--text-primary)] text-left transition-colors cursor-pointer"
              on:click={() => handleOpenWith(undefined)}
            >
              <div class="flex items-center gap-2">
                <ExternalLink size={12} />
                <span>Standardprogram</span>
              </div>
              <span class="text-[9px] font-mono opacity-70">Standard</span>
            </button>
            <div class="h-px my-1 bg-[var(--border)]"></div>
            <div class="px-2 py-0.5 text-[9.5px] font-bold text-[var(--text-muted)] uppercase tracking-wider">Text / Rådata</div>
            <button
              class="flex items-center justify-between px-2 py-1 rounded hover:bg-sky-600 hover:text-[var(--text-primary)] text-[var(--text-primary)] text-left transition-colors cursor-pointer"
              on:click={() => handleOpenInEditor('Visual Studio Code')}
            >
              <div class="flex items-center gap-2 min-w-0">
                <Code size={12} class="text-sky-400 shrink-0" />
                <span class="truncate">Visual Studio Code</span>
              </div>
              <span class="text-[9px] font-mono opacity-60 shrink-0">VSCode</span>
            </button>
          {:else if isDocx}
            <div class="px-2 py-0.5 text-[9.5px] font-bold text-[var(--text-muted)] uppercase tracking-wider">Dokument</div>
            <button
              class="flex items-center justify-between px-2 py-1 rounded hover:bg-blue-600 hover:text-[var(--text-primary)] text-blue-400 text-left transition-colors cursor-pointer"
              on:click={() => handleOpenWith('Microsoft Word')}
            >
              <div class="flex items-center gap-2">
                <FileText size={12} />
                <span>Microsoft Word</span>
              </div>
              <span class="text-[9px] font-mono opacity-70">Word</span>
            </button>
            <button
              class="flex items-center justify-between px-2 py-1 rounded hover:bg-slate-700 hover:text-[var(--text-primary)] text-[var(--text-primary)] text-left transition-colors cursor-pointer"
              on:click={() => handleOpenWith(undefined)}
            >
              <div class="flex items-center gap-2">
                <ExternalLink size={12} />
                <span>Standardprogram</span>
              </div>
              <span class="text-[9px] font-mono opacity-70">Standard</span>
            </button>
          {:else}
            <div class="px-2 py-0.5 text-[9.5px] font-bold text-[var(--text-muted)] uppercase tracking-wider">Editorer</div>
            {#each SUPPORTED_EDITORS as ed}
              <button
                class="flex items-center justify-between px-2 py-1 rounded hover:bg-sky-600 hover:text-[var(--text-primary)] text-[var(--text-primary)] text-left transition-colors cursor-pointer"
                on:click={() => handleOpenInEditor(ed.appName)}
                title="Öppna och sätt {ed.name} som standard (⌘E)"
              >
                <div class="flex items-center gap-2 min-w-0">
                  <Code size={12} class="text-sky-400 shrink-0" />
                  <span class="truncate">{ed.name}</span>
                  {#if $favoriteEditor === ed.appName}
                    <span class="text-[8.5px] text-amber-400 font-bold shrink-0">★ Standard</span>
                  {/if}
                </div>
                <span class="text-[9px] font-mono opacity-60 shrink-0">{ed.badge}</span>
              </button>
            {/each}
            <div class="h-px my-1 bg-[var(--border)]"></div>
            <button
              class="flex items-center justify-between px-2 py-1 rounded hover:bg-amber-600 hover:text-[var(--text-primary)] text-amber-400 text-left transition-colors cursor-pointer"
              on:click={() => handleOpenWith(undefined)}
            >
              <div class="flex items-center gap-2">
                <ExternalLink size={12} />
                <span>Standardprogram</span>
              </div>
              <span class="text-[9px] font-mono opacity-70">Default</span>
            </button>
          {/if}
        </div>
      {/if}
    </div>
  {:else}
    <!-- Directory actions -->
    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-sky-600 hover:text-[var(--text-primary)] text-left transition-colors font-medium text-sky-400 cursor-pointer"
      on:click={() => handleOpenInEditor()}
      title="Öppna projektmappen i {$favoriteEditor} (Kortkommando: ⌘E)"
    >
      <div class="flex items-center gap-2 min-w-0">
        <Code size={13} class="text-sky-400 shrink-0" />
        <span class="truncate font-semibold">Öppna projekt i {$favoriteEditor}</span>
      </div>
      <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-sky-950/80 border border-sky-700/60 text-sky-300 shrink-0">
        ⌘E
      </span>
    </button>
  {/if}

  {#if !isSSH}
    <button
      class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-[var(--text-primary)] text-left"
      on:click={handleReveal}
    >
      <FolderOpen size={13} />
      <span>Reveal in Finder</span>
    </button>
  {/if}

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-[var(--text-primary)] text-left"
    on:click={handleCopyPath}
  >
    <Copy size={13} />
    <span>Copy Path</span>
  </button>

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-[var(--text-primary)] text-left"
    on:click={handleOpenInTerminal}
  >
    <TerminalIcon size={13} />
    <span>Open in Terminal</span>
  </button>

  {#if item.is_dir}
    <button
      class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-[var(--text-primary)] text-left"
      on:click={handleDu}
    >
      <PieChart size={13} />
      <span>Calculate Folder Size (du)</span>
    </button>
  {/if}

  <div class="h-px my-1 bg-[var(--border)]"></div>

  <button
    class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-red-600 hover:text-[var(--text-primary)] text-red-400 text-left transition-colors"
    on:click={handleTrash}
  >
    <div class="flex items-center gap-2 min-w-0">
      <Trash2 size={13} class="shrink-0" />
      <span class="truncate">{isMultiSelect ? `Flytta ${multiCount} filer till Papperskorg` : 'Flytta till Papperskorg'}</span>
    </div>
    <kbd class="text-[9px] font-mono opacity-80 shrink-0 ml-1">⌘⌫</kbd>
  </button>
</div>

<RelatedBamsModal
  referenceName={item.name}
  result={relatedResult}
  isLoading={isFindingRelated}
  error={relatedError}
  onOpenPath={openRelated}
  onClose={() => { relatedResult = null; relatedError = ''; onClose(); }}
/>

<!-- rs-qc Modal Output -->
{#if qcResultModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-6">
    <div class="w-[680px] max-h-[80vh] flex flex-col bg-[var(--bg-surface)] border border-[var(--border)] rounded-2xl shadow-2xl overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 bg-[var(--bg-panel)] border-b border-[var(--border)]">
        <div class="flex items-center gap-2 text-purple-400 font-bold text-sm">
          <Activity size={16} />
          <span>rs-qc Alignment QC: {item.name}</span>
        </div>
        <button
          class="p-1 rounded hover:bg-white/10 text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
          on:click={() => { qcResultModal = ''; onClose(); }}
        >
          <X size={16} />
        </button>
      </div>
      <div class="flex-1 overflow-auto p-4 font-mono text-xs text-[var(--text-primary)] bg-[var(--bg-base)] leading-relaxed select-text">
        <pre class="m-0 whitespace-pre-wrap">{qcResultModal}</pre>
      </div>
      <div class="px-4 py-2.5 bg-[var(--bg-panel)] border-t border-[var(--border)] flex justify-end">
        <button
          class="px-4 py-1.5 rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-xs font-semibold"
          on:click={() => { qcResultModal = ''; onClose(); }}
        >
          Klar
        </button>
      </div>
    </div>
  </div>
{/if}
