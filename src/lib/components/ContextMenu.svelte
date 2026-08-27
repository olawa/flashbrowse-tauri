<script lang="ts">
  import { openInDefault, revealInOs, trashItems, calculateDirSize } from '../invoke';
  import { executeTerminalCommand } from '../stores/terminal';
  import { refreshPane } from '../stores/navigation';
  import type { FileItem } from '../types';
  import {
    ExternalLink,
    FolderOpen,
    Copy,
    Trash2,
    Terminal as TerminalIcon,
    PieChart,
  } from 'lucide-svelte';

  export let item: FileItem;
  export let paneId: 'left' | 'right';
  export let x = 0;
  export let y = 0;
  export let onClose: () => void;

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
</script>

<div
  class="fixed z-50 w-52 py-1 bg-[var(--bg-surface)] border border-[var(--border)] rounded-md shadow-2xl text-xs text-[var(--text-primary)] select-none backdrop-blur-md"
  style="top: {y}px; left: {x}px;"
  on:click|stopPropagation
>
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
