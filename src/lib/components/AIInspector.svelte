<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    isOllamaOnline,
    installedModels,
    runningModels,
    selectedModel,
    isAiGenerating,
    aiChatMessages,
    checkOllamaConnection,
    selectModelWithAutoEvict,
    unloadAllOllamaModels,
    askOllamaStream,
    stopAiGeneration,
    clearAiChat,
  } from '../stores/ollamaStore';
  import { renderMarkdown } from '../markdown';
  import type { FileItem, PreviewContent } from '../types';
  import {
    Bot,
    Sparkles,
    Send,
    Square,
    Trash2,
    RefreshCw,
    FileText,
    Bug,
    Lightbulb,
    Terminal,
    Zap,
    Copy,
    Check,
    AlertCircle,
    CheckCircle2,
    Cpu,
    ArrowDown,
    ArrowUp,
    Download,
    ShieldCheck,
    ChevronLeft,
    ChevronRight,
    Layers,
    Filter,
  } from 'lucide-svelte';

  export let item: FileItem | null = null;
  export let preview: PreviewContent | null = null;
  export let isLoadingFull: boolean = false;
  export let onLoadFull: (() => void) | undefined = undefined;
  export let onLoadTail: (() => void) | undefined = undefined;

  let inputPrompt = '';
  let chatScrollContainer: HTMLElement;
  let copiedId: string | null = null;

  type ContextMode = 'tail' | 'full' | 'sections' | 'errors';
  let contextMode: ContextMode = 'tail';
  let lastItemPath = '';

  const CHUNK_LINE_COUNT = 300;
  let currentSectionIndex = 0;

  function formatBytes(bytes: number): string {
    if (!bytes) return '0 B';
    if (bytes >= 1024 * 1024 * 1024) return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
    if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
    if (bytes >= 1024) return (bytes / 1024).toFixed(0) + ' KB';
    return bytes + ' B';
  }

  $: filenameLower = item?.name.toLowerCase() || '';
  $: isSnakemakeCandidate = !!item && (
    filenameLower.endsWith('.log') ||
    filenameLower.endsWith('.err') ||
    filenameLower.endsWith('.out') ||
    filenameLower.includes('snakemake') ||
    filenameLower.includes('slurm') ||
    (preview?.text_content ? (
      preview.text_content.includes('Error in rule') ||
      preview.text_content.includes('WorkflowError') ||
      preview.text_content.includes('SnakemakeException') ||
      preview.text_content.includes('Building DAG')
    ) : false)
  );

  $: fileLines = preview?.text_content ? preview.text_content.split('\n') : [];
  $: totalSections = Math.max(1, Math.ceil(fileLines.length / CHUNK_LINE_COUNT));

  $: if (item && item.path !== lastItemPath) {
    lastItemPath = item.path;
    currentSectionIndex = 0;
    if (isSnakemakeCandidate || filenameLower.endsWith('.log') || filenameLower.endsWith('.err') || preview?.is_tail) {
      contextMode = 'tail';
    } else {
      contextMode = 'full';
    }
  }

  $: if (currentSectionIndex >= totalSections) {
    currentSectionIndex = Math.max(0, totalSections - 1);
  }

  onMount(async () => {
    await checkOllamaConnection();
  });

  $: if ($aiChatMessages) {
    scrollToBottom();
  }

  async function scrollToBottom() {
    await tick();
    if (chatScrollContainer) {
      chatScrollContainer.scrollTop = chatScrollContainer.scrollHeight;
    }
  }

  function extractSnakemakeErrorSnippet(text: string): string {
    if (!text) return '';
    const lines = text.split('\n');
    let errorIndices: number[] = [];
    for (let i = 0; i < lines.length; i++) {
      const l = lines[i];
      if (
        l.includes('Error in rule') ||
        l.includes('RuleException') ||
        l.includes('WorkflowError') ||
        l.includes('Traceback (most recent call last):') ||
        l.includes('CalledProcessError') ||
        l.includes('Exiting because a job execution failed') ||
        /(error|fatal|exception)/i.test(l)
      ) {
        errorIndices.push(i);
      }
    }

    if (errorIndices.length === 0) {
      return lines.slice(-250).join('\n');
    }

    const firstKeyError = errorIndices.find((idx) =>
      lines[idx].includes('Error in rule') || lines[idx].includes('WorkflowError')
    ) ?? errorIndices[0];

    const start = Math.max(0, firstKeyError - 10);
    const end = Math.min(lines.length, start + 300);
    return lines.slice(start, end).join('\n');
  }

  function extractErrorLines(text: string): string {
    if (!text) return '';
    const lines = text.split('\n');
    const matchedLineIndices = new Set<number>();
    const errorRegex = /(error|fail|exception|traceback|fatal|errno|syntaxerror|warning|oom|killed)/i;

    for (let i = 0; i < lines.length; i++) {
      if (errorRegex.test(lines[i])) {
        for (let j = Math.max(0, i - 2); j <= Math.min(lines.length - 1, i + 2); j++) {
          matchedLineIndices.add(j);
        }
      }
    }

    if (matchedLineIndices.size === 0) {
      return 'Inga uppenbara fel- eller varningsrader (error, fail, traceback) identifierades i filen.';
    }

    const sorted = Array.from(matchedLineIndices).sort((a, b) => a - b);
    let result = '';
    let lastIdx = -2;
    for (const idx of sorted) {
      if (lastIdx !== -2 && idx > lastIdx + 1) {
        result += '\n--- [...] ---\n';
      }
      result += `[Rad ${idx + 1}] ${lines[idx]}\n`;
      lastIdx = idx;
    }
    return result;
  }

  function getSectionText(secIdx: number): string {
    const start = secIdx * CHUNK_LINE_COUNT;
    const end = Math.min(fileLines.length, start + CHUNK_LINE_COUNT);
    return `[Sektion ${secIdx + 1} av ${totalSections} (Rader ${start + 1}–${end}) av ${item?.name || 'filen'}]:\n` +
      fileLines.slice(start, end).join('\n');
  }

  function getFileContext(): string {
    if (!item) return '';
    let meta = `Filnamn: ${item.name}\nSökväg: ${item.path}\nStorlek: ${item.formatted_size}\nTyp: ${item.extension}\n`;
    if (preview?.is_truncated) {
      meta += `Status: Trunkerad förhandsvisning (${formatBytes(preview.loaded_bytes || 0)} inläst av ${preview.formatted_size})\n`;
    } else if (preview?.text_content) {
      meta += `Status: Hela filen inläst lokalt (${fileLines.length} rader)\n`;
    }

    if (!preview?.text_content) {
      if (preview?.table_headers && preview?.table_rows) {
        return meta + `\nTabellrubriker: ${preview.table_headers.join(', ')}\nFörsta rader:\n` +
          preview.table_rows.slice(0, 15).map((r) => r.join('\t')).join('\n');
      }
      return meta;
    }

    if (contextMode === 'tail') {
      const tailSnippet = extractSnakemakeErrorSnippet(preview.text_content);
      return `${meta}\nKontextläge: Loggslut & Fel (Tail)\n\`\`\`\n${tailSnippet}\n\`\`\``;
    } else if (contextMode === 'errors') {
      const errSnippet = extractErrorLines(preview.text_content);
      return `${meta}\nKontextläge: Filtrerade fel- och varningsrader\n\`\`\`\n${errSnippet}\n\`\`\``;
    } else if (contextMode === 'sections') {
      const secSnippet = getSectionText(currentSectionIndex);
      return `${meta}\nKontextläge: AI-Scroll Sektion ${currentSectionIndex + 1} av ${totalSections}\n\`\`\`\n${secSnippet}\n\`\`\``;
    } else {
      return `${meta}\nInnehåll:\n${preview.text_content}`;
    }
  }

  async function handleSend() {
    const prompt = inputPrompt.trim();
    if (!prompt || $isAiGenerating) return;
    inputPrompt = '';
    const fileCtx = getFileContext();
    await askOllamaStream(prompt, '', fileCtx);
  }

  async function handleSnakemakeTroubleshoot() {
    if ($isAiGenerating) return;
    contextMode = 'tail';
    const errorSnippet = extractSnakemakeErrorSnippet(preview?.text_content || '');
    const prompt =
      `Analysera detta Snakemake-fel noggrant:\n` +
      `1. Vilken regel (rule) fallerade och vad orsakade att jobbet kraschade?\n` +
      `2. Identifiera bakomliggande orsak (t.ex. saknade input/output-filer, syntaxfel, felkoder, minnesbrist/OOM, eller saknade miljöpaket).\n` +
      `3. Ge konkreta steg för hur användaren åtgärdar felet i Snakefile, skriptet eller på klustret.`;

    const fileCtx = `Snakemake Logg (${item?.name || 'logg'} - Felavsnitt):\n\`\`\`\n${errorSnippet}\n\`\`\``;
    await askOllamaStream(prompt, '', fileCtx);
  }

  async function handleSummarizeSection() {
    if ($isAiGenerating) return;
    contextMode = 'sections';
    const secSnippet = getSectionText(currentSectionIndex);
    const startLine = currentSectionIndex * CHUNK_LINE_COUNT + 1;
    const endLine = Math.min(fileLines.length, (currentSectionIndex + 1) * CHUNK_LINE_COUNT);
    const prompt = `Sammanfatta sektion ${currentSectionIndex + 1} av ${totalSections} (rader ${startLine}–${endLine}) i denna fil. Vad sker i detta avsnitt?`;
    await askOllamaStream(prompt, '', secSnippet);
  }

  async function handleQuickAction(actionType: 'summary' | 'explain' | 'bugs' | 'bash' | 'optimize') {
    if ($isAiGenerating) return;
    const fileCtx = getFileContext();
    let prompt = '';

    if (actionType === 'summary') {
      prompt = `Ge en koncis och informativ sammanfattning av denna fil (${item?.name || 'filen'}). Förklara dess syfte och huvudsakliga innehåll i 3-4 punkter.`;
    } else if (actionType === 'explain') {
      prompt = `Förklara hur denna kod/data fungerar i detalj. Beskriv dess arkitektur, nyckelfunktioner och vad den åstadkommer.`;
    } else if (actionType === 'bugs') {
      prompt = `Granska denna kod/logg noggrant. Hitta potentiella buggar, felmeddelanden, prestandaproblem eller förbättringsmöjligheter.`;
    } else if (actionType === 'bash') {
      prompt = `Ge 2-3 användbara Bash / CLI-kommandon för att inspektera, transformera, filtrera eller felsöka denna fil i Terminalen.`;
    } else if (actionType === 'optimize') {
      prompt = `Föreslå konkreta optimeringar för att göra denna kod eller datahantering snabbare och mer minneseffektiv.`;
    }

    await askOllamaStream(prompt, '', fileCtx);
  }

  async function copyMessage(content: string, id: string) {
    try {
      await navigator.clipboard.writeText(content);
      copiedId = id;
      setTimeout(() => {
        copiedId = null;
      }, 2000);
    } catch {}
  }
