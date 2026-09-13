/**
 * GitHub-flavored Markdown -> sanitized HTML for the Flashbrowse previewer.
 *
 * Uses `marked` (GFM: tables, task lists, autolinks, strikethrough) and
 * sanitizes the result with DOMPurify. Styling lives in `.fb-md` in app.css
 * so tables, badges and images render the way GitHub shows them.
 */
import { marked } from 'marked';
import DOMPurify from 'dompurify';

marked.setOptions({
  gfm: true,
  breaks: false,
});

const ALLOWED_PROTOCOLS = /^(https?:|mailto:|tel:|data:image\/(png|jpe?g|gif|webp|svg\+xml);)/i;

function isSafeUrl(url: string): boolean {
  const trimmed = url.trim();
  // Relative links inside a repo README are safe (they resolve to nothing clickable).
  if (!/^[a-z][a-z0-9+.-]*:/i.test(trimmed)) return true;
  return ALLOWED_PROTOCOLS.test(trimmed);
}

let hooksInstalled = false;
function installHooks() {
  if (hooksInstalled || typeof window === 'undefined') return;
  hooksInstalled = true;
  DOMPurify.addHook('afterSanitizeAttributes', (node: any) => {
    if (node.tagName === 'A') {
      const href = node.getAttribute('href') || '';
      if (!isSafeUrl(href)) {
        node.removeAttribute('href');
      } else {
        node.setAttribute('target', '_blank');
        node.setAttribute('rel', 'noopener noreferrer');
      }
    }
    if (node.tagName === 'IMG') {
      const src = node.getAttribute('src') || '';
      if (!isSafeUrl(src)) node.remove();
      else node.setAttribute('loading', 'lazy');
    }
  });
}

// Svelte re-evaluates `{@html renderMarkdown(...)}` on every reactive update of the
// surrounding component. Parsing a 10 kB README is only ~1.5 ms, but sanitizing builds
// a DOM tree each time, so cache the last few documents: large files (notebooks, logs,
// generated reports) otherwise re-parse on every hover and scroll.
const CACHE_LIMIT = 8;
const cache = new Map<string, string>();

function cached(md: string, render: () => string): string {
  const hit = cache.get(md);
  if (hit !== undefined) {
    // refresh LRU position
    cache.delete(md);
    cache.set(md, hit);
    return hit;
  }
  const html = render();
  cache.set(md, html);
  if (cache.size > CACHE_LIMIT) cache.delete(cache.keys().next().value as string);
  return html;
}

export function renderMarkdown(md: string): string {
  if (!md) return '';
  return cached(md, () => renderMarkdownUncached(md));
}

function renderMarkdownUncached(md: string): string {

  let html: string;
  try {
    html = marked.parse(md, { async: false }) as string;
  } catch {
    return `<div class="fb-md"><pre>${escapeHtml(md)}</pre></div>`;
  }

  if (typeof window === 'undefined') {
    // SSR / non-DOM contexts: no sanitizer available, fall back to escaped source.
    return `<div class="fb-md"><pre>${escapeHtml(md)}</pre></div>`;
  }

  installHooks();
  const clean = DOMPurify.sanitize(html, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: ['style', 'form', 'input', 'button', 'iframe', 'script'],
    FORBID_ATTR: ['style', 'srcset'],
    ADD_ATTR: ['target', 'rel', 'align', 'loading'],
  });

  return `<div class="fb-md">${clean}</div>`;
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
