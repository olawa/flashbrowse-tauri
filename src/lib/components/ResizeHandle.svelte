<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let direction: 'vertical' | 'horizontal' = 'vertical';
  export let onResize: (delta: number) => void;
  export let onReset: (() => void) | undefined = undefined;
  export let className = '';

  const dispatch = createEventDispatcher();
  let isDragging = false;
  let startPos = 0;

  function handleMouseDown(e: MouseEvent) {
    if (e.button !== 0) return; // Only left click
    e.preventDefault();
    e.stopPropagation();

    isDragging = true;
    startPos = direction === 'vertical' ? e.clientX : e.clientY;

    const originalCursor = document.body.style.cursor;
    const originalUserSelect = document.body.style.userSelect;
    document.body.style.cursor = direction === 'vertical' ? 'col-resize' : 'row-resize';
    document.body.style.userSelect = 'none';

    function handleMouseMove(moveEvent: MouseEvent) {
      const currentPos = direction === 'vertical' ? moveEvent.clientX : moveEvent.clientY;
      const delta = currentPos - startPos;
      startPos = currentPos;
      onResize(delta);
    }

    function handleMouseUp() {
      isDragging = false;
      document.body.style.cursor = originalCursor;
      document.body.style.userSelect = originalUserSelect;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      dispatch('resizeEnd');
    }

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }

  function handleDblClick(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (onReset) {
      onReset();
    }
  }
</script>

<div
  class="relative shrink-0 select-none z-20 group transition-colors {direction === 'vertical'
    ? 'w-2 -mx-1 cursor-col-resize hover:bg-[var(--accent)]/30'
    : 'h-2 -my-1 cursor-row-resize hover:bg-[var(--accent)]/30'} {isDragging
    ? 'bg-[var(--accent)]/60'
    : ''} {className}"
  on:mousedown={handleMouseDown}
  on:dblclick={handleDblClick}
  role="separator"
  aria-orientation={direction}
  tabindex="-1"
  title="Dra för att justera storlek (Dubbelklicka för standard)"
>
  <!-- Thin center indicator line -->
  <div
    class="absolute inset-0 m-auto pointer-events-none {direction === 'vertical'
      ? 'w-px h-full bg-[var(--border)] group-hover:bg-[var(--accent)]'
      : 'h-px w-full bg-[var(--border)] group-hover:bg-[var(--accent)]'} {isDragging
      ? 'bg-[var(--accent)]'
      : ''}"
  ></div>
</div>