</script>

<div class="flex-1 flex flex-col h-full min-h-0 bg-[var(--bg-surface)] text-[var(--text-primary)] text-xs select-none">
  <!-- AI Top Control Bar -->
  <div class="px-3 py-2 border-b border-[var(--border)] bg-[var(--bg-panel)] flex items-center justify-between gap-2 shrink-0">
    <div class="flex items-center gap-2 min-w-0">
      <!-- Status Badge -->
      {#if $isOllamaOnline}
        <div class="flex items-center gap-1 text-emerald-400 font-bold text-[10px] tracking-wide shrink-0" title="Ollama API är ansluten (localhost:11434)">
          <div class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></div>
          <span>OLLAMA</span>
        </div>
      {:else}
        <div class="flex items-center gap-1 text-rose-400 font-bold text-[10px] tracking-wide shrink-0" title="Kunde inte ansluta till Ollama på http://127.0.0.1:11434">
          <AlertCircle size={11} />
          <span>OFFLINE</span>
        </div>
      {/if}

      <!-- Model Selector Dropdown -->
      {#if $isOllamaOnline && $installedModels.length > 0}
        <select
          value={$selectedModel}
          on:change={(e) => selectModelWithAutoEvict(e.currentTarget.value)}
          class="bg-[var(--bg-surface)] text-[var(--text-primary)] text-[11px] font-mono px-2 py-0.5 rounded border border-[var(--border)] focus:border-[var(--accent)] focus:outline-none max-w-[170px] truncate"
          title="Välj modell. Tidigare modell avlastas automatiskt för att spara RAM."
        >
          {#each $installedModels as m}
            <option value={m.name}>
              {m.name} {m.parameter_size ? `(${m.parameter_size})` : ''}
            </option>
          {/each}
        </select>
      {/if}

      <!-- Running Memory Usage Badge -->
      {#if $runningModels.length > 0}
        {@const totalRAM = $runningModels.map(m => m.formatted_size).join(', ')}
        {@const isHeavy = $runningModels.some(m => m.size > 20_000_000_000)}
        <div
          class="flex items-center gap-1 px-1.5 py-0.5 rounded {isHeavy ? 'bg-rose-500/20 text-rose-300 ring-1 ring-rose-500/40' : 'bg-amber-500/20 text-amber-300'} font-mono text-[10px] shrink-0"
          title="Modell aktiv i VRAM/RAM: {totalRAM}"
        >
          <Cpu size={10} />
          <span>{totalRAM}</span>
        </div>
      {/if}
    </div>

    <!-- Actions: Unload RAM, Refresh & Clear -->
    <div class="flex items-center gap-1 shrink-0">
      {#if $runningModels.length > 0}
        <button
          class="px-2 py-0.5 rounded bg-rose-600/20 hover:bg-rose-600/40 text-rose-300 border border-rose-500/40 font-sans text-[10.5px] font-semibold flex items-center gap-1 transition-colors shrink-0"
          on:click={() => unloadAllOllamaModels()}
          title="Frigör RAM: Avlasta modellen omedelbart ur minnet för att göra datorn snabb igen"
        >
          <span>Frigör RAM</span>
        </button>
      {/if}

      <button
        class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
        on:click={() => checkOllamaConnection()}
        title="Uppdatera modeller från Ollama"
      >
        <RefreshCw size={12} class={$isAiGenerating ? 'animate-spin' : ''} />
      </button>

      {#if $aiChatMessages.length > 0}
        <button
          class="p-1 rounded hover:bg-[var(--bg-hover)] text-rose-400 hover:text-rose-300 transition-colors"
          on:click={clearAiChat}
          title="Rensa konversation"
        >
          <Trash2 size={12} />
        </button>
      {/if}
    </div>
  </div>

  <!-- Security & Privacy Banner: Zero cluster access -->
  <div class="px-3 py-1 bg-emerald-950/40 border-b border-emerald-500/20 flex items-center justify-between text-[10px] text-emerald-300">
    <div class="flex items-center gap-1.5 truncate">
      <ShieldCheck size={12} class="text-emerald-400 shrink-0" />
      <span class="font-medium truncate">100% Lokal AI (Ollama) · Ingen åtkomst eller trafik till klustret</span>
    </div>
    {#if preview && !preview.is_truncated}
      <span class="text-[9.5px] text-emerald-400/80 font-mono shrink-0 font-medium">✓ Hela filen inläst</span>
    {/if}
  </div>

  <!-- Truncation / Full file load notification banner -->
  {#if preview?.is_truncated}
    <div class="px-3 py-1.5 bg-amber-500/10 border-b border-amber-500/30 flex items-center justify-between text-[11px] text-amber-300 gap-2">
      <div class="flex items-center gap-1.5 truncate">
        <AlertCircle size={13} class="text-amber-400 shrink-0" />
        <span class="truncate">
          Visar {preview.is_tail ? 'loggslutet' : 'början'} ({formatBytes(preview.loaded_bytes || 0)} av {preview.formatted_size || 'stor fil'})
        </span>
      </div>
      <div class="flex items-center gap-1.5 shrink-0">
        {#if !preview.is_tail && onLoadTail}
          <button
            class="px-2 py-0.5 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-200 border border-amber-500/40 font-medium transition-colors text-[10.5px] flex items-center gap-1"
            on:click={onLoadTail}
            disabled={isLoadingFull}
            title="Ladda snabbt in loggslutet där eventuella fel finns"
          >
            <ArrowDown size={11} />
            <span>Visa loggslut</span>
          </button>
        {/if}
        {#if onLoadFull}
          <button
            class="px-2 py-0.5 rounded bg-[var(--accent)] hover:opacity-90 text-white font-medium transition-opacity text-[10.5px] flex items-center gap-1 shadow-sm"
            on:click={onLoadFull}
            disabled={isLoadingFull}
            title="Ladda in hela filen lokalt till Macen för fullständig AI-analys"
          >
            {#if isLoadingFull}
              <RefreshCw size={11} class="animate-spin" />
              <span>Läser in...</span>
            {:else}
              <Download size={11} />
              <span>Läs in hela ({preview.formatted_size})</span>
            {/if}
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <!-- AI Context Mode Selector & AI-Scroll ("så att AI kan scrolla") -->
  {#if preview?.text_content && (fileLines.length > 50 || preview.is_truncated)}
    <div class="px-3 py-1.5 bg-[var(--bg-surface)] border-b border-[var(--border)] flex items-center justify-between text-[11px] gap-2 flex-wrap">
      <div class="flex items-center gap-1 flex-wrap">
        <span class="text-[var(--text-muted)] font-semibold text-[9.5px] uppercase tracking-wider mr-1">Kontext:</span>
        <button
          class="px-2 py-0.5 rounded font-medium transition-colors {contextMode === 'tail' ? 'bg-[var(--accent)] text-white font-bold' : 'bg-[var(--bg-panel)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] border border-[var(--border)]'}"
          on:click={() => (contextMode = 'tail')}
          title="Fokusera på de sista 300 raderna i filen där krascher och tracebacks normalt finns"
        >
          ⚡ Loggslut & Fel
        </button>
        <button
          class="px-2 py-0.5 rounded font-medium transition-colors {contextMode === 'full' ? 'bg-[var(--accent)] text-white font-bold' : 'bg-[var(--bg-panel)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] border border-[var(--border)]'}"
          on:click={() => (contextMode = 'full')}
          title="Skicka hela det inlästa innehållet till AI-modellen"
        >
          📄 Hela ({fileLines.length} rader)
        </button>
        <button
          class="px-2 py-0.5 rounded font-medium transition-colors {contextMode === 'sections' ? 'bg-[var(--accent)] text-white font-bold' : 'bg-[var(--bg-panel)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] border border-[var(--border)]'}"
          on:click={() => (contextMode = 'sections')}
          title="AI-Scroll: Bläddra sektion för sektion genom stora loggar och filer"
        >
          📜 AI-Scroll
        </button>
        <button
          class="px-2 py-0.5 rounded font-medium transition-colors {contextMode === 'errors' ? 'bg-[var(--accent)] text-white font-bold' : 'bg-[var(--bg-panel)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] border border-[var(--border)]'}"
          on:click={() => (contextMode = 'errors')}
          title="Filtrera ut alla rader med error, traceback, exception, warning eller failed"
        >
          🔍 Endast felrader
        </button>
      </div>

      <!-- Section Navigator (AI-Scroll Paging) -->
      {#if contextMode === 'sections' && totalSections > 1}
        <div class="flex items-center gap-1.5 bg-[var(--bg-panel)] px-2 py-0.5 rounded border border-[var(--border)] text-[10.5px]">
          <button
            class="px-1.5 py-0.5 rounded hover:bg-[var(--bg-active)] disabled:opacity-30 font-bold"
            disabled={currentSectionIndex === 0}
            on:click={() => (currentSectionIndex = Math.max(0, currentSectionIndex - 1))}
            title="Föregående sektion"
          >
            <ChevronLeft size={12} />
          </button>
          <span class="font-mono text-[var(--text-secondary)]">
            {currentSectionIndex + 1} / {totalSections}
            <span class="text-[var(--text-muted)] text-[9.5px]">
              (rad {currentSectionIndex * CHUNK_LINE_COUNT + 1}–{Math.min(fileLines.length, (currentSectionIndex + 1) * CHUNK_LINE_COUNT)})
            </span>
          </span>
          <button
            class="px-1.5 py-0.5 rounded hover:bg-[var(--bg-active)] disabled:opacity-30 font-bold"
            disabled={currentSectionIndex >= totalSections - 1}
            on:click={() => (currentSectionIndex = Math.min(totalSections - 1, currentSectionIndex + 1))}
            title="Nästa sektion"
          >
            <ChevronRight size={12} />
          </button>
          <button
            class="ml-1 px-1.5 py-0.5 rounded bg-[var(--bg-surface)] hover:bg-[var(--accent)] hover:text-white text-[10px] font-medium border border-[var(--border)] transition-colors"
            on:click={handleSummarizeSection}
            disabled={$isAiGenerating}
            title="Be modellen sammanfatta denna specifika sektion"
          >
            Sammanfatta
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Quick Action Chips -->
  {#if item}
    <div class="px-3 py-2 border-b border-[var(--border)]/60 bg-[var(--bg-base)] flex items-center gap-1.5 overflow-x-auto shrink-0 scrollbar-none">
      <!-- Snakemake Specific Diagnostic Action -->
      {#if isSnakemakeCandidate}
        <button
          class="px-2.5 py-1 rounded-md bg-emerald-600 hover:bg-emerald-500 text-white border border-emerald-400/40 text-[11px] font-bold flex items-center gap-1.5 transition-all disabled:opacity-50 shrink-0 shadow-sm"
          on:click={handleSnakemakeTroubleshoot}
          disabled={$isAiGenerating || !$isOllamaOnline}
          title="Extrahera automatiskt fallerande regel, Python-traceback och felorsak från loggen"
        >
          <span>🐍</span>
          <span>Felsök Snakemake-fel</span>
        </button>
      {/if}

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-[var(--text-primary)] border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('summary')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <FileText size={11} class="text-blue-400" />
        <span>Sammanfatta</span>
      </button>

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-[var(--text-primary)] border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('explain')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <Lightbulb size={11} class="text-amber-400" />
        <span>Förklara</span>
      </button>

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-[var(--text-primary)] border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('bugs')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <Bug size={11} class="text-rose-400" />
        <span>Felsök</span>
      </button>

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-[var(--text-primary)] border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('bash')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <Terminal size={11} class="text-emerald-400" />
        <span>Bash-kommando</span>
      </button>

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-[var(--text-primary)] border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('optimize')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <Zap size={11} class="text-purple-400" />
        <span>Optimera</span>
      </button>
    </div>
  {/if}

  <!-- Main Chat Conversation Scroll Area -->
  <div bind:this={chatScrollContainer} class="flex-1 overflow-y-auto p-3 space-y-3 bg-[var(--bg-base)]">
    {#if !$isOllamaOnline}
      <div class="h-full flex flex-col items-center justify-center p-6 text-center text-[var(--text-muted)] space-y-3">
        <Bot size={36} class="opacity-30 text-amber-400" />
        <div class="space-y-1 max-w-xs">
          <p class="font-bold text-xs text-[var(--text-primary)]">Ollama är inte igång</p>
          <p class="text-[11px] text-[var(--text-secondary)]">
            Starta Ollama i Terminalen med <code class="px-1.5 py-0.5 rounded bg-[var(--bg-active)] text-amber-300 font-mono">ollama serve</code>
          </p>
        </div>
        <button
          class="px-3 py-1.5 rounded bg-[var(--accent)] text-white font-bold text-xs hover:opacity-90 transition-opacity flex items-center gap-1.5"
          on:click={() => checkOllamaConnection()}
        >
          <RefreshCw size={12} />
          <span>Försök igen</span>
        </button>
      </div>

    {:else if $aiChatMessages.length === 0}
      <div class="h-full flex flex-col items-center justify-center p-6 text-center text-[var(--text-muted)] space-y-3">
        <Sparkles size={32} class="opacity-30 text-[var(--accent)]" />
        <div class="space-y-1.5 max-w-sm">
          <p class="font-bold text-xs text-[var(--text-primary)]">Lokal & Säker AI-Assistent</p>
          <p class="text-[11px] text-[var(--text-secondary)] leading-relaxed">
            {#if item}
              Modellen ({$selectedModel || 'Ollama'}) körs 100% lokalt på din Mac. Inga förfrågningar eller data skickas till klustret.
              {#if isSnakemakeCandidate}
                <br /><span class="text-emerald-400 font-medium">Loggfil detekterad: Klicka på "🐍 Felsök Snakemake-fel" för omedelbar analys.</span>
              {/if}
            {:else}
              Välj en fil i utforskaren för att analysera den med din lokala modell ({$selectedModel}).
            {/if}
          </p>
        </div>
      </div>

    {:else}
      {#each $aiChatMessages as msg (msg.id)}
        {#if msg.role === 'user'}
          <div class="flex justify-end">
            <div class="max-w-[85%] rounded-2xl rounded-tr-sm px-3 py-2 bg-[var(--accent)] text-white font-sans text-xs shadow-md">
              {msg.content}
            </div>
          </div>
        {:else if msg.role === 'assistant'}
          <div class="flex flex-col space-y-1 group">
            <div class="flex items-center justify-between px-1">
              <div class="flex items-center gap-1 text-[10px] font-bold text-emerald-400 font-mono">
                <Bot size={11} />
                <span>{$selectedModel.split(':')[0]}</span>
              </div>
              <button
                class="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-opacity"
                on:click={() => copyMessage(msg.content, msg.id)}
                title="Kopiera svar"
              >
                {#if copiedId === msg.id}
                  <Check size={11} class="text-emerald-400" />
                {:else}
                  <Copy size={11} />
                {/if}
              </button>
            </div>

            <div class="rounded-xl px-3 py-2.5 bg-[var(--bg-panel)] border border-[var(--border)] text-xs text-[var(--text-primary)] leading-relaxed select-text font-sans shadow-sm prose-invert overflow-x-auto">
              {#if !msg.content && $isAiGenerating}
                <div class="flex items-center gap-1.5 text-amber-400 text-xs font-mono py-1 animate-pulse">
                  <Sparkles size={13} class="animate-spin" />
                  <span>Tänker & analyserar...</span>
                </div>
              {:else}
                {@html renderMarkdown(msg.content)}
              {/if}
            </div>
          </div>
        {/if}
      {/each}
    {/if}
  </div>

  <!-- Bottom Input Bar -->
  <div class="p-2.5 border-t border-[var(--border)] bg-[var(--bg-panel)] shrink-0">
    <form on:submit|preventDefault={handleSend} class="flex items-center gap-2">
      <input
        type="text"
        placeholder={item ? `Fråga om ${item.name}...` : 'Ställ en fråga till modellen...'}
        bind:value={inputPrompt}
        disabled={!$isOllamaOnline || $isAiGenerating}
        class="flex-1 px-3 py-1.5 rounded-lg bg-[var(--bg-surface)] text-xs text-[var(--text-primary)] border border-[var(--border)] focus:border-[var(--accent)] focus:outline-none placeholder-[var(--text-muted)] disabled:opacity-50"
      />

      {#if $isAiGenerating}
        <button
          type="button"
          on:click={stopAiGeneration}
          class="px-3 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-[var(--text-primary)] text-xs font-bold flex items-center gap-1 shadow transition-colors"
          title="Stoppa generering"
        >
          <Square size={11} />
          <span>Stopp</span>
        </button>
      {:else}
        <button
          type="submit"
          disabled={!$isOllamaOnline || !inputPrompt.trim()}
          class="px-3 py-1.5 rounded-lg bg-[var(--accent)] hover:opacity-90 text-white text-xs font-bold flex items-center gap-1 shadow transition-opacity disabled:opacity-40"
          title="Skicka fråga (Enter)"
        >
          <Send size={11} />
          <span>Fråga</span>
        </button>
      {/if}
    </form>
  </div>
</div>
