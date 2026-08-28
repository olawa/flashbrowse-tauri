/**
 * Lightweight, fast Markdown to HTML parser for Flashbrowse previewer
 */
export function renderMarkdown(md: string): string {
  if (!md) return '';

  let html = md;

  // Escape basic HTML entities in text outside code
  const codeBlocks: string[] = [];
  html = html.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, (_match, lang, code) => {
    const escaped = escapeHtml(code);
    const index = codeBlocks.length;
    codeBlocks.push(
      `<div class="my-2 rounded-lg bg-[#0e1015] border border-[#252d3d] overflow-hidden"><div class="px-3 py-1 bg-[#161a24] text-slate-400 text-[10px] font-mono border-b border-[#252d3d] uppercase">${lang || 'code'}</div><pre class="p-3 text-[11px] font-mono text-emerald-300 overflow-x-auto m-0"><code>${escaped}</code></pre></div>`
    );
    return `<!--CODE_BLOCK_${index}-->`;
  });

  const inlineCodes: string[] = [];
  html = html.replace(/`([^`]+)`/g, (_match, code) => {
    const escaped = escapeHtml(code);
    const index = inlineCodes.length;
    inlineCodes.push(
      `<code class="px-1.5 py-0.5 rounded bg-[#1c2230] text-[#e85422] font-mono text-[11px] border border-[#2d374d]">${escaped}</code>`
    );
    return `<!--INLINE_CODE_${index}-->`;
  });

  // Headers (# H1, ## H2, ### H3, etc.)
  html = html.replace(/^######\s+(.*)$/gm, '<h6 class="text-xs font-bold text-slate-300 mt-3 mb-1">$1</h6>');
  html = html.replace(/^#####\s+(.*)$/gm, '<h5 class="text-xs font-bold text-slate-200 mt-3 mb-1">$1</h5>');
  html = html.replace(/^####\s+(.*)$/gm, '<h4 class="text-sm font-bold text-slate-200 mt-3 mb-1.5">$1</h4>');
  html = html.replace(/^###\s+(.*)$/gm, '<h3 class="text-sm font-bold text-amber-400 mt-4 mb-1.5 pb-1 border-b border-slate-800">$1</h3>');
  html = html.replace(/^##\s+(.*)$/gm, '<h2 class="text-base font-bold text-[var(--accent)] mt-4 mb-2 pb-1 border-b border-[#252d3d]">$1</h2>');
  html = html.replace(/^#\s+(.*)$/gm, '<h1 class="text-lg font-black text-white mt-4 mb-2 pb-1 border-b border-[#252d3d]">$1</h1>');

  // Blockquotes
  html = html.replace(/^>\s+(.*)$/gm, '<blockquote class="border-l-2 border-[var(--accent)] pl-3 my-2 text-slate-400 italic text-[11px] bg-[#141720]/50 py-1 rounded-r">$1</blockquote>');

  // Horizontal rules
  html = html.replace(/^---$/gm, '<hr class="border-[#252d3d] my-3" />');

  // Bold & Italic
  html = html.replace(/\*\*([^*]+)\*\*/g, '<strong class="font-bold text-white">$1</strong>');
  html = html.replace(/\*([^*]+)\*/g, '<em class="italic text-slate-300">$1</em>');

  // Checkboxes
  html = html.replace(/^- \[x\]\s+(.*)$/gm, '<div class="flex items-center gap-2 text-[11px] text-emerald-400 my-0.5"><span class="w-3.5 h-3.5 rounded bg-emerald-950 border border-emerald-700 flex items-center justify-center text-[9px] font-bold">✓</span><span>$1</span></div>');
  html = html.replace(/^- \[ \]\s+(.*)$/gm, '<div class="flex items-center gap-2 text-[11px] text-slate-400 my-0.5"><span class="w-3.5 h-3.5 rounded bg-slate-900 border border-slate-700"></span><span>$1</span></div>');

  // Bullet Lists
  html = html.replace(/^- (.*)$/gm, '<li class="ml-4 list-disc text-slate-300 my-0.5 text-[11px]">$1</li>');

  // Links
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer" class="text-[var(--accent)] hover:underline inline-flex items-center gap-0.5">$1 ↗</a>');

  // Paragraphs
  html = html
    .split('\n\n')
    .map((p) => {
      const trimmed = p.trim();
      if (!trimmed) return '';
      if (trimmed.startsWith('<h') || trimmed.startsWith('<div') || trimmed.startsWith('<blockquote') || trimmed.startsWith('<hr') || trimmed.startsWith('<li')) {
        return trimmed;
      }
      return `<p class="my-1.5 leading-relaxed text-slate-300 text-[11px]">${trimmed.replace(/\n/g, '<br />')}</p>`;
    })
    .join('\n');

  // Restore code blocks and inline code
  codeBlocks.forEach((block, i) => {
    html = html.replace(`<!--CODE_BLOCK_${i}-->`, block);
  });
  inlineCodes.forEach((code, i) => {
    html = html.replace(`<!--INLINE_CODE_${i}-->`, code);
  });

  return html;
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
