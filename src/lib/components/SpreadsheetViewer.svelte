<script lang="ts">
  import {
    Table,
    Search,
    Download,
    Copy,
    Check,
    Layers,
    X,
    Filter,
  } from 'lucide-svelte';

  export let headers: string[] = [];
  export let rows: string[][] = [];
  export let sheetNames: string[] = [];
  export let filename: string = '';
  export let formattedSize: string = '--';

  let tableFilter = '';
  let copied = false;
  let activeSheet = sheetNames.length > 0 ? sheetNames[0] : 'Blad 1';

  $: filteredRows = tableFilter.trim()
    ? rows.filter((row) =>
        row.some((cell) => cell.toLowerCase().includes(tableFilter.toLowerCase()))
      )
    : rows;

  async function copyTableAsTSV() {
    if (headers.length === 0 && rows.length === 0) return;
    const tsvContent = [
      headers.join('\t'),
      ...filteredRows.map((r) => r.join('\t')),
    ].join('\n');

    try {
      await navigator.clipboard.writeText(tsvContent);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (err) {
      console.warn('Copy failed:', err);
    }
  }
</script>

<div class="flex flex-col h-full bg-[var(--bg-surface)] text-[var(--text-primary)] select-text overflow-hidden font-sans">
  <!-- Top Table Toolbar -->
  <div class="flex items-center justify-between px-3 py-1.5 bg-[var(--bg-panel)] border-b border-[var(--border)] shrink-0 text-xs select-none">
    <div class="flex items-center gap-2 flex-1 max-w-md">
      <div class="relative flex items-center w-full">
        <Filter size={12} class="absolute left-2 text-[var(--text-secondary)]" />
        <input
          type="text"
          placeholder="Filtra rader i tabell..."
          bind:value={tableFilter}
          class="w-full bg-[var(--bg-surface)] text-[var(--text-primary)] pl-7 pr-7 py-1 rounded text-xs border border-[var(--border)] focus:outline-none focus:border-cyan-400 font-mono"
        />
        {#if tableFilter}
          <button
            class="absolute right-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
            on:click={() => (tableFilter = '')}
          >
            <X size={12} />
          </button>
        {/if}
      </div>

      <span class="text-[10px] text-[var(--text-secondary)] font-mono shrink-0">
        {filteredRows.length} {filteredRows.length === 1 ? 'rad' : 'rader'}
      </span>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-1.5 ml-2">
      <!-- Sheet Tabs if available -->
      {#if sheetNames && sheetNames.length > 1}
        <div class="flex items-center gap-1 bg-[var(--bg-surface)] p-0.5 rounded border border-[var(--border)]">
          {#each sheetNames as sheet}
            <button
              class="px-2 py-0.5 rounded text-[10px] font-medium transition-colors {activeSheet === sheet ? 'bg-cyan-500 text-black font-bold' : 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'}"
              on:click={() => (activeSheet = sheet)}
            >
              {sheet}
            </button>
          {/each}
        </div>
      {/if}

      <!-- Copy Table Button -->
      <button
        class="flex items-center gap-1 px-2.5 py-1 rounded bg-[var(--bg-active)] hover:bg-[var(--bg-active)] text-[var(--text-primary)] hover:text-[var(--text-primary)] border border-[var(--border)] transition-colors text-xs"
        on:click={copyTableAsTSV}
        title="Kopiera filtrerad tabell som TSV"
      >
        {#if copied}
          <Check size={12} class="text-green-400" />
          <span class="text-green-400 font-semibold">Kopierad!</span>
        {:else}
          <Copy size={12} />
          <span>Kopiera tabell</span>
        {/if}
      </button>
    </div>
  </div>

  <!-- Spreadsheet Table Container -->
  <div class="flex-1 overflow-auto bg-[var(--bg-base)] relative">
    {#if headers.length === 0 && filteredRows.length === 0}
      <div class="h-full flex items-center justify-center text-[var(--text-muted)] text-xs">
        Tabellen är tom eller kunde inte tolkas.
      </div>
    {:else}
      <table class="w-full text-left border-collapse text-[11px] font-mono">
        <!-- Sticky Header Row -->
        <thead class="sticky top-0 z-10 bg-[var(--bg-panel)] shadow-sm">
          <tr class="border-b border-[var(--border)]">
            <!-- Row # Column Header -->
            <th class="py-1.5 px-2.5 w-12 text-right text-[var(--text-muted)] bg-[var(--bg-surface)] border-r border-[var(--border)] select-none text-[10px]">
              #
            </th>
            {#each headers as header, hIdx}
              <th class="py-1.5 px-3 font-semibold text-cyan-300 border-r border-[var(--border)]/50 whitespace-nowrap bg-[var(--bg-panel)]">
                {header || `Kolumn ${hIdx + 1}`}
              </th>
            {/each}
          </tr>
        </thead>

        <!-- Table Rows -->
        <tbody class="divide-y divide-[#202533]/60">
          {#each filteredRows as row, rIdx}
            <tr class="hover:bg-white/[0.04] transition-colors {rIdx % 2 === 0 ? 'bg-[var(--bg-base)]' : 'bg-[var(--bg-surface)]'}">
              <!-- Row Number -->
              <td class="py-1 px-2.5 text-right text-[var(--text-muted)] bg-[var(--bg-surface)]/60 border-r border-[var(--border)] select-none text-[10px]">
                {rIdx + 1}
              </td>

              <!-- Cells -->
              {#each row as cell}
                <td class="py-1 px-3 border-r border-[var(--border)]/40 whitespace-nowrap text-[var(--text-primary)] truncate max-w-xs">
                  {cell}
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
