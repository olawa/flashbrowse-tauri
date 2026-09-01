<script lang="ts">
  import {
    Copy,
    Check,
    WrapText,
    Search,
    X,
    Code,
    FileText,
    Hash,
  } from 'lucide-svelte';

  export let code: string = '';
  export let filename: string = '';
  export let language: string = 'plaintext';
  export let languageName: string = 'Plain Text';
  export let languageEmoji: string = '📄';
  export let formattedSize: string = '--';

  let wrapLines = false;
  let showLineNumbers = true;
  let copied = false;
  let isSearchOpen = false;
  let searchQuery = '';
  let searchInputEl: HTMLInputElement;
  let scrollContainerEl: HTMLElement;

  export function scrollByDelta(deltaY: number) {
    scrollContainerEl?.scrollBy({ top: deltaY, behavior: 'auto' });
  }

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

  function escapeHtml(str: string): string {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  }

  const codeKeywords: Record<string, string[]> = {
    python: ['def', 'class', 'import', 'from', 'as', 'return', 'if', 'elif', 'else', 'for', 'while', 'in', 'is', 'not', 'and', 'or', 'try', 'except', 'finally', 'with', 'lambda', 'yield', 'pass', 'break', 'continue', 'None', 'True', 'False', 'self', 'async', 'await'],
    rust: ['fn', 'let', 'mut', 'pub', 'struct', 'enum', 'impl', 'trait', 'for', 'in', 'if', 'else', 'match', 'return', 'use', 'mod', 'crate', 'self', 'Self', 'async', 'await', 'where', 'type', 'const', 'static', 'ref', 'move', 'true', 'false', 'Some', 'None', 'Ok', 'Err'],
    javascript: ['function', 'const', 'let', 'var', 'return', 'if', 'else', 'for', 'while', 'import', 'export', 'default', 'from', 'as', 'class', 'extends', 'new', 'this', 'async', 'await', 'try', 'catch', 'finally', 'throw', 'true', 'false', 'null', 'undefined'],
    typescript: ['function', 'const', 'let', 'var', 'return', 'if', 'else', 'for', 'while', 'import', 'export', 'default', 'from', 'as', 'class', 'extends', 'interface', 'type', 'enum', 'implements', 'new', 'this', 'async', 'await', 'true', 'false', 'null', 'undefined', 'string', 'number', 'boolean', 'any'],
    shell: ['if', 'then', 'else', 'elif', 'fi', 'for', 'in', 'do', 'done', 'while', 'case', 'esac', 'function', 'return', 'exit', 'export', 'local', 'source', 'echo', 'cd', 'mkdir', 'rm', 'cp', 'mv'],
    r: ['function', 'return', 'if', 'else', 'for', 'in', 'while', 'repeat', 'break', 'next', 'TRUE', 'FALSE', 'NULL', 'NA', 'library', 'require'],
    swift: ['func', 'let', 'var', 'class', 'struct', 'enum', 'protocol', 'extension', 'public', 'private', 'fileprivate', 'internal', 'open', 'override', 'import', 'return', 'if', 'else', 'guard', 'switch', 'case', 'default', 'for', 'in', 'while', 'self', 'Self', 'true', 'false', 'nil', 'async', 'await', 'throws', 'try'],
    c: ['int', 'char', 'float', 'double', 'void', 'long', 'short', 'signed', 'unsigned', 'struct', 'union', 'typedef', 'enum', 'extern', 'static', 'const', 'volatile', 'auto', 'register', 'sizeof', 'if', 'else', 'switch', 'case', 'default', 'for', 'do', 'while', 'break', 'continue', 'return', 'goto', 'NULL'],
    cpp: ['class', 'public', 'private', 'protected', 'virtual', 'template', 'typename', 'namespace', 'using', 'new', 'delete', 'this', 'friend', 'inline', 'constexpr', 'nullptr', 'bool', 'true', 'false', 'int', 'char', 'void', 'if', 'else', 'for', 'while', 'return', 'include'],
    go: ['package', 'import', 'func', 'return', 'var', 'type', 'struct', 'interface', 'chan', 'map', 'go', 'select', 'defer', 'if', 'else', 'for', 'range', 'switch', 'case', 'default', 'break', 'continue', 'fallthrough', 'nil', 'true', 'false'],
    java: ['public', 'private', 'protected', 'class', 'interface', 'enum', 'extends', 'implements', 'new', 'this', 'super', 'void', 'int', 'boolean', 'return', 'if', 'else', 'for', 'while', 'try', 'catch', 'finally', 'throw', 'throws', 'import', 'package', 'static', 'final', 'true', 'false', 'null'],
    kotlin: ['fun', 'val', 'var', 'class', 'object', 'interface', 'package', 'import', 'return', 'if', 'else', 'when', 'for', 'while', 'is', 'in', 'null', 'true', 'false', 'override', 'private', 'public', 'internal'],
  };

  // Syntax highlighting helper
  function highlightLine(line: string, lang: string): string {
    if (!line) return '&nbsp;';

    // If plaintext or non-code format, do not run code token coloring
    if (!lang || lang === 'plaintext' || lang === 'text' || !codeKeywords[lang]) {
      const escaped = escapeHtml(line);
      if (searchQuery.trim()) {
        const qEscaped = searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        const sRegex = new RegExp(`(${qEscaped})`, 'gi');
        return escaped.replace(sRegex, '<mark class="bg-amber-400 text-black px-0.5 rounded font-bold">$1</mark>');
      }
      return escaped;
    }

    // Comments handling
    if (['python', 'shell', 'r', 'yaml', 'toml'].includes(lang)) {
      const commentIdx = line.indexOf('#');
      if (commentIdx !== -1) {
        const before = line.slice(0, commentIdx);
        const comment = line.slice(commentIdx);
        return colorizeCode(before, lang) + `<span class="text-slate-500 italic">${escapeHtml(comment)}</span>`;
      }
    } else if (['c', 'cpp', 'rust', 'swift', 'go', 'javascript', 'typescript', 'java', 'kotlin', 'svelte'].includes(lang)) {
      const commentIdx = line.indexOf('//');
      if (commentIdx !== -1) {
        const before = line.slice(0, commentIdx);
        const comment = line.slice(commentIdx);
        return colorizeCode(before, lang) + `<span class="text-slate-500 italic">${escapeHtml(comment)}</span>`;
      }
    }

    return colorizeCode(line, lang);
  }

  function colorizeCode(rawText: string, lang: string): string {
    // 1. Extract string literals to placeholders
    const strings: string[] = [];
    const textWithoutStrings = rawText.replace(/(["'`])(?:(?=(\\?))\2.)*?\1/g, (match) => {
      const idx = strings.length;
      strings.push(match);
      return `___FB_STR_${idx}___`;
    });

    // 2. Escape HTML
    let result = escapeHtml(textWithoutStrings);

    // 3. Highlight numbers
    result = result.replace(/\b(\d+(?:\.\d+)?)\b/g, '<span class="text-amber-300">$1</span>');

    // 4. Highlight keywords
    const kwList = codeKeywords[lang];
    if (kwList && kwList.length > 0) {
      const kwRegex = new RegExp(`\\b(${kwList.join('|')})\\b`, 'g');
      result = result.replace(kwRegex, '<span class="text-purple-400 font-bold">$1</span>');
    }

    // 5. Decorators (@something in Python / TS)
    result = result.replace(/(@[a-zA-Z0-9_.]+)/g, '<span class="text-cyan-300 font-semibold">$1</span>');

    // 6. Function calls (word followed by '(')
    result = result.replace(/\b([a-zA-Z_][a-zA-Z0-9_]*)(?=\()/g, '<span class="text-blue-400 font-medium">$1</span>');

    // 7. Restore strings
    for (let i = 0; i < strings.length; i++) {
      const escapedStr = escapeHtml(strings[i]);
      result = result.replace(`___FB_STR_${i}___`, `<span class="text-emerald-400">${escapedStr}</span>`);
    }

    // 8. Highlight search matches
    if (searchQuery.trim()) {
      const qEscaped = searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const sRegex = new RegExp(`(${qEscaped})`, 'gi');
      result = result.replace(sRegex, '<mark class="bg-amber-400 text-black px-0.5 rounded font-bold">$1</mark>');
    }

    return result;
  }
</script>

<div class="flex flex-col h-full min-h-0 bg-[#12151c] text-slate-200 select-text overflow-hidden font-mono">
  <!-- Code Header Toolbar -->
  <div class="flex items-center justify-between px-3 py-1.5 bg-[#171b24] border-b border-[#252d3d] shrink-0 select-none text-[11px]">
    <div class="flex items-center gap-2 min-w-0">
      <!-- Language Pill -->
      <span class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#202634] text-cyan-300 font-bold border border-[#2c3547] text-[10.5px]">
        <span>{languageEmoji}</span>
        <span>{languageName}</span>
      </span>

      <!-- Stats Pill -->
      <span class="text-[10px] text-slate-400 truncate">
        {lineCount} {lineCount === 1 ? 'rad' : 'rader'} • {formattedSize}
      </span>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-1 shrink-0">
      <!-- Line Numbers Toggle -->
      <button
        class="flex items-center gap-1 px-1.5 py-0.5 rounded transition-colors {showLineNumbers ? 'bg-[#202634] text-slate-300 border border-[#2c3547]' : 'text-slate-500 hover:text-slate-300'}"
        on:click={() => (showLineNumbers = !showLineNumbers)}
        title="Växla radnummer"
      >
        <Hash size={11} />
        <span class="hidden sm:inline">Radnr</span>
      </button>

      <!-- Search Button -->
      <button
        class="flex items-center gap-1 px-1.5 py-0.5 rounded transition-colors {isSearchOpen ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' : 'bg-[#202634] hover:bg-[#2c3547] text-slate-300 border border-[#2c3547]'}"
        on:click={toggleSearch}
        title="Sök i kod"
      >
        <Search size={11} />
        <span class="hidden sm:inline">Sök</span>
      </button>

      <!-- Wrap Lines Toggle -->
      <button
        class="flex items-center gap-1 px-1.5 py-0.5 rounded transition-colors {wrapLines ? 'bg-[var(--accent)] text-white font-bold' : 'bg-[#202634] hover:bg-[#2c3547] text-slate-300 border border-[#2c3547]'}"
        on:click={() => (wrapLines = !wrapLines)}
        title="Växla radbrytning (Wrap lines)"
      >
        <WrapText size={11} />
        <span class="hidden sm:inline">{wrapLines ? 'Wrap' : 'No Wrap'}</span>
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

  <!-- Unified Code Body with Inline Row Numbers (Single Scroll Container) -->
  <div
    bind:this={scrollContainerEl}
    class="flex-1 overflow-auto py-2 px-2 text-[11.5px] leading-5 font-mono select-text bg-[#0d1017] min-h-0"
  >
    {#each lines as line, idx}
      {@const lineNum = idx + 1}
      {@const isMatch = matchedLines.includes(lineNum)}
      <div class="flex items-start hover:bg-white/[0.04] transition-colors {isMatch ? 'bg-amber-400/10' : ''}">
        {#if showLineNumbers}
          <span class="select-none text-right text-slate-600 w-11 pr-2 shrink-0 border-r border-[#202533] mr-2.5 font-mono text-[11px]">
            {lineNum}
          </span>
        {/if}
        <div class="flex-1 min-w-0 {wrapLines ? 'whitespace-pre-wrap break-all' : 'whitespace-pre'}">
          {@html highlightLine(line, language)}
        </div>
      </div>
    {/each}
  </div>
</div>
