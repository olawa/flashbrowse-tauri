/** Keep remote scrolling working across the sandbox's opaque origin. */
export function htmlPreviewDocument(html: string): string {
  const bridge = `<script>
    window.addEventListener('message', function (event) {
      if (event.source !== window.parent) return;
      var data = event.data;
      if (data && data.type === 'flashbrowse:scroll' && Number.isFinite(data.deltaY)) {
        window.scrollBy({ top: data.deltaY, left: 0, behavior: 'auto' });
      }
    });
  </script>`;
  // Preserve standards mode when the report already has a doctype.
  const doctype = html.match(/^\s*<!doctype[^>]*>/i)?.[0] ?? '';
  return doctype + bridge + html.slice(doctype.length);
}
