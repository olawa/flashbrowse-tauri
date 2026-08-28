<script lang="ts">
  import {
    Copy,
    Check,
    WrapText,
    Search,
    X,
    Code,
    FileText,
    ChevronUp,
    ChevronDown,
  } from 'lucide-svelte';

  export let code: string = '';
  export let filename: string = '';
  export let language: string = 'plaintext';
  export let languageName: string = 'Plain Text';
  export let languageEmoji: string = '📄';
  export let formattedSize: string = '--';

  let wrapLines = false;
  let copied = false;
  let isSearchOpen = false;
  let searchQuery = '';
  let searchInputEl: HTMLInputElement;

  $: lines = code ? code.split('\n') : [];
  $: lineCount = lines.length;

  // Search matches
  $: matchedLines = searchQuery.trim()
    ? lines
        .map((line, idx) => (line.toLowerCase().includes(searchQuery.toLowerCase()) ? idx + 1 : null))
        .filter((idx): idx is number => idx !== null)
    : [];

  async function copyCode() {
    if (!code) return;
    try {
      await navigator.clipboard.writeText(code);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (err) {
      console.warn('Copy failed:', err);
    }
  }

  function toggleSearch() {
    isSearchOpen = !isSearchOpen;
    if (isSearchOpen) {
      setTimeout(() => searchInputEl?.focus(), 50);
    } else {
      searchQuery = '';
    }
  }

  // Syntax highlighting helper for common programming languages
  function highlightLine(line: string, lang: string): string {
    if (!line) return '&nbsp;';

    // 1. HTML encode line
    let escaped = line
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');

    // 2. Comments (Python #, C-style //, SQL --)
    if (lang === 'python' || lang === 'shell' || lang === 'r' || lang === 'yaml' || lang === 'toml') {
      const commentIdx = escaped.indexOf('#');
      if (commentIdx !== -1) {
        const before = escaped.slice(0, commentIdx);
        const comment = escaped.slice(commentIdx);
        return colorizeTokens(before, lang) + `<span class="text-slate-500 italic">${comment}</span>`;
      }
    } else if (['c', 'cpp', 'rust', 'swift', 'go', 'javascript', 'typescript', 'java', 'kotlin', 'svelte'].includes(lang)) {
      const commentIdx = escaped.indexOf('//');
      if (commentIdx !== -1) {
        const before = escaped.slice(0, commentIdx);
        const comment = escaped.slice(commentIdx);
        return colorizeTokens(before, lang) + `<span class="text-slate-500 italic">${comment}</span>`;
      }
    }

    return colorizeTokens(escaped, lang);
  }

  function colorizeTokens(text: string, lang: string): string {
    // Strings in quotes
    let result = text.replace(/(&quot;.*?&quot;|&#039;.*?&#039;)/g, '<span class="text-emerald-400">$1</span>');

    // Numbers
    result = result.replace(/\b(\d+(?:\.\d+)?)\b/g, '<span class="text-amber-300">$1</span>');

    // Python / Rust / JS / C keywords
    const keywords: Record<string, string[]> = {
      python: ['def', 'class', 'import', 'from', 'as', 'return', 'if', 'elif', 'else', 'for', 'while', 'in', 'is', 'not', 'and', 'or', 'try', 'except', 'finally', 'with', 'lambda', 'yield', 'pass', 'break', 'continue', 'None', 'True', 'False', 'self', 'async', 'await'],
      rust: ['fn', 'let', 'mut', 'pub', 'struct', 'enum', 'impl', 'trait', 'for', 'in', 'if', 'else', 'match', 'return', 'use', 'mod', 'crate', 'self', 'Self', 'async', 'await', 'where', 'type', 'const', 'static', 'ref', 'move', 'true', 'false', 'Some', 'None', 'Ok', 'Err'],
      javascript: ['function', 'const', 'let', 'var', 'return', 'if', 'else', 'for', 'while', 'import', 'export', 'default', 'from', 'as', 'class', 'extends', 'new', 'this', 'async', 'await', 'try', 'catch', 'finally', 'throw', 'true', 'false', 'null', 'undefined'],
      typescript: ['function', 'const', 'let', 'var', 'return', 'if', 'else', 'for', 'while', 'import', 'export', 'default', 'from', 'as', 'class', 'extends', 'interface', 'type', 'enum', 'implements', 'new', 'this', 'async', 'await', 'true', 'false', 'null', 'undefined', 'string', 'number', 'boolean', 'any'],
      shell: ['if', 'then', 'else', 'elif', 'fi', 'for', 'in', 'do', 'done', 'while', 'case', 'esac', 'function', 'return', 'exit', 'export', 'local', 'source', 'echo', 'cd', 'mkdir', 'rm', 'cp', 'mv'],
      r: ['function', 'return', 'if', 'else', 'for', 'in', 'while', 'repeat', 'break', 'next', 'TRUE', 'FALSE', 'NULL', 'NA', 'library', 'require'],
      swift: ['func', 'let', 'var', 'class', 'struct', 'enum', 'protocol', 'extension', 'public', 'private', 'fileprivate', 'internal', 'open', 'override', 'import', 'return', 'if', 'else', 'guard', 'switch', 'case', 'default', 'for', 'in', 'while', 'self', 'Self', 'true', 'false', 'nil', 'async', 'await', 'throws', 'try'],
    };

    const kwList = keywords[lang] || ['function', 'return', 'if', 'else', 'for', 'while', 'import', 'class', 'true', 'false'];
    const kwRegex = new RegExp(`\\b(${kwList.join('|')})\\b`, 'g');
    result = result.replace(kwRegex, '<span class="text-purple-400 font-bold">$1</span>');

    // Decorators (@something in Python / TS)
    result = result.replace(/(@[a-zA-Z0-9_.]+)/g, '<span class="text-cyan-300 font-semibold">$1</span>');

    // Function calls (word followed by '(')
    result = result.replace(/\b([a-zA-Z_][a-zA-Z0-9_]*)(?=\()/g, '<span class="text-blue-400 font-medium">$1</span>');

    // Highlight search matches
    if (searchQuery.trim()) {
      const qEscaped = searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const sRegex = new RegExp(`(${qEscaped})`, 'gi');
      result = result.replace(sRegex, '<mark class="bg-amber-400 text-black px-0.5 rounded font-bold">$1</mark>');
    }

    return result;
  }
</script>

<div class="flex flex-col h-full bg-[#12151c] text-slate-200 select-text overflow-hidden font-mono">
  <!-- Code Header Toolbar -->
  <div class="flex items-center justify-between px-3 py-1.5 bg-[#171b24] border-b border-[#252d3d] shrink-0 select-none text-[11px]">
    <div class="flex items-center gap-2 min-w-0">
      <!-- Language Pill -->
      <span class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#202634] text-cyan-300 font-bold border border-[#2c3547] text-[10.5px]">
        <span>{languageEmoji}</span>
        <span>{languageName}</span>
      </span>

      <!-- Stats Pill -->
      <span class="text-[10px] text-slate-400">
        {lineCount} {lineCount === 1 ? 'rad' : 'rader'} • {formattedSize}
      </span>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-1">
      <!-- Search Button -->
      <button
        class="flex items-center gap-1 px-2 py-0.5 rounded transition-colors {isSearchOpen ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' : 'bg-[#202634] hover:bg-[#2c3547] text-slate-300 border border-[#2c3547]'}"
        on:click={toggleSearch}
        title="Sök i kod"
      >
        <Search size={11} />
        <span>Sök</span>
      </button>

      <!-- Wrap Lines Toggle -->
      <button
        class="flex items-center gap-1 px-2 py-0.5 rounded transition-colors {wrapLines ? 'bg-[var(--accent)] text-white font-bold' : 'bg-[#202634] hover:bg-[#2c3547] text-slate-300 border border-[#2c3547]'}"
        on:click={() => (wrapLines = !wrapLines)}
        title="Växla radbrytning (Wrap lines)"
      >
        <WrapText size={11} />
        <span>{wrapLines ? 'Radbryt: På' : 'Radbryt: Av'}</span>
      </button>

      <!-- Copy Button -->
      <button
        class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#202634] hover:bg-[#2c3547] text-slate-300 hover:text-white border border-[#2c3547] transition-colors"
        on:click={copyCode}
        title="Kopiera all källkod"
      >
        {#if copied}
          <Check size={11} class="text-green-400" />
          <span class="text-green-400 font-semibold">Kopierad</span>
        {:else}
          <Copy size={11} />
          <span>Kopiera</span>
        {/if}
      </button>
    </div>
  </div>

  <!-- Search Bar (Collapsible) -->
  {#if isSearchOpen}
    <div class="flex items-center gap-2 px-3 py-1.5 bg-[#1b202c] border-b border-[#2c3547] shrink-0 text-xs">
      <Search size={12} class="text-amber-400" />
      <input
        bind:this={searchInputEl}
        type="text"
        placeholder="Sök i filen..."
        bind:value={searchQuery}
        class="flex-1 bg-[#12151c] text-white px-2 py-0.5 rounded border border-[#2c3547] focus:outline-none focus:border-amber-400 text-xs"
      />
      {#if searchQuery}
        <span class="text-[10px] text-slate-400 font-mono">
          {matchedLines.length} {matchedLines.length === 1 ? 'träff' : 'träffar'}
        </span>
      {/if}
      <button class="p-1 text-slate-400 hover:text-white" on:click={toggleSearch}>
        <X size={12} />
      </button>
    </div>
  {/if}

  <!-- Code Body with Gutter -->
  <div class="flex-1 overflow-auto flex text-[11.5px] leading-5 font-mono select-text bg-[#0d1017]">
    <!-- Line Number Gutter -->
    <div class="select-none py-2 px-2 text-right text-slate-600 bg-[#12151c] border-r border-[#202533] shrink-0 min-w-[40px]">
      {#each lines as _, idx}
        {@const lineNum = idx + 1}
        {@const isMatch = matchedLines.includes(lineNum)}
        <div class="{isMatch ? 'text-amber-400 font-bold bg-amber-400/10' : ''}">{lineNum}</div>
      {/each}
    </div>

    <!-- Code Content -->
    <div class="flex-1 py-2 px-3 overflow-x-auto min-w-0 {wrapLines ? 'whitespace-pre-wrap break-all' : 'whitespace-pre'}">
      {#each lines as line, idx}
        {@const lineNum = idx + 1}
        {@const isMatch = matchedLines.includes(lineNum)}
        <div class="hover:bg-white/[0.03] transition-colors {isMatch ? 'bg-amber-400/10' : ''}">
          {@html highlightLine(line, language)}
        </div>
      {/each}
    </div>
  </div>
</div>
