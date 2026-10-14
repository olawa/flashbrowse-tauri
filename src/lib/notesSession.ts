import type { DirectoryNotes } from './types';

interface NotesIO {
  load(path: string): Promise<DirectoryNotes>;
  save(path: string, content: string, filename?: string): Promise<DirectoryNotes>;
}

interface Draft {
  path: string;
  notes: DirectoryNotes;
  content: string;
  revision: number;
  savedRevision: number;
  error: string;
  saving?: Promise<void>;
  timer?: ReturnType<typeof setTimeout>;
}

export interface NotesState {
  notes: DirectoryNotes | null;
  content: string;
  isLoading: boolean;
  isSaving: boolean;
  isDirty: boolean;
  message: string;
}

/** Owns document identities and save queues independently of the view's lifetime. */
export function createNotesSession(io: NotesIO, delay = 1500) {
  const drafts = new Map<string, Draft>();
  const listeners = new Set<(state: NotesState) => void>();
  let activePath = '';
  let generation = 0;
  let loading = false;
  let loadError = '';

  function snapshot(): NotesState {
    const draft = drafts.get(activePath);
    const dirty = !!draft && draft.revision !== draft.savedRevision;
    return {
      notes: draft?.notes ?? null,
      content: draft?.content ?? '',
      isLoading: loading,
      isSaving: !!draft?.saving,
      isDirty: dirty,
      message: loadError || draft?.error || (dirty ? 'Osparade ändringar...' :
        draft?.notes.last_modified ? `Sparad ${draft.notes.last_modified}` :
        draft?.notes.exists ? 'Laddad' : 'Ny anteckning'),
    };
  }

  function publish() {
    const state = snapshot();
    for (const listener of listeners) listener(state);
  }

  function saveDraft(draft: Draft): Promise<void> {
    clearTimeout(draft.timer);
    if (draft.saving) return draft.saving;
    if (draft.revision === draft.savedRevision) return Promise.resolve();
    draft.error = '';
    draft.saving = (async () => {
      try {
        // Changes made during a write are saved afterwards, in order.
        while (draft.revision !== draft.savedRevision) {
          const revision = draft.revision;
          const content = draft.content;
          draft.notes = await io.save(draft.path, content, draft.notes.filename);
          draft.savedRevision = revision;
        }
      } catch (error) {
        // Retain the dirty draft across navigation/unmount so it can be retried.
        draft.error = `Kunde inte spara: ${error}`;
      } finally {
        draft.saving = undefined;
        if (draft.path !== activePath && draft.revision === draft.savedRevision) {
          drafts.delete(draft.path);
        }
        publish();
      }
    })();
    publish();
    return draft.saving;
  }

  function leaveCurrent() {
    const draft = drafts.get(activePath);
    if (!draft) return;
    if (draft.revision !== draft.savedRevision) void saveDraft(draft);
    else if (!draft.saving) drafts.delete(draft.path);
  }

  return {
    subscribe(listener: (state: NotesState) => void) {
      listeners.add(listener);
      listener(snapshot());
      return () => { listeners.delete(listener); };
    },
    async open(path: string) {
      if (path === activePath && !loadError) return;
      leaveCurrent();
      activePath = path;
      const request = ++generation;
      loadError = '';
      loading = !!path && !drafts.has(path);
      publish();
      if (!loading) return;
      try {
        const notes = await io.load(path);
        if (request !== generation) return;
        drafts.set(path, { path, notes, content: notes.content, revision: 0, savedRevision: 0, error: '' });
      } catch (error) {
        if (request !== generation) return;
        loadError = `Kunde inte läsa anteckningar: ${error}`;
      } finally {
        if (request === generation) {
          loading = false;
          publish();
        }
      }
    },
    edit(content: string) {
      const draft = drafts.get(activePath);
      if (!draft || loading || draft.content === content) return;
      draft.content = content;
      draft.revision++;
      draft.error = '';
      clearTimeout(draft.timer);
      draft.timer = setTimeout(() => { void saveDraft(draft); }, delay);
      publish();
    },
    save() {
      const draft = drafts.get(activePath);
      return draft ? saveDraft(draft) : Promise.resolve();
    },
    async close() {
      generation++;
      activePath = '';
      loading = false;
      await Promise.all([...drafts.values()].map(saveDraft));
      for (const [path, draft] of drafts) {
        if (path !== activePath && !draft.saving && draft.revision === draft.savedRevision) drafts.delete(path);
      }
    },
  };
}
