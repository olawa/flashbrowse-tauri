<script lang="ts">
  import { openInDefault, revealInOs, trashItems, calculateDirSize, launchRsnap, runRsQc } from '../invoke';
  import { executeTerminalCommand } from '../stores/terminal';
  import { refreshPane } from '../stores/navigation';
  import { addToStash } from '../stores/stash';
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
    X,
  } from 'lucide-svelte';

  export let item: FileItem;
  export let paneId: 'left' | 'right';
  export let x = 0;
  export let y = 0;
  export let onClose: () => void;

  let qcResultModal = '';

  const ext = item.extension.toLowerCase();
  const isBamOrCram = ext === 'bam' || ext === 'cram' || item.name.endsWith('.bam') || item.name.endsWith('.cram');
  const isGenomics = isBamOrCram || ['vcf', 'bcf', 'bed', 'bw', 'bigwig'].includes(ext) || item.name.endsWith('.vcf.gz');

  async function handleOpen() {
    await openInDefault(item.path);
    onClose();
  }

  async function handleReveal() {
    await revealInOs(item.path);
    onClose();
  }

  async function handleCopyPath() {
    await navigator.clipboard.writeText(item.path);
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
      await launchRsnap([item.path]);
    } catch (e: any) {
      alert(`rsnap fel: ${e}`);
    }
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
</script>

<div
  class="fixed z-50 w-56 py-1 bg-[var(--bg-surface)] border border-[var(--border)] rounded-md shadow-2xl text-xs text-[var(--text-primary)] select-none backdrop-blur-md"
  style="top: {y}px; left: {x}px;"
  on:click|stopPropagation
>
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
      class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-emerald-600 hover:text-white text-emerald-400 font-medium text-left transition-colors"
      on:click={handleRsnap}
    >
      <Dna size={13} />
      <span>Öppna i rsnap</span>
    </button>
    <div class="h-px my-1 bg-[var(--border)]"></div>
  {/if}

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
    on:click={handleAddToStash}
  >
    <Layers size={13} class="text-[var(--accent)]" />
    <span>Lägg i Samlingsfack (Stash)</span>
  </button>

  <div class="h-px my-1 bg-[var(--border)]"></div>

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
    on:click={handleOpen}
  >
    <ExternalLink size={13} />
    <span>Open</span>
  </button>

  <button
    class="w-full flex items-center gap-2 px-3 py-1.5 hover:bg-[var(--accent)] hover:text-white text-left"
    on:click={handleReveal}
  >
    <FolderOpen size={13} />
    <span>Reveal in Finder</span>
  </button>

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
