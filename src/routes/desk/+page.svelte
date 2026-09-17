<script lang="ts">
  import { onMount } from 'svelte';
  import { cguDeskStatus, revealInOs } from '$lib/invoke';
  import type { DeskStatus, SampleState } from '$lib/types';
  import { Activity, AlertTriangle, FolderOpen, Mail, RefreshCw } from 'lucide-svelte';

  let status: DeskStatus | null = null;
  let error = '';
  let loading = false;
  let days = 7;
  let selected: SampleState | null = null;

  // Klara prov och sample sheet-notiser hör inte hemma i en lista över vad
  // som behöver uppmärksamhet. De räknas, de listas inte.
  const WATCH = ['fel', 'hänger', 'pågår', 'köad'];

  // Appens temavariabler har ingen fel- eller varningsfärg; resten av
  // kodbasen använder red-400 och amber-400 direkt, så det gör den här med.
  const TONE: Record<string, string> = {
    fel: 'text-red-400 border-red-400/50',
    hänger: 'text-amber-400 border-amber-400/50',
    pågår: 'text-[var(--accent)] border-[var(--accent)]',
    köad: 'text-[var(--text-muted)] border-[var(--border)]',
  };

  async function load() {
    loading = true;
    error = '';
    try {
      status = await cguDeskStatus(days);
      if (selected) {
        selected = status.samples.find((s) => s.sample === selected?.sample) ?? null;
      }
    } catch (e: any) {
      error = String(e);
      status = null;
    } finally {
      loading = false;
    }
  }

  function when(iso: string): string {
    return (iso || '').slice(5, 16).replace('T', ' ');
  }

  function timelineFor(sample: string) {
    return status?.timeline.filter((e) => e.sample === sample) ?? [];
  }

  onMount(load);
</script>

