import test from 'node:test';
import assert from 'node:assert/strict';
import { createNotesSession } from '../src/lib/notesSession.ts';

const note = (path, content = '', filename = 'NOTES.md') => ({
  path: `${path}/${filename}`, filename, content, exists: true, last_modified: null,
});
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const settle = () => new Promise((resolve) => setImmediate(resolve));
function observe(session) {
  let state;
  session.subscribe((value) => { state = value; });
  return () => state;
}

test('navigation flushes edits to their original directory and filename', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const nextLoad = deferred();
  const saves = [];
  const session = createNotesSession({
    load: async (path) => path === '/A' ? note(path, 'old', 'README.md') : nextLoad.promise,
    save: async (...args) => { saves.push(args); return note(...args); },
  });
  const state = observe(session);
  await session.open('/A');
  session.edit('edited in A');
  const loading = session.open('/B');
  t.mock.timers.tick(2000);
  await settle();
  assert.deepEqual(saves, [['/A', 'edited in A', 'README.md']]);
  assert.equal(state().isLoading, true);
  assert.equal(state().content, '');
  nextLoad.resolve(note('/B', 'B content'));
  await loading;
  assert.equal(state().content, 'B content');
  await session.close();
});

test('stale directory loads and errors cannot replace the current notes', async () => {
  for (const fail of [false, true]) {
    const first = deferred();
    const session = createNotesSession({
      load: async (path) => path === '/A' ? first.promise : note(path, 'newest'),
      save: async (...args) => note(...args),
    });
    const state = observe(session);
    const stale = session.open('/A');
    await session.open('/B');
    if (fail) first.reject(new Error('stale error'));
    else first.resolve(note('/A', 'old'));
    await stale;
    assert.equal(state().content, 'newest');
    assert.equal(state().notes.path, '/B/NOTES.md');
    assert.equal(state().isLoading, false);
    assert.doesNotMatch(state().message, /stale error/);
    await session.close();
  }
});

test('edits during a write are saved afterwards without reverting the editor', async () => {
  const writes = [deferred(), deferred()];
  const saved = [];
  const session = createNotesSession({
    load: async (path) => note(path),
    save: (...args) => { saved.push(args); return writes[saved.length - 1].promise; },
  });
  const state = observe(session);
  await session.open('/A');
  session.edit('first');
  const saving = session.save();
  session.edit('second');
  assert.equal(saved.length, 1);
  writes[0].resolve(note('/A', 'first'));
  await settle();
  assert.equal(state().content, 'second');
  assert.equal(state().isDirty, true);
  assert.deepEqual(saved.map((args) => args[1]), ['first', 'second']);
  writes[1].resolve(note('/A', 'second'));
  await saving;
  assert.equal(state().isDirty, false);
  assert.equal(state().isSaving, false);
  await session.close();
});

test('unmount flushes a pending timer and does not overwrite a newly opened folder', async () => {
  const write = deferred();
  const saves = [];
  const session = createNotesSession({
    load: async (path) => note(path, path),
    save: (...args) => { saves.push(args); return write.promise; },
  });
  const state = observe(session);
  await session.open('/A');
  session.edit('last keystroke');
  const closing = session.close();
  await session.open('/B');
  write.resolve(note('/A', 'last keystroke'));
  await closing;
  assert.deepEqual(saves, [['/A', 'last keystroke', 'NOTES.md']]);
  assert.equal(state().notes.path, '/B/NOTES.md');
  assert.equal(state().content, '/B');
  await session.close();
});

test('failed writes retain the draft across unmount and can be retried', async () => {
  let failing = true;
  const saves = [];
  const session = createNotesSession({
    load: async (path) => note(path, 'disk version'),
    save: async (...args) => {
      saves.push(args);
      if (failing) throw new Error('disk full');
      return note(...args);
    },
  });
  const state = observe(session);
  await session.open('/A');
  session.edit('precious draft');
  await session.close();
  await session.open('/A');
  assert.equal(state().content, 'precious draft');
  assert.equal(state().isDirty, true);
  assert.match(state().message, /disk full/);
  failing = false;
  await session.save();
  assert.equal(state().isDirty, false);
  assert.equal(saves.at(-1)[1], 'precious draft');
  await session.close();
});

test('a failed load cannot save the previous folder content under the new path', async () => {
  const saves = [];
  const session = createNotesSession({
    load: async (path) => { if (path === '/B') throw new Error('unreadable'); return note(path, 'A'); },
    save: async (...args) => { saves.push(args); return note(...args); },
  });
  const state = observe(session);
  await session.open('/A');
  await session.open('/B');
  session.edit('must not be written');
  await session.save();
  assert.equal(state().notes, null);
  assert.equal(state().content, '');
  assert.match(state().message, /unreadable/);
  assert.deepEqual(saves, []);
  await session.close();
});

test('autosave uses the latest input and closes clean drafts without caching stale disk content', async (t) => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  let disk = 'before';
  const saves = [];
  const session = createNotesSession({
    load: async (path) => note(path, disk),
    save: async (...args) => { saves.push(args); return note(...args); },
  });
  const state = observe(session);
  await session.open('/A');
  session.edit('one');
  t.mock.timers.tick(1000);
  session.edit('two');
  t.mock.timers.tick(1499);
  assert.deepEqual(saves, []);
  t.mock.timers.tick(1);
  await settle();
  assert.deepEqual(saves, [['/A', 'two', 'NOTES.md']]);
  await session.close();
  disk = 'changed externally';
  await session.open('/A');
  assert.equal(state().content, disk);
  await session.close();
});
