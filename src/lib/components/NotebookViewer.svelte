<script lang="ts">
  import { renderMarkdown } from '../markdown';
  import { Copy, Check, Terminal, FileCode, Play } from 'lucide-svelte';

  export let jsonContent: string = '';
  export let filename: string = '';
  export let formattedSize: string = '--';

  interface NotebookOutput {
    output_type?: string;
    text?: string | string[];
    data?: {
      'text/plain'?: string | string[];
      'text/html'?: string | string[];
      'image/png'?: string;
      'image/jpeg'?: string;
      'image/svg+xml'?: string | string[];
    };
    name?: string;
    traceback?: string[];
  }

  interface NotebookCell {
    cell_type: 'markdown' | 'code' | 'raw';
    source: string | string[];
    execution_count?: number | null;
    outputs?: NotebookOutput[];
  }

  let cells: NotebookCell[] = [];
  let parseError = '';

  $: {
    try {
      if (jsonContent) {
        const parsed = JSON.parse(jsonContent);
        cells = parsed.cells || [];
        parseError = '';
      }
    } catch (err: any) {
      parseError = err.message || 'Kunde inte tolka Jupyter Notebook-strukturen';
      cells = [];
    }
  }

  function getSourceText(source: string | string[]): string {
    if (Array.isArray(source)) {
      return source.join('');
    }
    return source || '';
  }

  function getOutputText(text?: string | string[]): string {
    if (!text) return '';
    if (Array.isArray(text)) return text.join('');
    return text;
  }
</script>

<div class="flex flex-col h-full bg-[#12151c] text-slate-200 select-text overflow-hidden font-sans">
  <!-- Notebook Header -->
  <div class="flex items-center justify-between px-4 py-2 bg-[#171b24] border-b border-[#252d3d] shrink-0 text-xs select-none">
    <div class="flex items-center gap-2">
      <span class="flex items-center gap-1.5 px-2.5 py-0.5 rounded bg-amber-500/20 text-amber-300 font-bold border border-amber-500/30 text-xs">
        <span>🪐</span>
        <span>Jupyter Notebook</span>
      </span>
      <span class="text-slate-400 text-[11px] font-mono">
        {cells.length} {cells.length === 1 ? 'cell' : 'celler'} • {formattedSize}
      </span>
    </div>
  </div>

  <!-- Notebook Body -->
  <div class="flex-1 overflow-auto p-4 space-y-4 max-w-5xl mx-auto w-full">
    {#if parseError}
      <div class="p-4 rounded-lg bg-red-500/10 border border-red-500/30 text-red-300 text-xs">
        <span class="font-bold">Fel vid tolkning av notebook:</span> {parseError}
      </div>
    {:else if cells.length === 0}
      <div class="text-center text-slate-500 py-12 text-xs">
        Notebooken har inga celler.
      </div>
    {:else}
      {#each cells as cell, cIdx}
        {@const src = getSourceText(cell.source)}

        {#if cell.cell_type === 'markdown'}
          <!-- Markdown Cell -->
          <div class="p-4 rounded-lg bg-[#161a24]/80 border border-[#252d3d] text-sm leading-relaxed space-y-2">
            {@html renderMarkdown(src)}
          </div>

        {:else if cell.cell_type === 'code'}
          <!-- Code Cell -->
          <div class="rounded-lg bg-[#0d1017] border border-[#252d3d] overflow-hidden shadow-md">
            <!-- Code Input -->
            <div class="flex items-start bg-[#141822] border-b border-[#202636]">
              <!-- Execution Prompt -->
              <div class="w-16 py-2.5 text-right pr-3 font-mono text-[11px] font-bold text-cyan-400 select-none shrink-0 bg-[#12151c]/60">
                [{cell.execution_count ?? ' '}] :
              </div>
              <!-- Code Text -->
              <pre class="flex-1 p-2.5 m-0 font-mono text-xs text-slate-200 overflow-x-auto whitespace-pre leading-relaxed">{src}</pre>
            </div>

            <!-- Code Output (if any) -->
            {#if cell.outputs && cell.outputs.length > 0}
              <div class="divide-y divide-[#202636]/60 bg-[#0d1017]">
                {#each cell.outputs as out}
                  <!-- 1. Text Stream (stdout / stderr) -->
                  {#if out.text}
                    <div class="flex items-start">
                      <div class="w-16 py-1.5 text-right pr-3 font-mono text-[10px] text-slate-500 select-none shrink-0">
                        {out.name === 'stderr' ? 'err:' : 'out:'}
                      </div>
                      <pre class="flex-1 p-2 m-0 font-mono text-[11px] {out.name === 'stderr' ? 'text-rose-400' : 'text-slate-300'} whitespace-pre-wrap">{getOutputText(out.text)}</pre>
                    </div>
                  {/if}

                  <!-- 2. Rich Data Outputs (Plots, HTML tables, plain data) -->
                  {#if out.data}
                    <!-- Matplotlib / Seaborn Image Plot -->
                    {#if out.data['image/png']}
                      <div class="flex items-start p-3 bg-white/[0.02]">
                        <div class="w-16 text-right pr-3 font-mono text-[10px] text-slate-500 select-none shrink-0">
                          plot:
                        </div>
                        <div class="flex-1 bg-white p-2 rounded-md shadow max-w-2xl">
                          <img
                            src="data:image/png;base64,{out.data['image/png']}"
                            alt="Jupyter plot output"
                            class="max-w-full h-auto"
                          />
                        </div>
                      </div>
                    {:else if out.data['image/jpeg']}
                      <div class="flex items-start p-3 bg-white/[0.02]">
                        <div class="w-16 text-right pr-3 font-mono text-[10px] text-slate-500 select-none shrink-0">
                          plot:
                        </div>
                        <div class="flex-1 bg-white p-2 rounded-md shadow max-w-2xl">
                          <img
                            src="data:image/jpeg;base64,{out.data['image/jpeg']}"
                            alt="Jupyter plot output"
                            class="max-w-full h-auto"
                          />
                        </div>
                      </div>
                    {/if}

                    <!-- HTML Output (e.g. Pandas DataFrame) -->
                    {#if out.data['text/html']}
                      <div class="flex items-start p-3 overflow-x-auto">
                        <div class="w-16 text-right pr-3 font-mono text-[10px] text-slate-500 select-none shrink-0">
                          html:
                        </div>
                        <div class="flex-1 text-xs select-text overflow-x-auto text-slate-200">
                          {@html getOutputText(out.data['text/html'])}
                        </div>
                      </div>
                    <!-- Plain text return value -->
                    {:else if out.data['text/plain'] && !out.data['image/png'] && !out.data['image/jpeg']}
                      <div class="flex items-start">
                        <div class="w-16 py-1.5 text-right pr-3 font-mono text-[10px] text-slate-500 select-none shrink-0">
                          out:
                        </div>
                        <pre class="flex-1 p-2 m-0 font-mono text-[11px] text-cyan-300 whitespace-pre-wrap">{getOutputText(out.data['text/plain'])}</pre>
                      </div>
                    {/if}
                  {/if}

                  <!-- 3. Error Traceback -->
                  {#if out.traceback && out.traceback.length > 0}
                    <div class="flex items-start bg-rose-950/20 p-2">
                      <div class="w-16 text-right pr-3 font-mono text-[10px] text-rose-400 select-none shrink-0">
                        trace:
                      </div>
                      <pre class="flex-1 m-0 font-mono text-[11px] text-rose-300 whitespace-pre-wrap">{out.traceback.join('\n')}</pre>
                    </div>
                  {/if}
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      {/each}
    {/if}
  </div>
</div>
