import { AlertCircle, Terminal as TerminalIcon } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { useWorkspace } from './WorkspaceContext';

import {
  closeTerminal,
  listenTerminalExit,
  listenTerminalOutput,
  resizeTerminal,
  spawnTerminal,
  writeTerminal,
} from '#/lib/terminal-bridge';

import '@xterm/xterm/css/xterm.css';

type TerminalEvent =
  | { kind: 'output'; data: string }
  | { kind: 'exit'; code: number | null };

export function TerminalPanel() {
  const { workspaceRoot } = useWorkspace();
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<import('@xterm/xterm').Terminal | null>(null);
  const sessionRef = useRef<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [terminalState, setTerminalState] = useState<
    'starting' | 'ready' | 'exited' | 'error'
  >('starting');

  useEffect(() => {
    let disposed = false;
    setError(null);
    setTerminalState('starting');
    const disposers: Array<() => void> = [];
    let inputDisposable: { dispose: () => void } | undefined;

    const runDisposers = () => {
      while (disposers.length > 0) disposers.pop()?.();
    };

    async function start() {
      if (!containerRef.current) return;

      try {
        const [{ Terminal }, { FitAddon }] = await Promise.all([
          import('@xterm/xterm'),
          import('@xterm/addon-fit'),
        ]);
        if (disposed || !containerRef.current) return;

        const terminal = new Terminal({
          cursorBlink: true,
          fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
          fontSize: 12,
          theme: {
            background: '#111318',
            foreground: '#d5d8e0',
          },
        });
        const fit = new FitAddon();
        terminal.loadAddon(fit);
        terminal.open(containerRef.current);
        fit.fit();
        terminalRef.current = terminal;
        disposers.push(() => {
          terminal.dispose();
          terminalRef.current = null;
        });

        const cwd = workspaceRoot ?? (await getHomeForTerminal());
        if (disposed) {
          runDisposers();
          return;
        }
        if (!cwd) {
          setError('Open a folder to start a project terminal.');
          setTerminalState('error');
          return;
        }

        // The PTY reader starts emitting the moment the session is registered,
        // which can be before spawnTerminal() resolves with its id. Buffer per
        // session id so the first prompt is replayed instead of dropped.
        let ownSessionId: number | null = null;
        const buffered = new Map<number, TerminalEvent[]>();

        const applyEvent = (event: TerminalEvent) => {
          if (event.kind === 'output') {
            terminal.write(event.data);

            return;
          }

          terminal.write(
            `\r\n[process exited${event.code === null ? '' : ` with code ${event.code}`} ]\r\n`,
          );
          sessionRef.current = null;
          terminal.options.disableStdin = true;
          inputDisposable?.dispose();
          inputDisposable = undefined;
          setTerminalState('exited');
        };

        const routeEvent = (id: number, event: TerminalEvent) => {
          if (ownSessionId === null) {
            const queue = buffered.get(id);

            if (queue) queue.push(event);
            else buffered.set(id, [event]);

            return;
          }

          if (id === ownSessionId) applyEvent(event);
        };

        const cleanupOutput = await listenTerminalOutput(
          ({ sessionId: id, data }) => routeEvent(id, { kind: 'output', data }),
        );
        disposers.push(cleanupOutput);
        if (disposed) {
          runDisposers();
          return;
        }

        const cleanupExit = await listenTerminalExit(
          ({ sessionId: id, code }) => routeEvent(id, { kind: 'exit', code }),
        );
        disposers.push(cleanupExit);
        if (disposed) {
          runDisposers();
          return;
        }

        const sessionId = await spawnTerminal(
          cwd,
          terminal.cols,
          terminal.rows,
        );
        if (disposed) {
          runDisposers();
          await closeTerminal(sessionId);
          return;
        }

        const replay = buffered.get(sessionId) ?? [];
        buffered.clear();
        ownSessionId = sessionId;
        const exitedBeforeAttach = replay.some(
          (event) => event.kind === 'exit',
        );
        sessionRef.current = exitedBeforeAttach ? null : sessionId;
        for (const event of replay) applyEvent(event);
        if (!exitedBeforeAttach) setTerminalState('ready');

        if (!exitedBeforeAttach) {
          inputDisposable = terminal.onData((data) => {
            void writeTerminal(sessionId, data).catch((cause: unknown) =>
              setError(cause instanceof Error ? cause.message : String(cause)),
            );
          });
          disposers.push(() => {
            inputDisposable?.dispose();
            inputDisposable = undefined;
          });
        }
        const resize = () => {
          fit.fit();
          void resizeTerminal(sessionId, terminal.cols, terminal.rows).catch(
            () => undefined,
          );
        };
        window.addEventListener('resize', resize);
        disposers.push(() => window.removeEventListener('resize', resize));
      } catch (cause) {
        runDisposers();
        const sessionId = sessionRef.current;
        sessionRef.current = null;
        if (sessionId !== null) {
          void closeTerminal(sessionId).catch(() => undefined);
        }
        if (!disposed) {
          setError(cause instanceof Error ? cause.message : String(cause));
          setTerminalState('error');
        }
      }
    }

    void start();

    return () => {
      disposed = true;
      runDisposers();
      const sessionId = sessionRef.current;
      sessionRef.current = null;
      if (sessionId !== null) {
        void closeTerminal(sessionId).catch(() => undefined);
      }
    };
  }, [workspaceRoot]);

  useEffect(() => {
    const focusTerminal = () => terminalRef.current?.focus();

    window.addEventListener('qedit:focus-terminal', focusTerminal);

    return () =>
      window.removeEventListener('qedit:focus-terminal', focusTerminal);
  }, []);

  const stateLabel =
    terminalState === 'starting'
      ? 'Starting...'
      : terminalState === 'ready'
        ? 'Ready'
        : terminalState === 'exited'
          ? 'Exited'
          : 'Unavailable';

  return (
    <section className="flex h-52 min-h-0 flex-col border-t bg-[#111318]">
      <div className="flex h-7 shrink-0 items-center gap-2 border-b border-white/10 px-3 text-[11px] font-medium text-slate-300">
        <TerminalIcon className="h-3.5 w-3.5" />
        Terminal
        <span
          className={
            terminalState === 'error' ? 'text-red-300' : 'text-slate-500'
          }
        >
          {stateLabel}
        </span>
        {workspaceRoot && (
          <span className="min-w-0 truncate text-slate-500">
            {workspaceRoot}
          </span>
        )}
      </div>
      {error && (
        <div className="flex items-center gap-1.5 px-3 py-1 text-xs text-red-300">
          <AlertCircle className="h-3.5 w-3.5" />
          <span className="truncate">{error}</span>
        </div>
      )}
      <div ref={containerRef} className="min-h-0 flex-1 px-2 py-1" />
    </section>
  );
}

async function getHomeForTerminal(): Promise<string | null> {
  try {
    const { homeDir } = await import('@tauri-apps/api/path');

    return await homeDir();
  } catch {
    return null;
  }
}
