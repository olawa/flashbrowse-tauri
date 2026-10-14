import { getDirectoryNotes, saveDirectoryNotes } from '../invoke';
import { createNotesSession } from '../notesSession';

// Keep pending writes and failed drafts when the notes tab is unmounted.
export const directoryNotes = createNotesSession({
  load: getDirectoryNotes,
  save: saveDirectoryNotes,
});
