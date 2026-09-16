<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getSubdirsTree } from '../invoke';
  import type { SubdirNode } from '../types';
  import { Folder, ChevronRight, ChevronDown, MoreHorizontal } from 'lucide-svelte';

  export let path: string;
  export let anchorX: number;
  export let anchorY: number;
  export let onNavigate: (p: string) => void;
  export let onClose: () => void;
  export let cancelClose: () => void = () => {};

  let nodes: SubdirNode[] = [];
  let loading = true;
  let tooltipEl: HTMLElement;

  let expandedL1Path: string | null = null;
  let expandedL1Timer: any = null;
  let expandedL2Path: string | null = null;
  let expandedL2Timer: any = null;

  onMount(async () => {
    try {
      nodes = await getSubdirsTree(path, 3, 10);
    } catch {
      nodes = [];
    } finally {
      loading = false;
    }
  });

  onDestroy(() => {
    clearTimeout(expandedL1Timer);
    clearTimeout(expandedL2Timer);
  });

  function handleL0MouseEnter(node: SubdirNode) {
    clearTimeout(expandedL1Timer);
    expandedL1Timer = setTimeout(() => {
      expandedL1Path = node.path;
      expandedL2Path = null;
    }, 150);
  }

  function handleL1MouseEnter(child: SubdirNode) {
    clearTimeout(expandedL2Timer);
    expandedL2Timer = setTimeout(() => {
      expandedL2Path = child.path;
    }, 150);
  }

  function navigate(p: string, e: MouseEvent) {
    e.stopPropagation();
    onNavigate(p);
  }

  let tooltipStyle = '';
  $: {
    const W = 256;
    const safeX = typeof window !== 'undefined' ? Math.max(10, Math.min(window.innerWidth - W - 15, anchorX)) : anchorX;
    const safeY = typeof window !== 'undefined' ? Math.max(10, Math.min(window.innerHeight - 300, anchorY)) : anchorY;
    tooltipStyle = `left: ${safeX}px; top: ${safeY}px;`;
  }

  function handleMouseEnter() {
    cancelClose();
  }

  function handleMouseLeave(e: MouseEvent) {
    const related = e.relatedTarget as HTMLElement | null;
    if (related && tooltipEl?.contains(related)) return;
    onClose();
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  bind:this={tooltipEl}
  class="fixed z-[9999] w-64 max-h-96 rounded-xl border border-[var(--border)] bg-[var(--bg-surface)]/95 shadow-2xl shadow-black/60 backdrop-blur-md text-xs select-none flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-100"
  style={tooltipStyle}
  on:mouseenter={handleMouseEnter}
  on:mouseleave={handleMouseLeave}
  role="tree"
>
  <!-- Header with folder title -->
  <div class="flex items-center gap-2 px-3 py-2 border-b border-[var(--border)] bg-[var(--bg-panel)]/80 text-[var(--text-secondary)] text-[11px] font-semibold shrink-0">
    <Folder size={12} class="text-[var(--accent)] shrink-0" />
    <span class="truncate font-mono">{path.split('/').pop() || path}</span>
    <span class="ml-auto text-[9px] text-[var(--text-muted)] font-normal uppercase tracking-wider">Träd</span>
  </div>

  {#if loading}
    <div class="px-4 py-4 text-[var(--text-muted)] flex items-center gap-2 text-xs">
      <div class="w-3.5 h-3.5 border-2 border-[var(--accent)] border-t-transparent rounded-full animate-spin"></div>
      <span>Läser in undermappar...</span>
    </div>
  {:else if nodes.length === 0}
    <div class="px-4 py-3 text-[var(--text-muted)] italic text-center text-xs">Inga underkataloger</div>
  {:else}
    <!-- Single vertical scroll view with progressive inline expansion -->
    <div class="flex-1 overflow-y-auto py-1 px-1 divide-y divide-[var(--border)]/20 font-sans">
      {#each nodes as node (node.path)}
        {@const isExpanded = expandedL1Path === node.path}
        {@const hasChildren = node.children && node.children.length > 0}

        <div
          class="flex flex-col rounded overflow-hidden"
          on:mouseenter={() => handleL0MouseEnter(node)}
          role="treeitem"
          aria-expanded={isExpanded}
          tabindex="-1"
        >
          <!-- Level 0 Row -->
          <button
            class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded hover:bg-[var(--bg-hover)] transition-colors text-[var(--text-primary)] text-left group cursor-pointer {isExpanded ? 'bg-[var(--bg-hover)] text-white' : ''}"
            on:click={(e) => navigate(node.path, e)}
            title="{node.path} (Klicka för att hoppa hit)"
          >
            <Folder size={13} class="shrink-0 text-amber-400 group-hover:text-amber-300" />
            <span class="truncate flex-1 font-medium text-xs">{node.name}</span>
            {#if hasChildren}
              {#if isExpanded}
                <ChevronDown size={12} class="shrink-0 text-[var(--accent)]" />
              {:else}
                <ChevronRight size={12} class="shrink-0 opacity-40 group-hover:opacity-80" />
              {/if}
            {/if}
          </button>

          <!-- Level 1 Children: Inline Vertical Accordion with Left Guide Line -->
          {#if isExpanded && hasChildren}
            <div class="flex flex-col ml-3.5 my-0.5 pl-2 border-l border-[var(--border)]/70 bg-black/10 rounded-r">
              {#each node.children as child (child.path)}
                {@const isL2Expanded = expandedL2Path === child.path}
                {@const hasL2Children = child.children && child.children.length > 0}

                <div
                  class="flex flex-col"
                  on:mouseenter={() => handleL1MouseEnter(child)}
                  role="treeitem"
                  aria-expanded={isL2Expanded}
                  tabindex="-1"
                >
                  <!-- Level 1 Row -->
                  <button
                    class="w-full flex items-center gap-1.5 px-2 py-1 rounded hover:bg-[var(--bg-hover)] transition-colors text-[var(--text-primary)] text-left group cursor-pointer {isL2Expanded ? 'bg-[var(--bg-hover)] text-white' : ''}"
                    on:click={(e) => navigate(child.path, e)}
                    title="{child.path} (Klicka för att hoppa hit)"
                  >
                    <Folder size={11} class="shrink-0 text-amber-400/80 group-hover:text-amber-300" />
                    <span class="truncate flex-1 text-[11.5px]">{child.name}</span>
                    {#if hasL2Children}
                      {#if isL2Expanded}
                        <ChevronDown size={11} class="shrink-0 text-[var(--accent)]" />
                      {:else}
                        <ChevronRight size={11} class="shrink-0 opacity-40 group-hover:opacity-80" />
                      {/if}
                    {/if}
                  </button>

                  <!-- Level 2 Children: Deeper Inline Vertical Accordion -->
                  {#if isL2Expanded && hasL2Children}
                    <div class="flex flex-col ml-3 my-0.5 pl-2 border-l border-[var(--border)]/60 bg-black/15 rounded-r">
                      {#each child.children as grandchild (grandchild.path)}
                        <button
                          class="w-full flex items-center gap-1.5 px-2 py-1 rounded hover:bg-[var(--bg-hover)] transition-colors text-[var(--text-secondary)] hover:text-white text-left text-[11px] cursor-pointer"
                          on:click={(e) => navigate(grandchild.path, e)}
                          title="{grandchild.path} (Klicka för att hoppa hit)"
                        >
                          <Folder size={10} class="shrink-0 text-amber-400/60" />
                          <span class="truncate flex-1">{grandchild.name}</span>
                        </button>
                      {/each}

                      {#if child.has_more}
                        <div class="flex items-center gap-1.5 px-2 py-0.5 text-[var(--text-muted)] italic text-[10px]">
                          <MoreHorizontal size={10} /> fler...
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}

              {#if node.has_more}
                <div class="flex items-center gap-1.5 px-2 py-0.5 text-[var(--text-muted)] italic text-[10px]">
                  <MoreHorizontal size={10} /> fler...
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}

      {#if nodes[nodes.length - 1]?.has_more}
        <div class="flex items-center gap-1.5 px-3 py-1.5 text-[var(--text-muted)] italic text-[10px] border-t border-[var(--border)] mt-1">
          <MoreHorizontal size={11} /> Fler mappar i rotkatalogen...
        </div>
      {/if}
    </div>
  {/if}
</div>
