import { writable, get } from 'svelte/store';
import { runCommand, tabComplete } from '../invoke';
import { leftPane, rightPane, activePaneId, navigatePane } from './navigation';

export interface TerminalLine {
  id: number;
  text: string;
  isError?: boolean;
  isPrompt?: boolean;
}

let nextLineId = 1;

export const isTerminalOpen = writable<boolean>(false);
export const terminalDockPosition = writable<'bottom' | 'side'>('bottom');
export const terminalLines = writable<TerminalLine[]>([
  { id: nextLineId++, text: '⚡ Flashbrowse Cross-Platform Terminal (2-Way Synced)', isPrompt: true },
  { id: nextLineId++, text: 'Commands like "cd <dir>" update browser panels in real time.', isPrompt: true },
  { id: nextLineId++, text: '---------------------------------------------------------', isPrompt: true },
]);

export const commandHistory = writable<string[]>([]);
export const historyIndex = writable<number>(-1);
export const isExecuting = writable<boolean>(false);

export function toggleTerminal() {
  isTerminalOpen.update((open) => !open);
}

export function toggleTerminalDock() {
  terminalDockPosition.update((pos) => (pos === 'bottom' ? 'side' : 'bottom'));
}

export async function executeTerminalCommand(input: string) {
  const trimmed = input.trim();
  if (!trimmed) return;

  const activeId = get(activePaneId);
  const currentPane = activeId === 'left' ? get(leftPane) : get(rightPane);
  const cwd = currentPane.currentPath;

  // Add prompt line
  terminalLines.update((lines) => [
    ...lines,
    { id: nextLineId++, text: `$ ${trimmed}`, isPrompt: true },
  ]);

  // Update history
  commandHistory.update((hist) => [...hist, trimmed]);
  historyIndex.set(-1);

  if (trimmed === 'clear') {
    terminalLines.set([]);
    return;
  }

  isExecuting.set(true);

  try {
    const res = await runCommand(trimmed, cwd);

    if (res.stdout) {
      const outLines = res.stdout.split('\n');
      terminalLines.update((lines) => [
        ...lines,
        ...outLines.filter(Boolean).map((t) => ({ id: nextLineId++, text: t })),
      ]);
    }

    if (res.stderr) {
      const errLines = res.stderr.split('\n');
      terminalLines.update((lines) => [
        ...lines,
        ...errLines.filter(Boolean).map((t) => ({ id: nextLineId++, text: t, isError: true })),
      ]);
    }

    // Handle cd directory sync
    if (res.new_cwd) {
      await navigatePane(activeId, res.new_cwd, true);
    }
  } catch (err: any) {
    terminalLines.update((lines) => [
      ...lines,
      { id: nextLineId++, text: `Error: ${err}`, isError: true },
    ]);
  } finally {
    isExecuting.set(false);
  }
}

export async function requestTabCompletion(input: string): Promise<string> {
  const activeId = get(activePaneId);
  const currentPane = activeId === 'left' ? get(leftPane) : get(rightPane);
  const cwd = currentPane.currentPath;

  try {
    const res = await tabComplete(input, cwd);
    if (res.suggestions.length > 0) {
      // Print suggestions in terminal log
      const suggestionText = res.suggestions.join('    ');
      terminalLines.update((lines) => [
        ...lines,
        { id: nextLineId++, text: suggestionText },
      ]);
    }
    return res.completed_line || input;
  } catch {
    return input;
  }
}
