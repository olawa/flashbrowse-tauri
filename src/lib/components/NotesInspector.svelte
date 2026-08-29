<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getDirectoryNotes, saveDirectoryNotes, revealInOs, listDirectory } from '../invoke';
  import { renderMarkdown } from '../markdown';
  import { isOllamaOnline, askOllamaStream, selectedModel, isAiGenerating } from '../stores/ollamaStore';
  import type { DirectoryNotes } from '../types';
  import {
    NotebookPen,
    Edit3,
    Eye,
    Save,
    Columns,
    Sparkles,
    CheckSquare,
    Clock,
    FlaskConical,
    Copy,
    Check,
    FolderOpen,
    FileText,
    RefreshCw,
  } from 'lucide-svelte';

  export let dirPath: string;

  let notes: DirectoryNotes | null = null;
  let noteContent = '';
  let isLoading = false;
  let isSaving = false;
  let isDirty = false;
  let lastSavedText = '';
  let viewMode: 'rendered' | 'edit' | 'split' = 'rendered';
  let copied = false;
  let autoSaveTimeout: any = null;
  let textareaEl: HTMLTextAreaElement;

  $: if (dirPath) {
    loadNotes(dirPath);
  }

  async function loadNotes(path: string) {
    isLoading = true;
    isDirty = false;
    try {
      notes = await getDirectoryNotes(path);
      noteContent = notes.content;
      if (notes.last_modified) {
        lastSavedText = `Sparad ${notes.last_modified}`;
      } else {
        lastSavedText = notes.exists ? 'Laddad' : 'Ny anteckning';
      }
      // If note is empty, open in edit mode by default
      if (!notes.exists || !notes.content.trim()) {
        viewMode = 'edit';
      }
    } catch (e) {
      console.error('Failed to load directory notes:', e);
    } finally {
      isLoading = false;
    }
  }

  function handleInput() {
    isDirty = true;
    lastSavedText = 'Osparade ändringar...';
    if (autoSaveTimeout) clearTimeout(autoSaveTimeout);
    autoSaveTimeout = setTimeout(() => {
      saveNotes();
    }, 1500);
  }

  async function saveNotes() {
    if (!dirPath || isSaving) return;
    if (autoSaveTimeout) clearTimeout(autoSaveTimeout);
    isSaving = true;
    try {
      notes = await saveDirectoryNotes(dirPath, noteContent, notes?.filename);
      isDirty = false;
      lastSavedText = notes.last_modified ? `Sparad ${notes.last_modified}` : 'Sparad';
    } catch (e: any) {
      lastSavedText = `Kunde inte spara: ${e}`;
    } finally {
      isSaving = false;
    }
  }

  function insertTemplate(type: 'experiment' | 'todo' | 'timestamp') {
    const now = new Date();
    const dateStr = now.toISOString().slice(0, 10);
    const timeStr = now.toTimeString().slice(0, 5);

    let snippet = '';
    if (type === 'experiment') {
      snippet = `\n## 🧪 Experiment: [Titel]\n- **Datum:** ${dateStr} ${timeStr}\n- **Mål / Hypotes:** \n- **Kommando / Script:**\n\`\`\`bash\n# kör kommando här\n\`\`\`\n- **Resultat & Observationer:**\n- **Slutsats & Nästa steg:**\n  - [ ] Analysera data\n`;
    } else if (type === 'todo') {
      snippet = `\n### 📋 Att göra (${dateStr})\n- [ ] Första uppgiften\n- [ ] Nästa steg\n- [ ] Kvalitetskontroll\n`;
    } else if (type === 'timestamp') {
      snippet = `\n### 🕒 ${dateStr} ${timeStr}\n`;
    }

    if (textareaEl) {
      const start = textareaEl.selectionStart || noteContent.length;
      const end = textareaEl.selectionEnd || noteContent.length;
      noteContent = noteContent.slice(0, start) + snippet + noteContent.slice(end);
      isDirty = true;
      handleInput();
      setTimeout(() => {
        textareaEl.focus();
        textareaEl.setSelectionRange(start + snippet.length, start + snippet.length);
      }, 50);
    } else {
      noteContent = (noteContent ? noteContent + '\n' : '') + snippet;
      isDirty = true;
      handleInput();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    // Cmd+S or Ctrl+S to save
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault();
      saveNotes();
    }
    // Tab key support in textarea
    if (e.key === 'Tab' && textareaEl && document.activeElement === textareaEl) {
      e.preventDefault();
      const start = textareaEl.selectionStart;
      const end = textareaEl.selectionEnd;
      noteContent = noteContent.substring(0, start) + '  ' + noteContent.substring(end);
      setTimeout(() => {
        textareaEl.selectionStart = textareaEl.selectionEnd = start + 2;
      }, 0);
      handleInput();
    }
  }

  async function copyAll() {
    if (!noteContent) return;
    try {
      await navigator.clipboard.writeText(noteContent);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {}
  }

  async function handleAiDraft() {
    if ($isAiGenerating) return;
    isLoading = true;
    try {
      const items = await listDirectory(dirPath, false);
      const topFiles = items.slice(0, 20).map((f) => `- ${f.name} (${f.formatted_size})`).join('\n');
      const prompt = `Jag har en projektmapp på sökvägen "${dirPath}" med följande filer:\n${topFiles}\n\nSkriv ett snyggt och strukturerat Markdown-utkast för NOTES.md / lab-anteckningar för denna mapp. Inkludera en kort sammanfattning av mappen, syfte, datatyper och en punktlista med föreslagna nästa steg / att-göra.`;

      viewMode = 'edit';
      await askOllamaStream(prompt, 'Du är en hjälpsam bioinformatik- och programmeringsassistent som skriver strukturerade labanteckningar.', '');
    } catch (e) {
      console.error('AI draft failed:', e);
    } finally {
      isLoading = false;
    }
  }

  onDestroy(() => {
    if (autoSaveTimeout) clearTimeout(autoSaveTimeout);
  });
</script>

<svelte:window on:keydown={handleKeyDown} />

<div class="flex-1 flex flex-col h-full overflow-hidden bg-[#0d0f14] text-slate-200 text-xs select-none">
  <!-- Top Toolbar -->
  <div class="px-3 py-2 bg-[#151922] border-b border-[#252d3d] flex items-center justify-between gap-2 shrink-0">
    <div class="flex items-center gap-2 min-w-0">
      <div class="w-6 h-6 rounded bg-amber-500/20 text-amber-400 flex items-center justify-center shrink-0">
        <NotebookPen size={14} />
      </div>
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1.5 flex-wrap">
          <span class="font-bold text-xs text-white">{notes?.filename || 'NOTES.md'}</span>
          {#if isDirty}
            <span class="w-2 h-2 rounded-full bg-amber-400 animate-pulse" title="Osparade ändringar"></span>
          {/if}
          <span class="text-[10px] text-slate-400 font-mono">{lastSavedText}</span>
        </div>
        <div class="text-[10px] text-slate-500 font-mono truncate" title={dirPath}>
          {dirPath}
        </div>
      </div>
    </div>

    <!-- Right: View Mode, Save, Actions -->
    <div class="flex items-center gap-1.5 shrink-0">
      <!-- Mode Toggle -->
      <div class="flex items-center bg-[#0e1015] rounded-md p-0.5 border border-[#252d3d]">
        <button
          class="px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors flex items-center gap-1 {viewMode === 'rendered' ? 'bg-amber-500/20 text-amber-300 font-bold' : 'text-slate-400 hover:text-white'}"
          on:click={() => (viewMode = 'rendered')}
          title="Visa formaterad Markdown"
        >
          <Eye size={11} />
          <span>Visning</span>
        </button>

        <button
          class="px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors flex items-center gap-1 {viewMode === 'edit' ? 'bg-amber-500/20 text-amber-300 font-bold' : 'text-slate-400 hover:text-white'}"
          on:click={() => (viewMode = 'edit')}
          title="Redigera Markdown-källkod"
        >
          <Edit3 size={11} />
          <span>Redigera</span>
        </button>

        <button
          class="hidden sm:flex px-2 py-0.5 rounded text-[10.5px] font-medium transition-colors items-center gap-1 {viewMode === 'split' ? 'bg-amber-500/20 text-amber-300 font-bold' : 'text-slate-400 hover:text-white'}"
          on:click={() => (viewMode = 'split')}
          title="Delad vy med editor och förhandsgranskning"
        >
          <Columns size={11} />
          <span>Delad</span>
        </button>
      </div>

      <!-- Save button -->
      <button
        class="px-2.5 py-1 rounded bg-amber-600 hover:bg-amber-500 text-white font-semibold text-xs shadow transition-colors flex items-center gap-1 disabled:opacity-50"
        on:click={saveNotes}
        disabled={isSaving}
        title="Spara (Cmd+S)"
      >
        {#if isSaving}
          <RefreshCw size={11} class="animate-spin" />
        {:else}
          <Save size={11} />
        {/if}
        <span>Spara</span>
      </button>

      <!-- Copy -->
      <button
        class="p-1 rounded bg-[#0e1015] hover:bg-white/10 border border-[#252d3d] text-slate-400 hover:text-white transition-colors"
        on:click={copyAll}
        title="Kopiera anteckningar"
      >
        {#if copied}
          <Check size={12} class="text-emerald-400" />
        {:else}
          <Copy size={12} />
        {/if}
      </button>

      <!-- Reveal in Finder -->
      {#if notes?.exists}
        <button
          class="p-1 rounded bg-[#0e1015] hover:bg-white/10 border border-[#252d3d] text-slate-400 hover:text-white transition-colors"
          on:click={() => notes && revealInOs(notes.path)}
          title="Visa NOTES.md i Finder"
        >
          <FolderOpen size={12} />
        </button>
      {/if}
    </div>
  </div>

  <!-- Template Snippet Bar -->
  <div class="px-3 py-1 bg-[#12151c] border-b border-[#252d3d] flex items-center justify-between gap-2 overflow-x-auto text-[11px] shrink-0">
    <div class="flex items-center gap-1.5">
      <span class="text-slate-500 text-[10px] uppercase font-mono tracking-wider mr-1">Mallar:</span>

      <button
        class="px-2 py-0.5 rounded bg-[#181d27] hover:bg-amber-500/20 text-slate-300 hover:text-amber-300 border border-[#252d3d] flex items-center gap-1 transition-colors"
        on:click={() => insertTemplate('experiment')}
      >
        <FlaskConical size={11} class="text-emerald-400" />
        <span>Lab Experiment</span>
      </button>

      <button
        class="px-2 py-0.5 rounded bg-[#181d27] hover:bg-amber-500/20 text-slate-300 hover:text-amber-300 border border-[#252d3d] flex items-center gap-1 transition-colors"
        on:click={() => insertTemplate('todo')}
      >
        <CheckSquare size={11} class="text-sky-400" />
        <span>Att göra</span>
      </button>

      <button
        class="px-2 py-0.5 rounded bg-[#181d27] hover:bg-amber-500/20 text-slate-300 hover:text-amber-300 border border-[#252d3d] flex items-center gap-1 transition-colors"
        on:click={() => insertTemplate('timestamp')}
      >
        <Clock size={11} class="text-amber-400" />
        <span>Tidstämpel</span>
      </button>
    </div>

    <!-- AI Helper Button -->
    {#if $isOllamaOnline}
      <button
        class="px-2 py-0.5 rounded bg-purple-950/40 hover:bg-purple-900/60 text-purple-300 border border-purple-800 flex items-center gap-1 transition-colors shrink-0"
        on:click={handleAiDraft}
        title="Generera utkast till anteckningar med AI från mappens filer"
      >
        <Sparkles size={11} class="text-purple-400" />
        <span>AI Utkast</span>
      </button>
    {/if}
  </div>

  <!-- Body Content -->
  <div class="flex-1 min-h-0 flex overflow-hidden">
    {#if isLoading}
      <div class="flex-1 flex items-center justify-center text-slate-400 gap-2">
        <RefreshCw size={16} class="animate-spin text-amber-400" />
        <span>Laddar anteckningar...</span>
      </div>

    {:else if viewMode === 'rendered'}
      <!-- Rendered Markdown View -->
      <div class="flex-1 overflow-y-auto p-4 select-text leading-relaxed">
        {#if noteContent.trim()}
          <div class="prose prose-invert max-w-none text-slate-200">
            {@html renderMarkdown(noteContent)}
          </div>
        {:else}
          <div class="h-full flex flex-col items-center justify-center p-8 text-center text-slate-500 space-y-3">
            <FileText size={32} class="opacity-30" />
            <p>Inga anteckningar i denna mapp ännu.</p>
            <button
              class="px-3 py-1.5 rounded-lg bg-amber-600 hover:bg-amber-500 text-white font-semibold text-xs shadow flex items-center gap-1.5 transition-colors"
              on:click={() => (viewMode = 'edit')}
            >
              <Edit3 size={13} />
              <span>Börja skriva</span>
            </button>
          </div>
        {/if}
      </div>

    {:else if viewMode === 'edit'}
      <!-- Full-height Editor -->
      <div class="flex-1 flex flex-col p-2 bg-[#0c0e14]">
        <textarea
          bind:this={textareaEl}
          bind:value={noteContent}
          on:input={handleInput}
          placeholder="Skriv dina Markdown-anteckningar här... (Cmd+S för att spara)"
          class="flex-1 w-full bg-transparent text-slate-200 font-mono text-xs p-2 leading-relaxed resize-none focus:outline-none placeholder:text-slate-600 select-text"
          spellcheck="false"
        ></textarea>
      </div>

    {:else if viewMode === 'split'}
      <!-- Split Editor & Rendered View -->
      <div class="flex-1 flex divide-x divide-[#252d3d] overflow-hidden">
        <div class="w-1/2 flex flex-col p-2 bg-[#0c0e14] overflow-hidden">
          <textarea
            bind:this={textareaEl}
            bind:value={noteContent}
            on:input={handleInput}
            placeholder="Skriv dina Markdown-anteckningar här..."
            class="flex-1 w-full bg-transparent text-slate-200 font-mono text-xs p-2 leading-relaxed resize-none focus:outline-none placeholder:text-slate-600 select-text"
            spellcheck="false"
          ></textarea>
        </div>
        <div class="w-1/2 overflow-y-auto p-4 select-text leading-relaxed bg-[#0d0f14]">
          <div class="prose prose-invert max-w-none text-slate-200">
            {@html renderMarkdown(noteContent)}
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>
