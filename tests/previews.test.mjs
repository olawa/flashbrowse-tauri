import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import { JSDOM } from 'jsdom';
import { latestRequest } from '../src/lib/latestRequest.ts';
import { htmlPreviewDocument } from '../src/lib/htmlPreview.ts';

const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};

// Execute the actual component loading functions with controllable IPC responses.
function previewHarness(component, functionName) {
  const source = readFileSync(new URL(`../src/lib/components/${component}.svelte`, import.meta.url), 'utf8');
  const script = source.match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
  const ast = ts.createSourceFile('component.ts', script, ts.ScriptTarget.Latest, true);
  const fn = ast.statements.find((node) => ts.isFunctionDeclaration(node) && node.name.text === functionName);
  assert.ok(fn, 'component loading function exists');
  const requests = new Map();
  const request = (path) => {
    const pending = deferred();
    requests.set(path, pending);
    return pending.promise;
  };
  const gate = latestRequest();
  const context = vm.createContext({ getPreview: request, sshGetPreview: (_host, path) => request(path), previewRequests: gate });
  const code = `let preview = null, dirSummary = null, isLoading = false;
    ${fn.getText(ast)}
    globalThis.load = ${functionName};
    globalThis.state = () => ({ preview, isLoading });`;
  vm.runInContext(ts.transpileModule(code, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText, context);
  const load = (path) => component === 'Inspector'
    ? context.load({ path, is_dir: false }, false, '') : context.load(path);
  return { load, requests, state: context.state, gate };
}

for (const [component, fn] of [['Inspector', 'loadItemPreview'], ['DetachedInspectorView', 'loadPreview']]) {
  test(`${component}: old results and errors cannot overwrite the newest preview or loading state`, async () => {
    for (const failure of [false, true]) {
      const view = previewHarness(component, fn);
      const first = view.load('/A');
      const second = view.load('/B');
      if (failure) view.requests.get('/A').reject(new Error('stale'));
      else view.requests.get('/A').resolve({ content: 'old' });
      await first;
      assert.equal(view.state().isLoading, true);
      assert.equal(view.state().preview, null);
      view.requests.get('/B').resolve({ content: 'new' });
      await second;
      assert.equal(view.state().preview.content, 'new');
      assert.equal(view.state().isLoading, false);
    }
    const view = previewHarness(component, fn);
    const first = view.load('/A');
    const second = view.load('/B');
    view.requests.get('/B').resolve({ content: 'new' });
    await second;
    view.requests.get('/A').resolve({ content: 'old' });
    await first;
    assert.equal(view.state().preview.content, 'new');
  });

  test(`${component}: invalidating on clear or unmount discards pending responses`, async () => {
    const view = previewHarness(component, fn);
    const request = view.load('/A');
    view.gate.invalidate();
    view.requests.get('/A').resolve({ content: 'old' });
    await request;
    assert.equal(view.state().preview, null);
  });

  test(`${component}: HTML runs without app-origin or popup privileges`, () => {
    const source = readFileSync(new URL(`../src/lib/components/${component}.svelte`, import.meta.url), 'utf8');
    const iframe = source.match(/<iframe\b[^>]*srcdoc=\{[^}]*preview.html_content[^}]*\}[^>]*>/)[0];
    assert.match(iframe, /sandbox="allow-scripts"/);
    assert.doesNotMatch(iframe, /allow-same-origin|allow-popups|allow-top-navigation/);
  });
}

test('isolated HTML accepts only finite scroll messages from its parent', () => {
  const html = '<!DOCTYPE html><html><body>Report</body></html>';
  const document = htmlPreviewDocument(html);
  assert.ok(document.startsWith('<!DOCTYPE html>'));
  assert.ok(document.endsWith('<html><body>Report</body></html>'));
  const calls = [];
  let handler;
  const parent = {};
  const window = { parent, addEventListener: (_name, fn) => { handler = fn; }, scrollBy: (args) => calls.push(args.top) };
  vm.runInNewContext(document.match(/<script>([\s\S]*?)<\/script>/)[1], { window });
  handler({ source: {}, data: { type: 'flashbrowse:scroll', deltaY: 20 } });
  handler({ source: parent, data: { type: 'flashbrowse:scroll', deltaY: Infinity } });
  handler({ source: parent, data: { type: 'execute', deltaY: 20 } });
  handler({ source: parent, data: null });
  assert.deepEqual(calls, []);
  handler({ source: parent, data: { type: 'flashbrowse:scroll', deltaY: 20 } });
  assert.deepEqual(calls, [20]);
});

test('notebook HTML strips executable content while retaining output tables', async () => {
  const dom = new JSDOM('');
  globalThis.window = dom.window;
  try {
    const { sanitizePreviewHtml } = await import('../src/lib/markdown.ts');
    const html = sanitizePreviewHtml(`<table><tbody><tr><td>42</td></tr></tbody></table>
      <script>window.__TAURI_INTERNALS__.invoke('trash_items')</script>
      <img src="x" onerror="alert(1)"><a href="javascript:alert(1)">bad</a>
      <iframe srcdoc="<script>alert(1)</script>"></iframe>
      <svg onload="alert(1)"></svg><form><button>Send</button></form>`);
    const document = new JSDOM(html).window.document;
    assert.equal(document.querySelector('td').textContent, '42');
    assert.equal(document.querySelector('script,iframe,svg,form,button,[onerror],[onload]'), null);
    assert.equal(document.querySelector('a').hasAttribute('href'), false);
    const component = readFileSync(new URL('../src/lib/components/NotebookViewer.svelte', import.meta.url), 'utf8');
    assert.match(component, /\{@html sanitizePreviewHtml\(getOutputText\(out.data\['text\/html'\]\)\)\}/);
  } finally {
    delete globalThis.window;
    dom.window.close();
  }
});
