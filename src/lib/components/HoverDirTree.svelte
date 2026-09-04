<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getSubdirsTree } from '../invoke';
  import type { SubdirNode } from '../types';
  import { Folder, ChevronRight, MoreHorizontal } from 'lucide-svelte';

  export let path: string;
  export let anchorX: number;
  export let anchorY: number;
  export let onNavigate: (p: string) => void;
  export let onClose: () => void;

  let nodes: SubdirNode[] = [];
  let loading = true;
  let tooltipEl: HTMLElement;

  let expandedL1Path: string | null = null;
  let expandedL1Timer: any = null;
  let expandedL2Path: string | null = null;
  let expandedL2Timer: any = null;

  onMount(async () => {
    try {
      nodes = await getSubdirsTree(path, 3, 8);
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

  function handleL1MouseEnter(node: SubdirNode) {
    clearTimeout(expandedL2Timer);
    expandedL2Timer = setTimeout(() => {
      expandedL2Path = node.path;
    }, 150);
  }

  function navigate(p: string, e: MouseEvent) {
    e.stopPropagation();
    onNavigate(p);
  }

  export let cancelClose: () => void = () => {};

  let tooltipStyle = '';
  $: {
    const GAP = 4;
    const W = 224;
    const fromRight = typeof window !== 'undefined' ? window.innerWidth - anchorX : 999;
    if (fromRight < W + GAP * 2) {
      tooltipStyle = `right: ${typeof window !== 'undefined' ? window.innerWidth - anchorX + GAP : 0}px; top: ${anchorY}px;`;
    } else {
      tooltipStyle = `left: ${anchorX}px; top: ${anchorY}px;`;
    }
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
  class="fixed z-[9999] w-56 rounded-lg border border-[var(--border)] bg-[var(--bg-surface)] shadow-2xl shadow-black/40 backdrop-blur-sm text-xs select-none overflow-visible"
  style={tooltipStyle}
  on:mouseenter={handleMouseEnter}
  on:mouseleave={handleMouseLeave}
  role="tree"
>
  <div class="flex items-center gap-1.5 px-2.5 py-1.5 border-b border-[var(--border)] text-[var(--text-muted)] text-[10px] font-semibold uppercase tracking-wide rounded-t-lg overflow-hidden">
    <Folder size={10} class="text-amber-400 shrink-0" />
    <span class="truncate">{path.split('/').pop() || path}</span>
  </div>

  {#if loading}
    <div class="px-3 py-3 text-[var(--text-muted)] flex items-center gap-2">
      <div class="w-3 h-3 border border-[var(--text-muted)] border-t-transparent rounded-full animate-spin"></div>
      <span>Laddar...</span>
    </div>
  {:else if nodes.length === 0}
    <div class="px-3 py-2.5 text-[var(--text-muted)] italic text-center">Inga underkataloger</div>
  {:else}
    <div class="max-h-72 overflow-y-auto overflow-x-visible py-0.5">
      {#each nodes as node (node.path)}
        {@const isExpanded = expandedL1Path === node.path}
        {@const hasChildren = node.children.length > 0}

        <div
          class="group relative"
          on:mouseenter={() => handleL0MouseEnter(node)}
          role="treeitem"
          aria-expanded={isExpanded}
          tabindex="-1"
        >
          <button
            class="w-full flex items-center gap-1.5 px-2.5 py-[5px] hover:bg-[var(--bg-hover)] transition-colors text-[var(--text-primary)] text-left {isExpanded ? 'bg-[var(--bg-hover)]' : ''}"
            on:click={(e) => navigate(node.path, e)}
            title={node.path}
          >
            <Folder size={12} class="shrink-0 text-amber-400" />
            <span class="truncate flex-1 font-medium">{node.name}</span>
            {#if hasChildren}
              <ChevronRight size={10} class="shrink-0 opacity-40 {isExpanded ? 'opacity-90 text-[var(--accent)] rotate-90' : ''} transition-transform" />
            {/if}
          </button>

          {#if isExpanded && hasChildren}
            <div
              class="absolute left-full top-0 w-52 rounded-lg border border-[var(--border)] bg-[var(--bg-surface)] shadow-2xl shadow-black/40 z-10 overflow-visible"
              style="margin-left: 2px;"
            >
              <div class="max-h-60 overflow-y-auto overflow-x-visible py-0.5">
                {#each node.children as child (child.path)}
                  {@const isL2Expanded = expandedL2Path === child.path}
                  {@const hasL2Children = child.children.length > 0}

                  <div
                    class="relative"
                    on:mouseenter={() => handleL1MouseEnter(child)}
                    role="treeitem"
                    aria-expanded={isL2Expanded}
                    tabindex="-1"
                  >
                    <button
                      class="w-full flex items-center gap-1.5 px-2.5 py-[5px] hover:bg-[var(--bg-hover)] transition-colors text-[var(--text-primary)] text-left {isL2Expanded ? 'bg-[var(--bg-hover)]' : ''}"
                      on:click={(e) => navigate(child.path, e)}
                      title={child.path}
                    >
                      <Folder size={12} class="shrink-0 text-amber-400/70" />
                      <span class="truncate flex-1">{child.name}</span>
                      {#if hasL2Children}
                        <ChevronRight size={10} class="shrink-0 opacity-40 {isL2Expanded ? 'opacity-90 text-[var(--accent)] rotate-90' : ''} transition-transform" />
                      {/if}
                    </button>

                    {#if isL2Expanded && hasL2Children}
                      <div
                        class="absolute left-full top-0 w-48 rounded-lg border border-[var(--border)] bg-[var(--bg-surface)] shadow-2xl shadow-black/40 z-20 overflow-hidden"
                        style="margin-left: 2px;"
                      >
                        <div class="max-h-56 overflow-y-auto py-0.5">
                          {#each child.children as grandchild (grandchild.path)}
                            <button
                              class="w-full flex items-center gap-1.5 px-2.5 py-[5px] hover:bg-[var(--bg-hover)] transition-colors text-[var(--text-primary)] text-left"
                              on:click={(e) => navigate(grandchild.path, e)}
                              title={grandchild.path}
                            >
                              <Folder size={11} class="shrink-0 text-amber-400/50" />
                              <span class="truncate flex-1 text-[11px]">{grandchild.name}</span>
                            </button>
                          {/each}
                          {#if child.has_more}
                            <div class="flex items-center gap-1.5 px-2.5 py-1 text-[var(--text-muted)] italic">
                              <MoreHorizontal size={11} /> fler...
                            </div>
                          {/if}
                        </div>
                      </div>
                    {/if}
                  </div>
                {/each}
                {#if node.has_more}
                  <div class="flex items-center gap-1.5 px-2.5 py-1 text-[var(--text-muted)] italic">
                    <MoreHorizontal size={11} /> fler...
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      {/each}

      {#if nodes[nodes.length - 1]?.has_more}
        <div class="flex items-center gap-1.5 px-2.5 py-1 text-[var(--text-muted)] italic border-t border-[var(--border)] mt-0.5">
          <MoreHorizontal size={11} /> fler kataloger...
        </div>
      {/if}
    </div>
  {/if}
</div>
