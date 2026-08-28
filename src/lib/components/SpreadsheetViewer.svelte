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

<div class="flex flex-col h-full bg-[#12151c] text-slate-200 select-text overflow-hidden font-sans">
  <!-- Top Table Toolbar -->
  <div class="flex items-center justify-between px-3 py-1.5 bg-[#171b24] border-b border-[#252d3d] shrink-0 text-xs select-none">
    <div class="flex items-center gap-2 flex-1 max-w-md">
      <div class="relative flex items-center w-full">
        <Filter size={12} class="absolute left-2 text-slate-400" />
        <input
          type="text"
          placeholder="Filtra rader i tabell..."
          bind:value={tableFilter}
          class="w-full bg-[#12151c] text-white pl-7 pr-7 py-1 rounded text-xs border border-[#2c3547] focus:outline-none focus:border-cyan-400 font-mono"
        />
        {#if tableFilter}
          <button
            class="absolute right-2 text-slate-400 hover:text-white"
            on:click={() => (tableFilter = '')}
          >
            <X size={12} />
          </button>
        {/if}
      </div>

      <span class="text-[10px] text-slate-400 font-mono shrink-0">
        {filteredRows.length} {filteredRows.length === 1 ? 'rad' : 'rader'}
      </span>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-1.5 ml-2">
      <!-- Sheet Tabs if available -->
      {#if sheetNames && sheetNames.length > 1}
        <div class="flex items-center gap-1 bg-[#12151c] p-0.5 rounded border border-[#252d3d]">
          {#each sheetNames as sheet}
            <button
              class="px-2 py-0.5 rounded text-[10px] font-medium transition-colors {activeSheet === sheet ? 'bg-cyan-500 text-black font-bold' : 'text-slate-400 hover:text-white'}"
              on:click={() => (activeSheet = sheet)}
            >
              {sheet}
            </button>
          {/each}
        </div>
      {/if}

      <!-- Copy Table Button -->
      <button
        class="flex items-center gap-1 px-2.5 py-1 rounded bg-[#202634] hover:bg-[#2c3547] text-slate-300 hover:text-white border border-[#2c3547] transition-colors text-xs"
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
  <div class="flex-1 overflow-auto bg-[#0d1017] relative">
    {#if headers.length === 0 && filteredRows.length === 0}
      <div class="h-full flex items-center justify-center text-slate-500 text-xs">
        Tabellen är tom eller kunde inte tolkas.
      </div>
    {:else}
      <table class="w-full text-left border-collapse text-[11px] font-mono">
        <!-- Sticky Header Row -->
        <thead class="sticky top-0 z-10 bg-[#161a24] shadow-sm">
          <tr class="border-b border-[#252d3d]">
            <!-- Row # Column Header -->
            <th class="py-1.5 px-2.5 w-12 text-right text-slate-500 bg-[#141720] border-r border-[#202533] select-none text-[10px]">
              #
            </th>
            {#each headers as header, hIdx}
              <th class="py-1.5 px-3 font-semibold text-cyan-300 border-r border-[#252d3d]/50 whitespace-nowrap bg-[#161a24]">
                {header || `Kolumn ${hIdx + 1}`}
              </th>
            {/each}
          </tr>
        </thead>

        <!-- Table Rows -->
        <tbody class="divide-y divide-[#202533]/60">
          {#each filteredRows as row, rIdx}
            <tr class="hover:bg-white/[0.04] transition-colors {rIdx % 2 === 0 ? 'bg-[#0d1017]' : 'bg-[#11141c]'}">
              <!-- Row Number -->
              <td class="py-1 px-2.5 text-right text-slate-500 bg-[#12151c]/60 border-r border-[#202533] select-none text-[10px]">
                {rIdx + 1}
              </td>

              <!-- Cells -->
              {#each row as cell}
                <td class="py-1 px-3 border-r border-[#202533]/40 whitespace-nowrap text-slate-200 truncate max-w-xs">
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
