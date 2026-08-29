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
  } from 'lucide-svelte';

  export let item: FileItem | null = null;
  export let preview: PreviewContent | null = null;

  let inputPrompt = '';
  let chatScrollContainer: HTMLElement;
  let copiedId: string | null = null;

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

  function getFileContext(): string {
    if (!item) return '';
    let context = `Filnamn: ${item.name}\nSökväg: ${item.path}\nStorlek: ${item.formatted_size}\nTyp: ${item.extension}\n`;
    if (preview?.text_content) {
      context += `\nInnehåll:\n${preview.text_content}`;
    } else if (preview?.table_headers && preview?.table_rows) {
      context += `\nTabellrubriker: ${preview.table_headers.join(', ')}\nFörsta rader:\n` +
        preview.table_rows.slice(0, 15).map((r) => r.join('\t')).join('\n');
    }
    return context;
  }

  async function handleSend() {
    const prompt = inputPrompt.trim();
    if (!prompt || $isAiGenerating) return;
    inputPrompt = '';
    const fileCtx = getFileContext();
    await askOllamaStream(prompt, '', fileCtx);
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
      prompt = `Granska denna kod noggrant. Hitta potentiella buggar, prestandaproblem, säkerhetsbrister eller förbättringsmöjligheter.`;
    } else if (actionType === 'bash') {
      prompt = `Ge 2-3 användbara Bash / CLI-kommandon för att inspektera, transformera, filtrera eller köra denna fil i Terminalen.`;
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
        class="p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-secondary)] hover:text-white transition-colors"
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

  <!-- Quick Action Chips -->
  {#if item}
    <div class="px-3 py-2 border-b border-[var(--border)]/60 bg-[var(--bg-base)] flex items-center gap-1.5 overflow-x-auto shrink-0 scrollbar-none">
      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-white border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('summary')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <FileText size={11} class="text-blue-400" />
        <span>Sammanfatta</span>
      </button>

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-white border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('explain')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <Lightbulb size={11} class="text-amber-400" />
        <span>Förklara</span>
      </button>

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-white border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('bugs')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <Bug size={11} class="text-rose-400" />
        <span>Felsök</span>
      </button>

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-white border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
        on:click={() => handleQuickAction('bash')}
        disabled={$isAiGenerating || !$isOllamaOnline}
      >
        <Terminal size={11} class="text-emerald-400" />
        <span>Bash-kommando</span>
      </button>

      <button
        class="px-2 py-1 rounded-md bg-[var(--bg-panel)] hover:bg-[var(--accent)] hover:text-white border border-[var(--border)] text-[11px] font-medium flex items-center gap-1 transition-all disabled:opacity-50 shrink-0"
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
            Starta Ollama i Terminalen med <code class="px-1.5 py-0.5 rounded bg-black/40 text-amber-300 font-mono">ollama serve</code>
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
        <div class="space-y-1 max-w-xs">
          <p class="font-bold text-xs text-[var(--text-primary)]">Lokal AI-Assistent</p>
          <p class="text-[11px] text-[var(--text-secondary)]">
            {#if item}
              Klicka på snabbknapparna ovan eller ställ en fråga om <span class="font-mono text-white font-semibold">{item.name}</span>.
            {:else}
              Välj en fil i filutforskaren för att analysera den med din lokala modell ({$selectedModel}).
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
                class="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-[var(--bg-hover)] text-[var(--text-muted)] hover:text-white transition-opacity"
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
          class="px-3 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold flex items-center gap-1 shadow transition-colors"
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
