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

export function TerminalPanel() {
  const { workspaceRoot } = useWorkspace();
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<import('@xterm/xterm').Terminal | null>(null);
  const sessionRef = useRef<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let cleanupOutput = () => {};
    let cleanupExit = () => {};
    let disposeTerminal = () => {};

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
        disposeTerminal = () => terminal.dispose();

        const cwd = workspaceRoot ?? (await getHomeForTerminal());
        if (!cwd || disposed) {
          setError('Open a folder to start a project terminal.');
          return;
        }

        cleanupOutput = await listenTerminalOutput(
          ({ sessionId: id, data }) => {
            if (id === sessionRef.current) terminal.write(data);
          },
        );
        cleanupExit = await listenTerminalExit(({ sessionId: id, code }) => {
          if (id === sessionRef.current) {
            terminal.write(
              `\r\n[process exited${code === null ? '' : ` with code ${code}`} ]\r\n`,
            );
            sessionRef.current = null;
          }
        });
        const sessionId = await spawnTerminal(
          cwd,
          terminal.cols,
          terminal.rows,
        );
        if (disposed) {
          await closeTerminal(sessionId);
          return;
        }
        sessionRef.current = sessionId;
        terminal.onData((data) => {
          void writeTerminal(sessionId, data).catch((cause: unknown) =>
            setError(cause instanceof Error ? cause.message : String(cause)),
          );
        });
        const resize = () => {
          fit.fit();
          void resizeTerminal(sessionId, terminal.cols, terminal.rows).catch(
            () => undefined,
          );
        };
        window.addEventListener('resize', resize);
        disposeTerminal = () => {
          window.removeEventListener('resize', resize);
          terminal.dispose();
        };
      } catch (cause) {
        if (!disposed) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      }
    }

    void start();

    return () => {
      disposed = true;
      cleanupOutput();
      cleanupExit();
      disposeTerminal();
      const sessionId = sessionRef.current;
      sessionRef.current = null;
      if (sessionId !== null) void closeTerminal(sessionId);
    };
  }, [workspaceRoot]);

  return (
    <section className="flex h-52 min-h-0 flex-col border-t bg-[#111318]">
      <div className="flex h-7 shrink-0 items-center gap-2 border-b border-white/10 px-3 text-[11px] font-medium text-slate-300">
        <TerminalIcon className="h-3.5 w-3.5" />
        Terminal
        {workspaceRoot && (
          <span className="truncate text-slate-500">{workspaceRoot}</span>
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
