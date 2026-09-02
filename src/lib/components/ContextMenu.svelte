<script lang="ts">
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
  import { executeTerminalCommand } from '../stores/terminal';
  import { refreshPane, leftPane, rightPane, transferBetweenPanes, isDualPane } from '../stores/navigation';
  import { addToStash } from '../stores/stash';
  import { castToSecondaryInspector } from '../stores/navigation';
  import { addTracksToHub, isGenomicsHubOpen } from '../stores/genomicsStore';
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
  } from 'lucide-svelte';
  import { saveRemoteOrLocalItem, downloadDirectory, saveNotification } from '../stores/downloadStore';

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

  const ext = item.extension.toLowerCase();
  const isBamOrCram = ext === 'bam' || ext === 'cram' || item.name.endsWith('.bam') || item.name.endsWith('.cram');
  const isGenomics = isBamOrCram || ['vcf', 'bcf', 'bed', 'bw', 'bigwig'].includes(ext) || item.name.endsWith('.vcf.gz');

  const isTable = ['xlsx', 'xls', 'csv', 'tsv', 'tab', 'ods'].includes(ext) || item.name.endsWith('.csv.gz') || item.name.endsWith('.tsv.gz');
  const isDoc = ['docx', 'doc', 'rtf', 'odt', 'txt', 'pages', 'pdf'].includes(ext);
  const isCode = ['py', 'rs', 'sh', 'json', 'yaml', 'yml', 'toml', 'md', 'c', 'cpp', 'h', 'swift', 'js', 'ts', 'r', 'smk'].includes(ext);

  async function handleOpenWith(appName?: string) {
    onClose();
    if (item.is_dir) {
      if (!isSSH) {
        await openFileWith(item.path, appName);
      }
      return;
    }

    if (isSSH) {
      const appDisplay = appName || 'standardprogram';
      saveNotification.set({
        text: `⬇️ Hämtar ${item.name} för att öppna i ${appDisplay}...`,
        success: true,
      });
      try {
        const localPath = await sshOpenFileLocally(sshHost, remotePath, appName);
        saveNotification.set({
          text: `🚀 Öppnade ${item.name} lokalt (${appDisplay})`,
          path: localPath,
          success: true,
        });
        setTimeout(() => saveNotification.set(null), 4000);
      } catch (err: any) {
        saveNotification.set({
          text: `❌ Kunde inte öppna lokalt: ${err}`,
          success: false,
        });
        setTimeout(() => saveNotification.set(null), 5000);
      }
    } else {
      try {
        await openFileWith(item.path, appName);
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

  async function handleCopyPath() {
    try {
      await navigator.clipboard.writeText(item.path);
    } catch (err) {
      console.warn('Clipboard write failed:', err);
    }
    onClose();
  }

  async function handleOpenInTerminal() {
    const dir = item.is_dir ? item.path : item.path.substring(0, item.path.lastIndexOf('/'));
    await executeTerminalCommand(`cd '${dir}'`);
    onClose();
  }

  async function handleTrash() {
    await trashItems([item.path]);
    await refreshPane(paneId);
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

  async function handleRsQc() {
    try {
      qcResultModal = await runRsQc(item.path);
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
    await transferBetweenPanes(paneId, otherPane, [item.path]);
    onClose();
  }

  async function handleSaveToDownloads() {
    const store = paneId === 'left' ? $leftPane : $rightPane;
    await saveRemoteOrLocalItem(store.isSSH, store.sshHost, item.path);
    onClose();
  }
</script>

<div
  class="fixed z-50 w-56 py-1 bg-[var(--bg-surface)] border border-[var(--border)] rounded-md shadow-2xl text-xs text-[var(--text-primary)] select-none backdrop-blur-md"
  style="top: {y}px; left: {x}px;"
  on:click|stopPropagation
>
  <!-- Save permanently to Downloads -->
  <button
    class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-emerald-600 hover:text-white text-left transition-colors font-medium text-emerald-400"
    on:click={handleSaveToDownloads}
    title="Spara permanent lokal kopia till {$downloadDirectory || '~/Downloads'}"
  >
    <div class="flex items-center gap-2 min-w-0">
      <Download size={13} class="text-emerald-400 shrink-0" />
      <span class="truncate">Spara till Downloads</span>
    </div>
    <span class="text-[9px] font-mono opacity-70">Lokal</span>
  </button>

  <!-- Transfer to other pane -->
  {#if $isDualPane}
    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-cyan-600 hover:text-white text-left transition-colors font-medium text-cyan-400"
      on:click={handleTransferToOtherPane}
      title="Överför/kopiera till motsatt panel"
    >
      <div class="flex items-center gap-2 min-w-0">
        <ArrowRightLeft size={13} class="text-cyan-400 shrink-0" />
        <span class="truncate">Överför till andra panelen</span>
      </div>
      <kbd class="text-[9px] font-mono opacity-70">F5</kbd>
    </button>
  {/if}

  <!-- Select all of same type -->
  <button
    class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left transition-colors font-medium text-emerald-400 hover:text-white"
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
    class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-orange-600 hover:text-white text-left transition-colors font-medium text-orange-400 hover:text-white"
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
      class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-purple-600 hover:text-white text-purple-400 font-medium text-left transition-colors"
      on:click={handleRsQc}
    >
      <Activity size={13} />
      <span>Kör rs-qc (Alignment QC)</span>
    </button>
  {/if}

  {#if isGenomics}
    <button
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-emerald-600 hover:text-white text-emerald-400 font-medium text-left transition-colors"
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
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-blue-600 hover:text-white text-blue-400 font-medium text-left transition-colors"
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
      class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[#273144] text-emerald-300 font-medium text-left transition-colors"
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
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-amber-600 hover:text-white text-left text-amber-400 font-medium transition-colors"
    on:click={handleCast}
  >
    <Rocket size={13} />
    <span>Kasta till Stort Fönster (Swipe ↑)</span>
  </button>

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
    on:click={handleAddToStash}
  >
    <Layers size={13} class="text-[var(--accent)]" />
    <span>Lägg i Samlingsfack (Stash)</span>
  </button>

  <div class="h-px my-1 bg-[var(--border)]"></div>

  {#if !item.is_dir}
    <!-- Smart primary action depending on file type -->
    {#if isTable}
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-emerald-600 hover:text-white text-left transition-colors font-medium text-emerald-400"
        on:click={() => handleOpenWith('Microsoft Excel')}
        title="Öppna i Microsoft Excel lokalt på din Mac"
      >
        <div class="flex items-center gap-2 min-w-0">
          <Table size={13} class="text-emerald-400 shrink-0" />
          <span class="truncate font-semibold">Öppna i Excel</span>
        </div>
        <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-emerald-950/80 border border-emerald-700/60 text-emerald-300">
          {isSSH ? 'SSH ➔ Mac' : 'Lokalt'}
        </span>
      </button>
    {:else if isDoc}
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-blue-600 hover:text-white text-left transition-colors font-medium text-blue-400"
        on:click={() => handleOpenWith('Microsoft Word')}
        title="Öppna i Microsoft Word lokalt på din Mac"
      >
        <div class="flex items-center gap-2 min-w-0">
          <FileText size={13} class="text-blue-400 shrink-0" />
          <span class="truncate font-semibold">Öppna i Word</span>
        </div>
        <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-blue-950/80 border border-blue-700/60 text-blue-300">
          {isSSH ? 'SSH ➔ Mac' : 'Lokalt'}
        </span>
      </button>
    {:else if isCode}
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-sky-600 hover:text-white text-left transition-colors font-medium text-sky-400"
        on:click={() => handleOpenWith('Visual Studio Code')}
        title="Öppna i Visual Studio Code lokalt på din Mac"
      >
        <div class="flex items-center gap-2 min-w-0">
          <Code size={13} class="text-sky-400 shrink-0" />
          <span class="truncate font-semibold">Öppna i VS Code</span>
        </div>
        <span class="text-[9px] font-mono px-1.5 py-0.2 rounded bg-sky-950/80 border border-sky-700/60 text-sky-300">
          {isSSH ? 'SSH ➔ Mac' : 'Lokalt'}
        </span>
      </button>
    {/if}

    <!-- Open with submenu -->
    <div class="relative">
      <button
        class="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--bg-hover)] text-left transition-colors text-slate-200"
        on:click={() => (isOpenWithSubmenu = !isOpenWithSubmenu)}
      >
        <div class="flex items-center gap-2 min-w-0">
          <ExternalLink size={13} class="text-amber-400 shrink-0" />
          <span>Öppna lokalt med...</span>
        </div>
        <ChevronRight size={12} class="text-slate-400 {isOpenWithSubmenu ? 'rotate-90' : ''} transition-transform" />
      </button>

      {#if isOpenWithSubmenu}
        <div class="bg-[var(--bg-panel)] border-y border-[var(--border)] py-1 pl-4 pr-2 text-[11px] flex flex-col gap-0.5">
          <button
            class="flex items-center justify-between px-2 py-1 rounded hover:bg-emerald-600 hover:text-white text-emerald-400 text-left transition-colors"
            on:click={() => handleOpenWith('Microsoft Excel')}
          >
            <div class="flex items-center gap-2">
              <Table size={12} />
              <span>Microsoft Excel</span>
            </div>
            <span class="text-[9px] font-mono opacity-70">Excel</span>
          </button>
          <button
            class="flex items-center justify-between px-2 py-1 rounded hover:bg-blue-600 hover:text-white text-blue-400 text-left transition-colors"
            on:click={() => handleOpenWith('Microsoft Word')}
          >
            <div class="flex items-center gap-2">
              <FileText size={12} />
              <span>Microsoft Word</span>
            </div>
            <span class="text-[9px] font-mono opacity-70">Word</span>
          </button>
          <button
            class="flex items-center justify-between px-2 py-1 rounded hover:bg-sky-600 hover:text-white text-sky-400 text-left transition-colors"
            on:click={() => handleOpenWith('Visual Studio Code')}
          >
            <div class="flex items-center gap-2">
              <Code size={12} />
              <span>Visual Studio Code</span>
            </div>
            <span class="text-[9px] font-mono opacity-70">VSCode</span>
          </button>
          <button
            class="flex items-center justify-between px-2 py-1 rounded hover:bg-slate-700 hover:text-white text-slate-300 text-left transition-colors"
            on:click={() => handleOpenWith('TextEdit')}
          >
            <div class="flex items-center gap-2">
              <FileText size={12} />
              <span>TextEdit</span>
            </div>
            <span class="text-[9px] font-mono opacity-70">Editor</span>
          </button>
          <button
            class="flex items-center justify-between px-2 py-1 rounded hover:bg-amber-600 hover:text-white text-amber-400 text-left transition-colors"
            on:click={() => handleOpenWith(undefined)}
          >
            <div class="flex items-center gap-2">
              <ExternalLink size={12} />
              <span>Standardprogram</span>
            </div>
            <span class="text-[9px] font-mono opacity-70">Default</span>
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
    on:click={handleOpen}
  >
    <ExternalLink size={13} />
    <span>{isSSH && !item.is_dir ? 'Öppna lokalt (Standard)' : 'Öppna'}</span>
  </button>

  {#if !isSSH}
    <button
      class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
      on:click={handleReveal}
    >
      <FolderOpen size={13} />
      <span>Reveal in Finder</span>
    </button>
  {/if}

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
    on:click={handleCopyPath}
  >
    <Copy size={13} />
    <span>Copy Path</span>
  </button>

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
    on:click={handleOpenInTerminal}
  >
    <TerminalIcon size={13} />
    <span>Open in Terminal</span>
  </button>

  {#if item.is_dir}
    <button
      class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
      on:click={handleDu}
    >
      <PieChart size={13} />
      <span>Calculate Folder Size (du)</span>
    </button>
  {/if}

  <div class="h-px my-1 bg-[var(--border)]"></div>

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-red-600 hover:text-white text-red-400 text-left"
    on:click={handleTrash}
  >
    <Trash2 size={13} />
    <span>Move to Trash</span>
  </button>
</div>

<!-- rs-qc Modal Output -->
{#if qcResultModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-6">
    <div class="w-[680px] max-h-[80vh] flex flex-col bg-[#14171d] border border-[#262d3d] rounded-2xl shadow-2xl overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 bg-[#191d24] border-b border-[#262d3d]">
        <div class="flex items-center gap-2 text-purple-400 font-bold text-sm">
          <Activity size={16} />
          <span>rs-qc Alignment QC: {item.name}</span>
        </div>
        <button
          class="p-1 rounded hover:bg-white/10 text-slate-400 hover:text-white"
          on:click={() => { qcResultModal = ''; onClose(); }}
        >
          <X size={16} />
        </button>
      </div>
      <div class="flex-1 overflow-auto p-4 font-mono text-xs text-slate-200 bg-[#0c0d10] leading-relaxed select-text">
        <pre class="m-0 whitespace-pre-wrap">{qcResultModal}</pre>
      </div>
      <div class="px-4 py-2.5 bg-[#191d24] border-t border-[#262d3d] flex justify-end">
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
