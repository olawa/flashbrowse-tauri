<script lang="ts">
  import { onMount } from 'svelte';
  import {
    initNavigation,
    leftPane,
    rightPane,
    isDualPane,
    isDualInspector,
    inspectorPreset,
    isInspectorDetached,
    activePaneId,
    activeHoveredItem,
    isInspectorLocked,
    toggleInspectorLock,
    reloadPane,
    type InspectorPreset,
    isTransferring,
    transferStatus,
    transferProgress,
    cancelActiveTransfer,
    transferBetweenPanes,
    triggerInspectorScroll,
    navigatePane,
    layoutMode,
    setLayoutMode,
  } from '$lib/stores/navigation';
  import {
    isTerminalOpen,
    terminalDockPosition,
    toggleTerminal,
    toggleTerminalDock,
  } from '$lib/stores/terminal';
  import { currentTheme, isKidsMode, setTheme } from '$lib/stores/theme';
  import {
    sidebarWidth,
    inspectorWidth,
    dualInspectorWidth,
    terminalHeight,
    terminalWidth,
  } from '$lib/stores/layoutStore';
  import ResizeHandle from '$lib/components/ResizeHandle.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Breadcrumb from '$lib/components/Breadcrumb.svelte';
  import FileTable from '$lib/components/FileTable.svelte';
  import Inspector from '$lib/components/Inspector.svelte';
  import DetachedInspectorView from '$lib/components/DetachedInspectorView.svelte';
  import IndexBrowserView from '$lib/components/IndexBrowserView.svelte';
  import Terminal from '$lib/components/Terminal.svelte';
  import StashShelf from '$lib/components/StashShelf.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import CleanSidebar from '$lib/components/CleanSidebar.svelte';
  import CleanSearchBar from '$lib/components/CleanSearchBar.svelte';
  import CleanSearchResults from '$lib/components/CleanSearchResults.svelte';
  import CleanAnswer from '$lib/components/CleanAnswer.svelte';
  import GenomicsTrackHub from '$lib/components/GenomicsTrackHub.svelte';
  import { toggleStash } from '$lib/stores/stash';
  import { isGenomicsHubOpen } from '$lib/stores/genomicsStore';
  import { activeIndexMeta, closeIndexView, refreshCurrentIndex } from '$lib/stores/indexStore';
  import { saveNotification } from '$lib/stores/downloadStore';
  import { openInFavoriteEditor } from '$lib/stores/editorStore';
  import type { FileItem, SearchMatch } from '$lib/types';
  import { get } from 'svelte/store';
  import { openInDefault } from '$lib/invoke';
  import { askOllamaStream, isOllamaOnline, selectedModel, isAiGenerating, aiChatMessages } from '$lib/stores/ollamaStore';
  import {
    Lock,
    Unlock,
    PanelRightClose,
    Folder,
    FileText,
    Dna,
    Table,
    Copy,
    Check,
    AlertCircle,
    LayoutTemplate,
    Columns2,
    Columns3,
    Square,
    EyeOff,
    Terminal as TerminalIcon,
  } from 'lucide-svelte';

  // MARK: - Clean mode
  //
  // The search field owns the main surface here, so its state lives with the
  // shell rather than inside the browser pane.
  let cleanMatches: SearchMatch[] | null = null;
  let cleanSearchBar: CleanSearchBar;
  let cleanQuestion = '';
  let cleanAnswer = '';
  let cleanAskError = '';
  let cleanAskSeconds: number | null = null;

  $: cleanRootLabel = (() => {
    const path = $leftPane.currentPath;
    if (!path) return 'mappen';
    const name = path.split('/').filter(Boolean).pop();
    return name || '/';
  })();

  /** A search hit shown as the inspector's subject. */
  function searchMatchToItem(match: SearchMatch): FileItem {
    const extension = match.is_dir ? '' : (match.name.split('.').pop() ?? '');
    return {
      name: match.name,
      path: match.path,
      is_dir: match.is_dir,
      is_symlink: false,
      size_bytes: 0,
      formatted_size: match.formatted_size,
      modified_timestamp: 0,
      formatted_modified: '',
      extension,
      permissions: '',
    } as FileItem;
  }

  function openSearchMatch(match: SearchMatch) {
    if (match.is_dir) {
      navigatePane('left', match.path);
      cleanSearchBar?.clear();
    } else {
      openInDefault(match.path).catch((e) => console.warn('Kunde inte öppna:', e));
    }
  }

  /**
   * The model answers questions about content; it never navigates and never
   * moves anything. The answer appears in its own framed panel, beside the
   * files it is about.
   */
  async function askAboutFiles(question: string) {
    cleanQuestion = question;
    cleanAnswer = '';
    cleanAskError = '';
    cleanAskSeconds = null;

    if (!$isOllamaOnline || !$selectedModel) {
      cleanAskError = 'Ingen lokal modell är igång. Starta Ollama och välj en modell i pro-läget.';
      return;
    }

    const started = performance.now();
    const listing = $leftPane.items
      .slice(0, 200)
      .map((i) => `${i.is_dir ? 'MAPP' : 'FIL '} ${i.name} (${i.formatted_size}, ${i.formatted_modified})`)
      .join('\n');

    try {
      await askOllamaStream(
        question,
        'Du svarar på frågor om filer i en mapp. Svara kort på svenska. Om listan inte räcker för att svara, säg det rakt ut i stället för att gissa.',
        `Mapp: ${$leftPane.currentPath}\n\n${listing}`
      );
      const last = get(aiChatMessages).filter((m) => m.role === 'assistant').pop();
      cleanAnswer = last?.content ?? '';
      cleanAskSeconds = (performance.now() - started) / 1000;
    } catch (e: any) {
      cleanAskError = String(e?.message ?? e);
    }
  }

  function closeCleanAnswer() {
    cleanQuestion = '';
    cleanAnswer = '';
    cleanAskError = '';
  }

  let leftPreviewItem: FileItem | null = null;
  let rightPreviewItem: FileItem | null = null;
  let isPaletteOpen = false;
  let isDetachedWindowMode = typeof window !== 'undefined' && (
    (window as any).__FLASHBROWSE_WINDOW__ === 'inspector' ||
    (window as any).__TAURI_INTERNALS__?.metadata?.currentWindow?.label === 'inspector' ||
    new URLSearchParams(window.location.search).get('window') === 'inspector'
  );

  // Kids Mode Pin Lock
  let isPinModalOpen = false;
  let pinInput = '';
  let pinError = '';

  onMount(async () => {
    if (!isDetachedWindowMode && typeof window !== 'undefined') {
      try {
        const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const win = getCurrentWebviewWindow();
        if (win && win.label === 'inspector') {
          isDetachedWindowMode = true;
        }
      } catch {}
    }

    if (isDetachedWindowMode) {
      return;
    }

    await initNavigation();
    setTheme($currentTheme);

    // Default select first items for instant preview
    if ($leftPane.items.length > 0) {
      leftPreviewItem = $leftPane.items[0];
    }
    if ($rightPane.items.length > 0) {
      rightPreviewItem = $rightPane.items[0];
    }
  });

  $: if ($leftPane.items.length && !leftPreviewItem) {
    leftPreviewItem = $leftPane.items[0];
  }
  $: if ($rightPane.items.length && !rightPreviewItem) {
    rightPreviewItem = $rightPane.items[0];
  }

  // Active Shared Preview Item for Center and Right layouts
  $: activeSharedItem =
    $activeHoveredItem ||
    ($activePaneId === 'right' ? (rightPreviewItem || leftPreviewItem) : (leftPreviewItem || rightPreviewItem));

  $: activeSharedTitle = $activeHoveredItem
    ? ($activePaneId === 'right' ? 'Höger (hovrad)' : 'Vänster (hovrad)')
    : ($activePaneId === 'right' ? 'Höger' : 'Vänster');

  let lastFocusedZone: 'list' | 'inspector' = 'list';

  async function toggleFocusAndPointerBetweenListAndInspector() {
    // 1. If Inspector is detached in its own window
    if ($isInspectorDetached) {
      try {
        const { toggleMouseBetweenWindows } = await import('$lib/invoke');
        await toggleMouseBetweenWindows();
      } catch (err) {
        console.warn('Failed to toggle mouse between windows:', err);
      }
      return;
    }

    // 2. If Inspector is visible in main window
    const inspectorEl = document.querySelector('[data-inspector-root="true"]') as HTMLElement | null;
    const fileTableEl = document.querySelector('[role="table"], [role="tree"], [data-file-table]') as HTMLElement | null;

    if (!inspectorEl) return;

    if (lastFocusedZone === 'list') {
      const rect = inspectorEl.getBoundingClientRect();
      const clientX = rect.left + rect.width / 2;
      const clientY = Math.min(rect.top + 220, rect.top + rect.height / 2);
      try {
        const { warpMouseToClientPos } = await import('$lib/invoke');
        await warpMouseToClientPos('main', clientX, clientY);
      } catch (err) {
        console.warn('Failed to warp mouse to inspector:', err);
      }
      inspectorEl.focus();
      lastFocusedZone = 'inspector';
    } else {
      const targetEl = fileTableEl || document.body;
      const rect = targetEl.getBoundingClientRect();
      const clientX = rect.left + Math.min(250, rect.width / 2);
      const clientY = Math.min(rect.top + 220, rect.top + rect.height / 2);
      try {
        const { warpMouseToClientPos } = await import('$lib/invoke');
        await warpMouseToClientPos('main', clientX, clientY);
      } catch (err) {
        console.warn('Failed to warp mouse to file list:', err);
      }
      if (fileTableEl) fileTableEl.focus();
      lastFocusedZone = 'list';
    }
  }

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if (isDetachedWindowMode) return;
    if (e.defaultPrevented) return;
    // Esc: Close Index View if open
    if (e.key === 'Escape' && $activeIndexMeta) {
      e.preventDefault();
      closeIndexView();
      return;
    }

    // Cmd+R / F5: Refresh Index or Panes
    if (((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === 'r') || e.key === 'F5') {
      e.preventDefault();
      if ($activeIndexMeta) {
        refreshCurrentIndex();
      } else {
        reloadPane('left');
        if ($isDualPane) reloadPane('right');
      }
      return;
    }

    // Cmd+E / Ctrl+E: Open active file, folder, or project in favorite editor
    if ((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === 'e') {
      e.preventDefault();
      const activeId = $activePaneId;
      const currentPane = activeId === 'left' ? $leftPane : $rightPane;
      const selected = Array.from(currentPane.selectedPaths)[0];
      const targetPath = selected || currentPane.currentPath;
      if (targetPath) {
        openInFavoriteEditor(targetPath, currentPane.isSSH, currentPane.sshHost);
      }
      return;
    }

    // Cmd + < / Cmd + > / Cmd + § / Cmd + ` / IntlBackslash:
    // Move focus AND mouse pointer between file list and inspector (matching Swift Flashbrowse toggleMouseBetweenScreens)
    const isToggleInspectorMouse =
      (e.metaKey || e.ctrlKey) &&
      (e.key === '<' ||
        e.key === '>' ||
        e.key === '§' ||
        e.key === '±' ||
        e.key === '`' ||
        e.key === '~' ||
        e.code === 'IntlBackslash' ||
        e.code === 'Backquote' ||
        (e.code === 'Comma' && e.shiftKey));

    if (isToggleInspectorMouse) {
      e.preventDefault();
      toggleFocusAndPointerBetweenListAndInspector();
      return;
    }

    // Cmd + [ / Cmd + ] / Ctrl + Tab: Switch active browser pane focus (dual pane)
    const isSwitchPane =
      ((e.metaKey || e.ctrlKey) &&
        (e.key === '[' ||
          e.key === ']' ||
          (e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')))) ||
      (e.ctrlKey && e.key === 'Tab');

    if (isSwitchPane) {
      e.preventDefault();
      activePaneId.update((p) => (p === 'left' ? 'right' : 'left'));
      return;
    }

    // Option + Down / Option + Up / Option + PageDown / Option + PageUp: Scroll Inspector from keyboard (matching Swift Flashbrowse)
    if (e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        triggerInspectorScroll(160);
        return;
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        triggerInspectorScroll(-160);
        return;
      } else if (e.key === 'PageDown') {
        e.preventDefault();
        triggerInspectorScroll(450);
        return;
      } else if (e.key === 'PageUp') {
        e.preventDefault();
        triggerInspectorScroll(-450);
        return;
      }
    }

    // Cmd + 1 / Cmd + 2: Direct focus to left / right pane
    if ((e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey) {
      if (e.key === '1') {
        e.preventDefault();
        activePaneId.set('left');
        return;
      } else if (e.key === '2' && $isDualPane) {
        e.preventDefault();
        activePaneId.set('right');
        return;
      }
    }

    // Cmd+K / Ctrl+K: Command Palette
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      isPaletteOpen = !isPaletteOpen;
    }
    // Cmd+Shift+L / Ctrl+Shift+L: Toggle Inspector Lock
    else if ((e.metaKey || e.ctrlKey) && !e.altKey && e.shiftKey && e.key.toLowerCase() === 'l') {
      e.preventDefault();
      toggleInspectorLock();
    }
    // Cmd+J / Ctrl+J: Toggle Terminal
    else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'j') {
      if (e.altKey) {
        e.preventDefault();
        toggleTerminalDock();
      } else {
        e.preventDefault();
        toggleTerminal();
      }
    }
    // Cmd+Option+I: Cycle Inspector Layout Presets
    else if ((e.metaKey || e.ctrlKey) && e.altKey && e.key.toLowerCase() === 'i') {
      e.preventDefault();
      const presets: InspectorPreset[] = ['center', 'right', 'dual', 'none'];
      const nextIdx = (presets.indexOf($inspectorPreset) + 1) % presets.length;
      inspectorPreset.set(presets[nextIdx]);
    }
    // Cmd+Shift+D: One or two file browsers
    else if ((e.metaKey || e.ctrlKey) && !e.altKey && e.shiftKey && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      setDualPane(!$isDualPane);
    }
    // Cmd+Option+D: Toggle Dual Inspector
    else if ((e.metaKey || e.ctrlKey) && e.altKey && e.key.toLowerCase() === 'd') {
      e.preventDefault();
      inspectorPreset.update((cur) => (cur === 'dual' ? 'center' : 'dual'));
    }
    // Cmd+Option+S: Toggle Stash Shelf
    else if ((e.metaKey || e.ctrlKey) && e.altKey && e.key.toLowerCase() === 's') {
      e.preventDefault();
      toggleStash();
    }
  }

  /**
   * Switch between one and two file browsers.
   *
   * Going back to one pane moves focus to the left pane, so keyboard commands
   * cannot act on a browser that is no longer visible.
   */
  function setDualPane(enabled: boolean) {
    isDualPane.set(enabled);
    if (!enabled) activePaneId.set('left');
  }

  function unlockKidsMode() {
    if (pinInput === '1234') {
      setTheme('pro-dark');
      isPinModalOpen = false;
      pinInput = '';
      pinError = '';
    } else {
      pinError = 'Wrong PIN (Default is 1234)';
    }
  }
</script>

<svelte:window on:keydown={handleGlobalKeyDown} />

{#if isDetachedWindowMode}
  <!-- Standalone Detached Inspector Window View -->
  <DetachedInspectorView />
{:else if $layoutMode === 'clean'}
  <!-- Clean: one browser, the search field as the main surface, inspector to
       the right. Same palette as pro - this is a layout, not a theme. -->
  <div class="flex h-screen w-screen bg-[var(--bg-base)] text-[var(--text-primary)] overflow-hidden font-sans select-none">
    <div class="h-full shrink-0" style="width: 200px;">
      <CleanSidebar />
    </div>

    <div class="flex-1 flex flex-col min-w-0 h-full overflow-hidden">
      <CleanSearchBar
        bind:this={cleanSearchBar}
        rootPath={$leftPane.currentPath}
        rootLabel={cleanRootLabel}
        isAsking={$isAiGenerating}
        onResults={(matches) => (cleanMatches = matches)}
        onAsk={askAboutFiles}
      />

      {#if cleanQuestion}
        <CleanAnswer
          question={cleanQuestion}
          answer={cleanAnswer}
          isGenerating={$isAiGenerating}
          error={cleanAskError}
          seconds={cleanAskSeconds}
          modelName={$selectedModel}
          onClose={closeCleanAnswer}
        />
      {/if}

      <div class="flex-1 flex min-h-0 overflow-hidden">
        <div class="flex-1 flex flex-col min-w-0 h-full">
          {#if cleanMatches}
            <div class="flex items-center gap-2 px-4 py-1.5 border-b border-[var(--border)] bg-[var(--bg-surface)] text-xs shrink-0">
              <span class="text-[var(--text-secondary)]">Träffar i {cleanRootLabel}</span>
              <button
                class="ml-auto text-[11px] text-[var(--accent)] hover:underline"
                on:click={() => cleanSearchBar?.clear()}
              >
                Tillbaka till mappen
              </button>
            </div>
            <CleanSearchResults
              matches={cleanMatches}
              rootLabel={cleanRootLabel}
              onSelect={(m) => (leftPreviewItem = searchMatchToItem(m))}
              onOpen={(m) => openSearchMatch(m)}
            />
          {:else}
            <Breadcrumb paneId="left" />
            <FileTable paneId="left" onSelectPreview={(item) => (leftPreviewItem = item)} />
          {/if}
        </div>

        <ResizeHandle
          direction="vertical"
          onResize={(delta) => inspectorWidth.update((w) => Math.max(280, Math.min(950, w - delta)))}
          onReset={() => inspectorWidth.set(452)}
        />
        <div
          class="h-full shrink-0 flex flex-col bg-[var(--bg-base)] border-l border-[var(--border)]"
          style="width: {$inspectorWidth}px; min-width: 280px; max-width: 950px;"
        >
          <Inspector item={leftPreviewItem} titlePrefix="Fil" />
        </div>
      </div>
    </div>

    <StashShelf />
  </div>
{:else}
  <!-- Main Workstation Window -->
  <div class="flex h-screen w-screen bg-[var(--bg-base)] text-[var(--text-primary)] overflow-hidden font-sans select-none">
    <!-- 1. Left Sidebar -->
    <Sidebar />

    <!-- Sidebar Resize Divider -->
    <ResizeHandle
      direction="vertical"
      onResize={(delta) => sidebarWidth.update((w) => Math.max(140, Math.min(420, w + delta)))}
      onReset={() => sidebarWidth.set(208)}
    />

    <!-- 2. Main Workstation Area -->
    <div class="flex-1 flex flex-col min-w-0 h-full overflow-hidden">
      <!-- Top Bar for Kids Mode or Detached status -->
      {#if $isKidsMode}
        <div class="flex items-center justify-between px-4 py-2 bg-pink-100 border-b border-pink-200 text-pink-900 text-xs font-semibold">
          <div class="flex items-center gap-2">
            <span>🎈 Barn-läge Aktivt (Skrivskyddat & Sandlåda)</span>
          </div>
          <button
            class="flex items-center gap-1 px-3 py-1 rounded-full bg-pink-600 hover:bg-pink-700 text-white shadow-sm transition-transform active:scale-95"
            on:click={() => (isPinModalOpen = true)}
          >
            <Lock size={12} />
            <span>Lås upp föräldraläge</span>
          </button>
        </div>
      {:else if $isInspectorDetached}
        <div class="flex items-center justify-between px-4 py-1.5 bg-[#14171d] border-b border-[#262d3d] text-xs text-slate-300">
          <div class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>Inspektor är löskopplad till eget fönster</span>
          </div>
          <button
            class="flex items-center gap-1 px-2.5 py-0.5 rounded bg-[#e85422]/20 hover:bg-[#e85422] text-[#e85422] hover:text-white border border-[#e85422]/40 text-[11px] font-medium transition-colors"
            on:click={() => isInspectorDetached.set(false)}
          >
            <PanelRightClose size={11} />
            <span>Fäst tillbaka i huvudfönstret</span>
          </button>
        </div>
      {/if}
      <!-- Global Top Active / Hover Path & Filename Bar -->
      <div class="px-3 py-1.5 bg-[var(--bg-surface)] border-b border-[var(--border)] flex items-center justify-between gap-3 text-xs select-text overflow-hidden shrink-0">
        {#if $activeHoveredItem || ($activePaneId === 'left' ? leftPreviewItem : rightPreviewItem)}
          {@const active = $activeHoveredItem || ($activePaneId === 'left' ? leftPreviewItem : rightPreviewItem)}
          {#if active}
            <div class="flex items-center gap-2 min-w-0 flex-1 overflow-hidden">
              <div class="w-5 h-5 rounded flex items-center justify-center shrink-0 {active.is_dir ? 'bg-amber-500/20 text-amber-400' : 'bg-blue-500/20 text-blue-400'}">
                {#if active.is_dir}
                  <Folder size={12} />
                {:else if ['bam', 'cram', 'sam'].includes(active.extension.toLowerCase()) || active.name.endsWith('.bam')}
                  <Dna size={12} class="text-emerald-400" />
                {:else if ['vcf', 'bcf'].includes(active.extension.toLowerCase()) || active.name.endsWith('.vcf.gz')}
                  <Dna size={12} class="text-purple-400" />
                {:else if ['tsv', 'csv', 'tab', 'xlsx'].includes(active.extension.toLowerCase())}
                  <Table size={12} class="text-blue-400" />
                {:else}
                  <FileText size={12} />
                {/if}
              </div>

              <!-- Full filename in bold -->
              <span class="font-bold text-xs text-white select-text font-mono truncate hover:overflow-visible hover:whitespace-normal" title={active.name}>
                {active.name}
              </span>

              <!-- Type / Extension badge -->
              <span class="px-1.5 py-0.2 rounded bg-[#191d26] text-slate-300 text-[10px] font-mono border border-[#262d3d] shrink-0">
                {active.is_dir ? 'MAPP' : active.extension.toUpperCase() || 'FIL'}
              </span>

              <!-- Size & Modified -->
              <span class="text-[11px] text-slate-400 font-mono shrink-0">
                {active.is_dir ? '' : active.formatted_size} • {active.formatted_modified}
              </span>

              <!-- Full path in subtle font with copy button -->
              <span class="text-[10.5px] text-slate-500 font-mono truncate hidden md:inline select-text" title={active.path}>
                {active.path}
              </span>
            </div>
          {/if}
        {:else}
          <div class="text-[11px] text-slate-500 font-mono">
            Hovra eller markera en fil för att visa fullständigt namn och sökväg
          </div>
        {/if}

        <div class="flex items-center gap-2 shrink-0">
          {#if $activeHoveredItem || ($activePaneId === 'left' ? leftPreviewItem : rightPreviewItem)}
            {@const active = $activeHoveredItem || ($activePaneId === 'left' ? leftPreviewItem : rightPreviewItem)}
            {#if active}
              <button
                class="flex items-center gap-1 px-2 py-0.5 rounded bg-[#191d26] hover:bg-[#222836] border border-[#262d3d] text-[10.5px] text-slate-300 hover:text-white shrink-0 font-mono transition-colors"
                on:click={() => navigator.clipboard.writeText(active.path)}
                title="Kopiera fullständig sökväg"
              >
                <Copy size={10} />
                <span class="hidden sm:inline">Kopiera sökväg</span>
              </button>
            {/if}
          {/if}

          <!-- Terminal Toggle Button -->
          <button
            class="flex items-center gap-1.5 px-2.5 py-1 rounded border text-xs font-semibold transition-all {$isTerminalOpen ? 'bg-amber-500 text-black border-amber-400 shadow-md' : 'bg-[#141822] hover:bg-[#1f2535] text-slate-300 border-[#252d3d]'}"
            on:click={toggleTerminal}
            title="Öppna/Stäng Terminal (⌘J)"
          >
            <TerminalIcon size={13} class={$isTerminalOpen ? 'text-black' : 'text-amber-400'} />
            <span class="font-mono">Terminal</span>
            <kbd class="px-1 py-0.2 rounded text-[9px] font-mono {$isTerminalOpen ? 'bg-black/20 text-black' : 'bg-white/10 text-slate-400'}">⌘J</kbd>
          </button>

          <!-- Leave the workstation for the stripped-down layout -->
          <button
            class="flex items-center gap-1 px-2 py-1 rounded border border-[#252d3d] bg-[#141822] text-slate-400 hover:text-white hover:border-[var(--accent)] text-[10.5px] shrink-0 transition-colors"
            on:click={() => setLayoutMode('clean')}
            title="Clean-läge: en filbrowser, sökfältet i centrum, inga bioinformatikverktyg"
          >
            <Square size={11} />
            <span class="hidden md:inline">Clean</span>
          </button>

          <!-- Browser Count: one or two file panes -->
          <div class="flex items-center gap-0.5 bg-[#141822] p-0.5 rounded border border-[#252d3d] shrink-0 text-[10.5px]">
            <button
              class="flex items-center gap-1 px-2 py-0.5 rounded transition-colors {!$isDualPane ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
              on:click={() => setDualPane(false)}
              title="En filbrowser (⌘⇧D)"
            >
              <Square size={11} />
              <span class="hidden md:inline">1 panel</span>
            </button>
            <button
              class="flex items-center gap-1 px-2 py-0.5 rounded transition-colors {$isDualPane ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
              on:click={() => setDualPane(true)}
              title="Två filbrowsers sida vid sida (⌘⇧D)"
            >
              <Columns2 size={11} />
              <span class="hidden md:inline">2 paneler</span>
            </button>
          </div>

          <!-- Inspector Layout Preset Switcher -->
          <div class="flex items-center gap-0.5 bg-[#141822] p-0.5 rounded border border-[#252d3d] shrink-0 text-[10.5px]">
            <button
              class="flex items-center gap-1 px-2 py-0.5 rounded transition-colors {$inspectorPreset === 'center' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
              on:click={() => inspectorPreset.set('center')}
              title="Inspektör i mitten (Gemensam standard)"
            >
              <Columns3 size={11} />
              <span class="hidden md:inline">Center</span>
            </button>

            <button
              class="flex items-center gap-1 px-2 py-0.5 rounded transition-colors {$inspectorPreset === 'right' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
              on:click={() => inspectorPreset.set('right')}
              title="Inspektör till höger"
            >
              <LayoutTemplate size={11} />
              <span class="hidden md:inline">Höger</span>
            </button>

            <button
              class="flex items-center gap-1 px-2 py-0.5 rounded transition-colors {$inspectorPreset === 'dual' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
              on:click={() => inspectorPreset.set('dual')}
              title="Dubbla inspektörer (Vänster + Höger)"
            >
              <Columns2 size={11} />
              <span class="hidden md:inline">Dubbel</span>
            </button>

            <button
              class="flex items-center gap-1 px-2 py-0.5 rounded transition-colors {$inspectorPreset === 'none' ? 'bg-[var(--accent)] text-white font-bold' : 'text-slate-400 hover:text-white'}"
              on:click={() => inspectorPreset.set('none')}
              title="Dölj inspektör i huvudfönstret"
            >
              <EyeOff size={11} />
              <span class="hidden md:inline">Dold</span>
            </button>
          </div>
        </div>
      </div>

      <!-- Workstation Columns Container -->
      <div class="flex-1 flex min-h-0 overflow-hidden">
        {#if $activeIndexMeta}
          <!-- RECURSIVE HUB INDEX VIEW -->
          <div class="flex-1 flex min-w-0 min-h-0 h-full">
            <IndexBrowserView onSelectPreview={(item) => { leftPreviewItem = item; rightPreviewItem = item; }} />
          </div>

          <!-- Inspector alongside Index Hub -->
          {#if $inspectorPreset !== 'none'}
            <ResizeHandle
              direction="vertical"
              onResize={(delta) => inspectorWidth.update((w) => Math.max(280, Math.min(950, w - delta)))}
              onReset={() => inspectorWidth.set(540)}
            />
            <div
              class="h-full shrink-0 flex flex-col bg-[var(--bg-surface)]"
              style="width: {$inspectorWidth}px; min-width: 280px; max-width: 950px;"
            >
              <Inspector item={activeSharedItem} titlePrefix="Index Hub" />
            </div>
          {/if}

        {:else if $isDualPane}
          <!-- 1. CENTER SHARED INSPECTOR (DEFAULT) -->
          {#if $inspectorPreset === 'center'}
            <!-- Left Browser -->
            <div class="flex-1 flex flex-col min-w-[200px] h-full">
              <Breadcrumb paneId="left" />
              <FileTable paneId="left" onSelectPreview={(item) => (leftPreviewItem = item)} />
            </div>

            <!-- Left to Center Divider -->
            <ResizeHandle
              direction="vertical"
              onResize={(delta) => inspectorWidth.update((w) => Math.max(280, Math.min(950, w - delta)))}
              onReset={() => inspectorWidth.set(540)}
            />

            <!-- Central Shared Inspector -->
            <div
              class="h-full shrink-0 flex flex-col bg-[var(--bg-surface)]"
              style="width: {$inspectorWidth}px; min-width: 280px; max-width: 950px;"
            >
              <Inspector item={activeSharedItem} titlePrefix={activeSharedTitle} />
            </div>

            <!-- Center to Right Divider -->
            <ResizeHandle
              direction="vertical"
              onResize={(delta) => inspectorWidth.update((w) => Math.max(280, Math.min(950, w + delta)))}
              onReset={() => inspectorWidth.set(540)}
            />

            <!-- Right Browser -->
            <div class="flex-1 flex flex-col min-w-[200px] h-full">
              <Breadcrumb paneId="right" />
              <FileTable paneId="right" onSelectPreview={(item) => (rightPreviewItem = item)} />
            </div>

          <!-- 2. RIGHT-ALIGNED INSPECTOR -->
          {:else if $inspectorPreset === 'right'}
            <!-- Left Browser -->
            <div class="flex-1 flex flex-col min-w-[200px] h-full border-r border-[var(--border)]">
              <Breadcrumb paneId="left" />
              <FileTable paneId="left" onSelectPreview={(item) => (leftPreviewItem = item)} />
            </div>

            <!-- Right Browser -->
            <div class="flex-1 flex flex-col min-w-[200px] h-full">
              <Breadcrumb paneId="right" />
              <FileTable paneId="right" onSelectPreview={(item) => (rightPreviewItem = item)} />
            </div>

            <!-- Right Inspector Divider -->
            <ResizeHandle
              direction="vertical"
              onResize={(delta) => inspectorWidth.update((w) => Math.max(280, Math.min(950, w - delta)))}
              onReset={() => inspectorWidth.set(540)}
            />

            <!-- Right Inspector -->
            <div
              class="h-full shrink-0 flex flex-col bg-[var(--bg-surface)]"
              style="width: {$inspectorWidth}px; min-width: 280px; max-width: 950px;"
            >
              <Inspector item={activeSharedItem} titlePrefix={activeSharedTitle} />
            </div>

          <!-- 3. DUAL SEPARATE INSPECTORS -->
          {:else if $inspectorPreset === 'dual'}
            <!-- Left Workstation -->
            <div class="flex-1 flex min-w-0 h-full border-r border-[var(--border)]">
              <div class="flex-1 flex flex-col min-w-[180px] h-full">
                <Breadcrumb paneId="left" />
                <FileTable paneId="left" onSelectPreview={(item) => (leftPreviewItem = item)} />
              </div>
              <ResizeHandle
                direction="vertical"
                onResize={(delta) => dualInspectorWidth.update((w) => Math.max(220, Math.min(600, w - delta)))}
                onReset={() => dualInspectorWidth.set(360)}
              />
              <div
                class="h-full shrink-0 bg-[var(--bg-surface)] flex flex-col"
                style="width: {$dualInspectorWidth}px; min-width: 220px; max-width: 600px;"
              >
                <Inspector item={leftPreviewItem} titlePrefix="Vänster" />
              </div>
            </div>

            <!-- Right Workstation -->
            <div class="flex-1 flex min-w-0 h-full">
              <div class="flex-1 flex flex-col min-w-[180px] h-full">
                <Breadcrumb paneId="right" />
                <FileTable paneId="right" onSelectPreview={(item) => (rightPreviewItem = item)} />
              </div>
              <ResizeHandle
                direction="vertical"
                onResize={(delta) => dualInspectorWidth.update((w) => Math.max(220, Math.min(600, w - delta)))}
                onReset={() => dualInspectorWidth.set(360)}
              />
              <div
                class="h-full shrink-0 bg-[var(--bg-surface)] flex flex-col"
                style="width: {$dualInspectorWidth}px; min-width: 220px; max-width: 600px;"
              >
                <Inspector
                  item={rightPreviewItem}
                  titlePrefix={$rightPane.isSSH ? `Remote (${$rightPane.sshHost.split('.')[0]})` : 'Höger'}
                />
              </div>
            </div>

          <!-- 4. NO INSPECTOR (PURE DUAL BROWSERS) -->
          {:else if $inspectorPreset === 'none'}
            <div class="flex-1 flex flex-col min-w-[280px] h-full border-r border-[var(--border)]">
              <Breadcrumb paneId="left" />
              <FileTable paneId="left" onSelectPreview={(item) => (leftPreviewItem = item)} />
            </div>
            <div class="flex-1 flex flex-col min-w-[280px] h-full">
              <Breadcrumb paneId="right" />
              <FileTable paneId="right" onSelectPreview={(item) => (rightPreviewItem = item)} />
            </div>
          {/if}

        <!-- SINGLE PANE MODE -->
        {:else}
          <div class="flex-1 flex flex-col min-w-[320px] h-full">
            <Breadcrumb paneId="left" />
            <FileTable paneId="left" onSelectPreview={(item) => (leftPreviewItem = item)} />
          </div>

          {#if $inspectorPreset !== 'none'}
            <ResizeHandle
              direction="vertical"
              onResize={(delta) => inspectorWidth.update((w) => Math.max(280, Math.min(950, w - delta)))}
              onReset={() => inspectorWidth.set(540)}
            />
            <div
              class="h-full shrink-0 bg-[var(--bg-surface)] flex flex-col"
              style="width: {$inspectorWidth}px; min-width: 280px; max-width: 950px;"
            >
              <Inspector item={leftPreviewItem} titlePrefix="Lokal" />
            </div>
          {/if}
        {/if}

        <!-- Vertical Full-Height Side Terminal Column -->
        {#if $isTerminalOpen && $terminalDockPosition === 'side'}
          <ResizeHandle
            direction="vertical"
            onResize={(delta) => terminalWidth.update((w) => Math.max(240, Math.min(850, w - delta)))}
            onReset={() => terminalWidth.set(400)}
          />
          <Terminal />
        {/if}
      </div>

      <!-- Stash Shelf Drawer (Staging) -->
      <StashShelf />

      <!-- Horizontal Bottom Terminal Drawer -->
      {#if $isTerminalOpen && $terminalDockPosition === 'bottom'}
        <ResizeHandle
          direction="horizontal"
          onResize={(delta) => terminalHeight.update((h) => Math.max(120, Math.min(600, h - delta)))}
          onReset={() => terminalHeight.set(240)}
        />
        <Terminal />
      {/if}
    </div>
  </div>
{/if}

<!-- Transfer Progress / Status -->
{#if $transferProgress}
  {@const p = $transferProgress}
  <div class="fixed top-12 left-1/2 -translate-x-1/2 z-50 w-[min(30rem,90vw)] px-4 py-3 bg-cyan-950/95 text-cyan-100 border border-cyan-500/50 rounded-xl shadow-2xl backdrop-blur-md text-xs font-mono">
    <div class="flex items-center gap-2 mb-2">
      <div class="w-3.5 h-3.5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin shrink-0"></div>
      <span class="truncate flex-1" title={p.current_file}>{p.current_file || 'Förbereder…'}</span>
      <span class="shrink-0 tabular-nums text-cyan-300">{Math.round(p.percent)}%</span>
      <button
        class="shrink-0 px-2 py-0.5 rounded border border-cyan-500/50 hover:bg-cyan-500/20 text-cyan-200 transition-colors"
        on:click={cancelActiveTransfer}
        title="Avbryt överföringen (redan överförd data behålls)"
      >
        Avbryt
      </button>
    </div>

    <div class="h-1.5 rounded-full bg-cyan-500/20 overflow-hidden">
      <div class="h-full bg-cyan-400 transition-[width] duration-150" style="width: {Math.min(100, Math.max(0, p.percent))}%"></div>
    </div>

    <div class="flex items-center justify-between mt-1.5 text-[10px] text-cyan-300/80 tabular-nums">
      <span>{p.files_done}/{p.files_total} objekt</span>
      <span>{p.speed}{p.eta ? ` · ${p.eta} kvar` : ''}</span>
    </div>
  </div>
{:else if $isTransferring || $transferStatus}
  <div class="fixed top-12 left-1/2 -translate-x-1/2 z-50 px-4 py-2 bg-cyan-950/90 text-cyan-200 border border-cyan-500/50 rounded-full shadow-2xl backdrop-blur-md flex items-center gap-2.5 text-xs font-mono">
    {#if $isTransferring}
      <div class="w-3.5 h-3.5 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin"></div>
    {/if}
    <span>{$transferStatus || 'Överför...'}</span>
  </div>
{/if}

<!-- Save / Download Notification Toast -->
{#if $saveNotification}
  <div class="fixed top-12 left-1/2 -translate-x-1/2 z-50 px-4 py-2 {$saveNotification.success ? 'bg-emerald-950/95 text-emerald-200 border-emerald-500/50' : 'bg-red-950/95 text-red-200 border-red-500/50'} border rounded-full shadow-2xl backdrop-blur-md flex items-center gap-2.5 text-xs font-mono animate-bounce">
    {#if $saveNotification.success}
      <Check size={14} class="text-emerald-400" />
    {:else}
      <AlertCircle size={14} class="text-red-400" />
    {/if}
    <span>{$saveNotification.text}</span>
  </div>
{/if}

<!-- Command Palette Modal (Cmd+K) -->
<CommandPalette isOpen={isPaletteOpen} onClose={() => (isPaletteOpen = false)} />

<!-- Genomics Track & Viewer Hub -->
<GenomicsTrackHub />

<!-- Kids Mode PIN Unlock Modal -->
{#if isPinModalOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
    <div class="w-80 p-6 bg-[var(--bg-surface)] border border-[var(--border)] rounded-2xl shadow-2xl text-center space-y-4">
      <div class="w-12 h-12 rounded-full bg-pink-500/20 text-pink-500 flex items-center justify-center mx-auto">
        <Unlock size={24} />
      </div>
      <div>
        <h3 class="font-bold text-base text-[var(--text-primary)]">Lås upp föräldraläge</h3>
        <p class="text-xs text-[var(--text-secondary)] mt-1">Ange PIN-kod för att avsluta barn-läget (Standard: 1234)</p>
      </div>

      <input
        type="password"
        bind:value={pinInput}
        placeholder="PIN-kod"
        maxlength={6}
        class="w-full text-center text-xl tracking-widest font-mono p-2 rounded-lg bg-[var(--bg-panel)] border border-[var(--border)] text-[var(--text-primary)] focus:outline-none focus:border-[var(--accent)]"
        on:keydown={(e) => e.key === 'Enter' && unlockKidsMode()}
      />

      {#if pinError}
        <span class="text-xs text-red-400 block">{pinError}</span>
      {/if}

      <div class="flex gap-2">
        <button
          class="flex-1 py-2 rounded-lg border border-[var(--border)] hover:bg-[var(--bg-hover)] text-xs"
          on:click={() => { isPinModalOpen = false; pinInput = ''; pinError = ''; }}
        >
          Avbryt
        </button>
        <button
          class="flex-1 py-2 rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-xs font-semibold"
          on:click={unlockKidsMode}
        >
          Lås upp
        </button>
      </div>
    </div>
  </div>
{/if}
