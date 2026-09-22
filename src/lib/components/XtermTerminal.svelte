<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { ptySpawn, ptyWrite, ptyResize, ptyKill, ptyHasSession } from '../invoke';

  export let sessionId: string = 'local';
  export let sessionType: 'local' | 'ssh' = 'local';
  export let cwd: string = '';
  export let host: string | undefined = undefined;
  export let onDirectoryChange: ((newPath: string) => void) | undefined = undefined;

  let container: HTMLDivElement;
  let term: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let unlistenOutput: UnlistenFn | null = null;
  let unlistenExit: UnlistenFn | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let isExited = false;

  export function clear() {
    term?.clear();
  }

  export function focus() {
    term?.focus();
  }

  export async function restart() {
    if (!term || !fitAddon) return;
    try {
      await ptyKill(sessionId);
    } catch {
      // ignore
    }
    isExited = false;
    term.reset();
    fitAddon.fit();
    try {
      await ptySpawn(sessionId, sessionType, cwd, host, term.rows, term.cols);
      term.focus();
    } catch (err: any) {
      term.write(`\r\n\x1b[31mKunde inte starta terminal: ${err?.message || err}\x1b[0m\r\n`);
    }
  }

  export function sendCd(path: string) {
    if (!path) return;
    const escaped = path.replace(/'/g, "'\\''");
    ptyWrite(sessionId, `cd '${escaped}'\r`);
  }

  export function sendInput(data: string) {
    ptyWrite(sessionId, data);
  }

  onMount(async () => {
    term = new Terminal({
      cursorBlink: true,
      cursorStyle: 'bar',
      fontSize: 12,
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace',
      lineHeight: 1.25,
      scrollback: 5000,
      allowProposedApi: true,
      theme: {
        background: '#0c0d10',
        foreground: '#f1f5f9',
        cursor: '#e85422',
        cursorAccent: '#0c0d10',
        selectionBackground: 'rgba(232, 84, 34, 0.35)',
        selectionForeground: '#ffffff',
        black: '#191d24',
        red: '#f87171',
        green: '#34d399',
        yellow: '#fbbf24',
        blue: '#60a5fa',
        magenta: '#c084fc',
        cyan: '#22d3ee',
        white: '#f1f5f9',
        brightBlack: '#64748b',
        brightRed: '#ef4444',
        brightGreen: '#10b981',
        brightYellow: '#f59e0b',
        brightBlue: '#3b82f6',
        brightMagenta: '#a855f7',
        brightCyan: '#06b6d4',
        brightWhite: '#ffffff',
      },
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(container);

    // Initial fit
    try {
      fitAddon.fit();
    } catch {
      // ignore
    }

    // OSC 7 handler for shell directory notification (macOS zsh / modern Linux shells)
    // OSC 7 emits: \x1b]7;file://hostname/path/to/cwd\x07
    term.parser.registerOscHandler(7, (data) => {
      try {
        const text = typeof data === 'string' ? data : new TextDecoder().decode(data);
        if (text.startsWith('file://')) {
          const parsed = new URL(text);
          const decoded = decodeURIComponent(parsed.pathname);
          if (decoded && onDirectoryChange) {
            onDirectoryChange(decoded);
          }
        }
      } catch {
        // ignore parsing errors
      }
      return true;
    });

    // Listen for incoming PTY output from backend
    unlistenOutput = await listen<string>(`pty-output-${sessionId}`, (event) => {
      if (!term) return;
      try {
        const raw = atob(event.payload);
        const len = raw.length;
        const bytes = new Uint8Array(len);
        for (let i = 0; i < len; i++) {
          bytes[i] = raw.charCodeAt(i);
        }
        term.write(bytes);
      } catch {
        term.write(event.payload);
      }
    });

    // Listen for PTY process termination
    unlistenExit = await listen(`pty-exit-${sessionId}`, () => {
      if (!term) return;
      term.write('\r\n\x1b[33m[Processen avslutades - Tryck Retur för att starta om]\x1b[0m\r\n');
      isExited = true;
    });

    // Wire user keystrokes to PTY stdin
    term.onData((data) => {
      if (isExited) {
        if (data === '\r' || data === '\n') {
          restart();
        }
        return;
      }
      ptyWrite(sessionId, data);
    });

    // Check if session exists in backend; if not, spawn it
    try {
      const exists = await ptyHasSession(sessionId);
      if (!exists) {
        const rows = term.rows > 0 ? term.rows : 24;
        const cols = term.cols > 0 ? term.cols : 80;
        await ptySpawn(sessionId, sessionType, cwd, host, rows, cols);
      } else {
        // Trigger a redraw for existing session
        ptyWrite(sessionId, '\x0c'); // Ctrl+L redraw
      }
    } catch (err: any) {
      term.write(`\r\n\x1b[31mKunde inte starta terminalsession (${sessionId}): ${err?.message || err}\x1b[0m\r\n`);
    }

    // Observe size changes to adjust PTY rows/cols dynamically
    resizeObserver = new ResizeObserver(() => {
      if (!container || !term || !fitAddon) return;
      if (container.clientWidth <= 0 || container.clientHeight <= 0) return;
      try {
        fitAddon.fit();
        if (term.rows > 0 && term.cols > 0) {
          ptyResize(sessionId, term.rows, term.cols);
        }
      } catch {
        // ignore fit errors during hidden state
      }
    });
    resizeObserver.observe(container);

    term.focus();
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    if (unlistenOutput) unlistenOutput();
    if (unlistenExit) unlistenExit();
    term?.dispose();
    term = null;
    fitAddon = null;
  });
</script>

<div
  bind:this={container}
  class="w-full h-full overflow-hidden bg-[var(--bg-base)] p-1.5 focus:outline-none"
  tabindex="0"
  on:click={() => term?.focus()}
  on:keydown={() => {}}
  role="application"
  aria-label="Terminal emulator"
></div>

<style>
  div :global(.xterm) {
    height: 100%;
    padding: 0;
  }
  div :global(.xterm-viewport) {
    overflow-y: auto !important;
  }
</style>
