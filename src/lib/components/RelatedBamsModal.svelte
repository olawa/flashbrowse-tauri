<script lang="ts">
  import { GitBranch, X } from 'lucide-svelte';
  import type { RelatedBamResult } from '../invoke';

  /** The file whose relatives are being shown. */
  export let referenceName: string;
  export let result: RelatedBamResult | null = null;
  export let isLoading = false;
  export let error = '';
  /** Called with the path of a related file the user picked. */
  export let onOpenPath: (path: string) => void;
  export let onClose: () => void;
</script>

{#if isLoading || result || error}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-6">
    <div class="w-[720px] max-h-[80vh] flex flex-col bg-[#14171d] border border-[#262d3d] rounded-2xl shadow-2xl overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 bg-[#191d24] border-b border-[#262d3d]">
        <div class="flex items-center gap-2 text-emerald-400 font-bold text-sm">
          <GitBranch size={16} />
          <span>Samma ursprung som {referenceName}</span>
        </div>
        <button
          class="p-1 rounded hover:bg-white/10 text-slate-400 hover:text-white"
          on:click={onClose}
        >
          <X size={16} />
        </button>
      </div>

      <div class="flex-1 overflow-auto p-4 text-xs text-slate-200 bg-[#0c0d10] space-y-3">
        {#if isLoading}
          <div class="flex items-center gap-2 text-slate-400 font-mono">
            <div class="w-3.5 h-3.5 border-2 border-emerald-400 border-t-transparent rounded-full animate-spin"></div>
            <span>Läser BAM-headers…</span>
          </div>
        {:else if error}
          <pre class="m-0 whitespace-pre-wrap text-red-300 font-mono">{error}</pre>
        {:else if result}
          <!-- What we matched on -->
          <div class="p-2.5 rounded-lg bg-[#151922] border border-[#252d3d] font-mono text-[11px] space-y-1">
            {#if result.reference.fastqs.length > 0}
              <div><span class="text-slate-500">FASTQ:</span> {result.reference.fastqs.join(', ')}</div>
            {/if}
            {#if result.reference.source_bams.length > 0}
              <div><span class="text-slate-500">Härledd från:</span> {result.reference.source_bams.join(', ')}</div>
            {/if}
            {#if result.reference.read_groups.length > 0}
              <div><span class="text-slate-500">Läsgrupper:</span> {result.reference.read_groups.slice(0, 4).join(', ')}{result.reference.read_groups.length > 4 ? ` +${result.reference.read_groups.length - 4}` : ''}</div>
            {/if}
            {#if result.reference.sample}
              <div><span class="text-slate-500">Prov:</span> {result.reference.sample}</div>
            {/if}
            {#if result.reference.fastqs.length === 0 && result.reference.source_bams.length === 0 && result.reference.read_groups.length === 0 && !result.reference.sample}
              <div class="text-amber-300">
                Headern innehåller varken @PG-kommandorad eller @RG-läsgrupper — det finns inget att matcha på.
                Det händer när aligneraren körts i en pipe utan att skriva sin @PG-rad.
              </div>
            {/if}
          </div>

          {#if result.related.length === 0}
            <div class="text-slate-400">
              Ingen av de {result.examined} granskade filerna delar ursprung.
            </div>
          {:else}
            <div class="space-y-1.5">
              {#each result.related as rel (rel.path)}
                <button
                  class="w-full text-left p-2.5 rounded-lg bg-[#151922] border border-[#252d3d] hover:border-emerald-600 transition-colors"
                  on:click={() => onOpenPath(rel.path)}
                  title="Öppna mappen som innehåller {rel.name}"
                >
                  <div class="flex items-center justify-between gap-2">
                    <span class="font-mono font-semibold text-emerald-300 truncate">{rel.name}</span>
                    <span class="font-mono text-[10px] text-slate-500 shrink-0">{rel.formatted_size}</span>
                  </div>
                  <div class="text-[10px] text-slate-500 font-mono truncate">{rel.path}</div>
                  <div class="mt-1 flex flex-wrap gap-1">
                    {#each rel.matched_on as reason}
                      <span class="px-1.5 py-0.5 rounded bg-emerald-950/60 border border-emerald-800/60 text-emerald-300 text-[10px]">{reason}</span>
                    {/each}
                  </div>
                </button>
              {/each}
            </div>
          {/if}

          <div class="text-[10px] text-slate-500 font-mono pt-1 border-t border-[#252d3d]">
            Granskade {result.examined} filer{result.capped ? ' (avkortat)' : ''}{result.unreadable.length > 0 ? ` · ${result.unreadable.length} headers kunde inte läsas` : ''}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
