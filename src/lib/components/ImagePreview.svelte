<script lang="ts">
  import { onDestroy } from 'svelte';
  import { ZoomIn, ZoomOut, Maximize2 } from 'lucide-svelte';

  export let src: string;
  export let alt = '';

  // The preview fills the panel and zooms under the pointer, the way an image
  // viewer does - a photo squeezed into a fixed 256px box wastes most of an
  // inspector that is half the window wide.
  const MIN_SCALE = 1;
  const MAX_SCALE = 12;

  let scale = 1;
  let offsetX = 0;
  let offsetY = 0;
  let container: HTMLElement;

  $: if (src) reset();

  function reset() {
    scale = 1;
    offsetX = 0;
    offsetY = 0;
  }

  /** Zoom about a point, so what is under the pointer stays under it. */
  function zoomAt(factor: number, clientX: number, clientY: number) {
    const rect = container?.getBoundingClientRect();
    if (!rect) return;

    const next = Math.min(Math.max(scale * factor, MIN_SCALE), MAX_SCALE);
    if (next === scale) return;

    // Pointer position relative to the untransformed centre.
    const px = clientX - rect.left - rect.width / 2;
    const py = clientY - rect.top - rect.height / 2;

    offsetX = px - ((px - offsetX) * next) / scale;
    offsetY = py - ((py - offsetY) * next) / scale;
    scale = next;

    if (scale === MIN_SCALE) {
      offsetX = 0;
      offsetY = 0;
    }
  }

  function handleWheel(e: WheelEvent) {
    // macOS sends a pinch as a wheel event with ctrlKey set.
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      zoomAt(Math.exp(-e.deltaY * 0.01), e.clientX, e.clientY);
      return;
    }

    // Two fingers pan once there is something to pan around.
    if (scale > MIN_SCALE) {
      e.preventDefault();
      offsetX -= e.deltaX;
      offsetY -= e.deltaY;
    }
  }

  let dragging = false;
  let dragStart = { x: 0, y: 0, offsetX: 0, offsetY: 0 };

  function startDrag(e: MouseEvent) {
    if (scale === MIN_SCALE) return;
    dragging = true;
    dragStart = { x: e.clientX, y: e.clientY, offsetX, offsetY };
    window.addEventListener('mousemove', onDrag);
    window.addEventListener('mouseup', endDrag);
  }

  function onDrag(e: MouseEvent) {
    if (!dragging) return;
    offsetX = dragStart.offsetX + (e.clientX - dragStart.x);
    offsetY = dragStart.offsetY + (e.clientY - dragStart.y);
  }

  function endDrag() {
    dragging = false;
    window.removeEventListener('mousemove', onDrag);
    window.removeEventListener('mouseup', endDrag);
  }

  onDestroy(endDrag);
</script>

<div
  bind:this={container}
  class="relative flex-1 min-h-[320px] flex items-center justify-center overflow-hidden bg-black/50 group"
  on:wheel={handleWheel}
  on:mousedown={startDrag}
  on:dblclick={reset}
  role="img"
  aria-label={alt}
  title="Nyp för att zooma · dra för att flytta · dubbelklick återställer"
>
  <img
    {src}
    {alt}
    draggable="false"
    class="max-h-full max-w-full object-contain select-none {dragging ? '' : 'transition-transform duration-75'}"
    style="transform: translate({offsetX}px, {offsetY}px) scale({scale}); cursor: {scale > 1
      ? dragging
        ? 'grabbing'
        : 'grab'
      : 'zoom-in'};"
  />

  <div
    class="absolute bottom-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
  >
    <button
      class="p-1.5 rounded bg-black/70 border border-white/15 text-white hover:bg-black/90"
      on:click|stopPropagation={() => zoomAt(1 / 1.4, 0, 0)}
      title="Zooma ut"
    >
      <ZoomOut size={13} />
    </button>
    <button
      class="p-1.5 rounded bg-black/70 border border-white/15 text-white hover:bg-black/90"
      on:click|stopPropagation={() => zoomAt(1.4, 0, 0)}
      title="Zooma in"
    >
      <ZoomIn size={13} />
    </button>
    <button
      class="p-1.5 rounded bg-black/70 border border-white/15 text-white hover:bg-black/90"
      on:click|stopPropagation={reset}
      title="Passa in i rutan (dubbelklick)"
    >
      <Maximize2 size={13} />
    </button>
    {#if scale > 1}
      <span class="px-1.5 py-1 rounded bg-black/70 border border-white/15 text-white text-[10px] font-mono tabular-nums">
        {Math.round(scale * 100)}%
      </span>
    {/if}
  </div>
</div>