<div class="h-screen flex flex-col bg-[var(--bg-base)] text-[var(--text-primary)]">
  <header
    class="flex items-center gap-3 px-4 py-2 border-b border-[var(--border)] bg-[var(--bg-surface)]"
  >
    <Activity size={14} class="text-[var(--accent)]" />
    <h1 class="text-[13px] font-semibold">Pipelineläge</h1>
    <a href="/" class="text-[11px] text-[var(--accent)] hover:underline">← filer</a>
    {#if status}
      <span class="text-[11px] text-[var(--text-muted)] font-mono">
        {Object.entries(status.counts)
          .map(([k, n]) => `${n} ${k}`)
          .join(', ')}
      </span>
    {/if}
    <div class="ml-auto flex items-center gap-2">
      <select
        bind:value={days}
        on:change={load}
        class="text-[12px] px-2 py-1 rounded bg-[var(--bg-base)] border border-[var(--border)]"
      >
        <option value={1}>1 dygn</option>
        <option value={7}>7 dygn</option>
        <option value={21}>21 dygn</option>
      </select>
      <button
        on:click={load}
        class="p-1 rounded hover:bg-[var(--bg-hover)]"
        title="Läs om"
        disabled={loading}
      >
        <RefreshCw size={13} class={loading ? 'animate-spin' : ''} />
      </button>
    </div>
  </header>

  {#if error}
    <div class="m-4 p-3 rounded border border-red-400/50 text-red-400 text-[12px]">
      {error}
    </div>
  {:else if status}
    <div class="flex-1 grid grid-cols-[minmax(320px,420px)_1fr] min-h-0">
      <div class="overflow-y-auto border-r border-[var(--border)]">
        {#each status.samples.filter((s) => WATCH.includes(s.state)) as s}
          <button
            class="w-full text-left px-4 py-2.5 border-b border-[var(--border)] hover:bg-[var(--bg-hover)]"
            class:bg-[var(--bg-active)]={selected?.sample === s.sample}
            on:click={() => (selected = s)}
          >
            <div class="flex items-baseline gap-2">
              <span
                class="text-[10px] uppercase font-mono px-1.5 rounded border {TONE[s.state] ?? ''}"
              >
                {s.state}
              </span>
              <span class="text-[13px] font-medium">{s.sample}</span>
              <span class="ml-auto text-[11px] font-mono text-[var(--text-muted)]">
                {when(s.last)}
              </span>
            </div>
            <div class="mt-1 text-[11px] text-[var(--text-secondary)]">
              {[s.wp, s.assay].filter(Boolean).join('/')} — {s.note}
            </div>
          </button>
        {/each}

        {#if status.mail.length}
          <div
            class="px-4 py-1.5 text-[10px] uppercase tracking-wider font-mono text-[var(--text-muted)] bg-[var(--bg-base)] border-b border-[var(--border)] sticky top-0 flex items-center gap-1.5"
          >
            <Mail size={11} /> posten i övrigt
          </div>
          {#each status.mail.slice(0, 12) as m}
            <div class="px-4 py-2.5 border-b border-[var(--border)]">
              <div class="text-[12px] font-medium">{m.subject}</div>
              <div class="mt-1 text-[11px] text-[var(--text-secondary)] line-clamp-2">
                {m.lage}
              </div>
              <div class="mt-1 text-[10px] font-mono text-[var(--text-muted)]">
                {m.sender} · {when(m.covers_to)}{m.datum && m.datum !== 'inget nämnt'
                  ? ` · ${m.datum}`
                  : ''}
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <div class="overflow-y-auto p-5">
        {#if selected}
          <h2 class="text-[15px] font-semibold">{selected.sample}</h2>
          <div class="mt-1 text-[11px] font-mono text-[var(--text-muted)]">
            {[selected.wp, selected.assay].filter(Boolean).join('/')} · {selected.state} ·
            {selected.note} · {selected.events} händelser
          </div>

          <div class="mt-4 rounded border border-[var(--border)] bg-[var(--bg-surface)]">
            <div
              class="px-3 py-1.5 text-[10px] uppercase tracking-wider font-mono text-[var(--text-muted)] border-b border-[var(--border)]"
            >
              händelser
            </div>
            <div class="p-3 font-mono text-[11px] space-y-1">
              {#each timelineFor(selected.sample) as e}
                <div class="grid grid-cols-[90px_70px_1fr] gap-2">
                  <span class="text-[var(--text-muted)]">{when(e.sent)}</span>
                  <span
                    class={e.status === 'ERROR'
                      ? 'text-red-400'
                      : e.status === 'SUCCESS' || e.status === 'DONE'
                        ? 'text-[var(--accent)]'
                        : ''}>{e.status}</span
                  >
                  <span class="text-[var(--text-secondary)]">
                    {(e.subject || '').split('] - ').slice(-1)[0]}
                  </span>
                </div>
              {/each}
            </div>
          </div>

          {#if selected.folder}
            <!-- Det här är hela poängen med att panelen sitter i en filhanterare:
                 felet pekar ut en mapp, och mappen går att öppna. -->
            <button
              class="mt-4 flex items-center gap-2 px-3 py-2 rounded border border-[var(--border)] hover:bg-[var(--bg-hover)] text-[12px] font-mono"
              on:click={() => selected && revealInOs(selected.folder)}
            >
              <FolderOpen size={13} class="text-[var(--accent)]" />
              {selected.folder}
            </button>
            {#if selected.execution}
              <div class="mt-2 text-[10px] font-mono text-[var(--text-muted)]">
                workflow {selected.execution}
              </div>
            {/if}
          {/if}
        {:else}
          <p class="text-[12px] text-[var(--text-muted)] max-w-[46ch] leading-relaxed">
            Välj ett prov till vänster.
          </p>
          <p class="mt-3 text-[11px] font-mono text-[var(--text-muted)]">
            {status.store}
          </p>
        {/if}
      </div>
    </div>
  {:else}
    <div class="flex-1 grid place-items-center text-[12px] text-[var(--text-muted)]">
      <span class="flex items-center gap-2"><AlertTriangle size={13} /> läser lagret…</span>
    </div>
  {/if}
</div>
