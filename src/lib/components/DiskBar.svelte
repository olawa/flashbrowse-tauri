<script lang="ts">
  import { onMount } from 'svelte';
  import { getDiskInfo } from '../invoke';
  import type { DiskInfo } from '../types';
  import { HardDrive } from 'lucide-svelte';

  export let path = '/';
  let disk: DiskInfo | null = null;
  let error = '';

  async function loadDiskInfo() {
    try {
      disk = await getDiskInfo(path);
    } catch (e: any) {
      error = String(e);
    }
  }

  onMount(() => {
    loadDiskInfo();
    const interval = setInterval(loadDiskInfo, 10000);
    return () => clearInterval(interval);
  });
</script>

<div class="px-3 py-2 border-t border-[var(--border)] bg-[var(--bg-surface)] text-[11px] text-[var(--text-secondary)]">
  {#if disk}
    <div class="flex items-center justify-between mb-1">
      <div class="flex items-center gap-1 font-medium text-[var(--text-primary)]">
        <HardDrive size={12} class="text-[var(--accent)]" />
        <span>Disk Usage</span>
      </div>
      <span>{disk.formatted_available} free</span>
    </div>
    
    <!-- Progress bar -->
    <div class="w-full h-1.5 bg-[var(--border)] rounded-full overflow-hidden">
      <div
        class="h-full bg-[var(--accent)] transition-all duration-300"
        style="width: {Math.min(100, Math.max(0, disk.percentage_used))}%"
      ></div>
    </div>
    
    <div class="flex justify-between mt-1 text-[10px] text-[var(--text-muted)]">
      <span>{disk.formatted_used} used</span>
      <span>{disk.formatted_total} total</span>
    </div>
  {:else if error}
    <span class="text-red-400">{error}</span>
  {:else}
    <span class="opacity-50">Loading disk info...</span>
  {/if}
</div>
