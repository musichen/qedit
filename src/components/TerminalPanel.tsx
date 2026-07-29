import {
  AlertCircle,
  Circle,
  Plus,
  Terminal as TerminalIcon,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useReducer, useRef, useState } from 'react';

import { useWorkspace } from './WorkspaceContext';

import {
  closeTerminal,
  listenTerminalExit,
  listenTerminalOutput,
  resizeTerminal,
  spawnTerminal,
  writeTerminal,
} from '#/lib/terminal-bridge';
import {
  createTerminalTabsState,
  terminalTabsReducer,
  terminalTabIndexForId,
} from '#/lib/terminal-tabs';
import type { TerminalStatus, TerminalTabState } from '#/lib/terminal-tabs';

import '@xterm/xterm/css/xterm.css';

type TerminalEvent =
  | { kind: 'output'; data: string }
  | { kind: 'exit'; code: number | null };

export function TerminalPanel() {
  const { workspaceRoot } = useWorkspace();
  const [state, dispatch] = useReducer(
    terminalTabsReducer,
    undefined,
    createTerminalTabsState,
  );
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const renameCancelledRef = useRef(false);
  const nextNumberRef = useRef(2);
  const tabRefs = useRef(new Map<string, HTMLDivElement>());

  const activeTab = state.tabs.find((tab) => tab.id === state.activeId);

  const setStatus = useCallback(
    (id: string, status: TerminalStatus, error?: string | null) => {
      dispatch({ type: 'status', id, status, error });
    },
    [],
  );
  const markActivity = useCallback((id: string) => {
    dispatch({ type: 'activity', id });
  }, []);

  const addTerminal = useCallback(() => {
    const number = nextNumberRef.current++;
    const id = `terminal-${number}`;

    dispatch({
      type: 'add',
      tab: {
        id,
        name: `Terminal ${number}`,
        status: 'starting',
        isDirty: false,
        error: null,
      },
    });
  }, []);

  const selectTerminal = useCallback((id: string) => {
    dispatch({ type: 'activate', id });
  }, []);

  const closeTerminalTab = useCallback((id: string) => {
    setRenamingId((current) => (current === id ? null : current));
    dispatch({ type: 'close', id });
  }, []);

  const reorderTerminal = useCallback((id: string, toIndex: number) => {
    dispatch({ type: 'reorder', id, toIndex });
  }, []);

  const navigateTerminal = useCallback(
    (direction: 'next' | 'previous' | 'first' | 'last') => {
      if (state.tabs.length === 0) return;

      const activeIndex = state.activeId
        ? terminalTabIndexForId(state.tabs, state.activeId)
        : 0;
      const index =
        direction === 'next'
          ? (activeIndex + 1) % state.tabs.length
          : direction === 'previous'
            ? (activeIndex - 1 + state.tabs.length) % state.tabs.length
            : direction === 'first'
              ? 0
              : state.tabs.length - 1;
      const tab = state.tabs[index];

      if (!tab) return;

      selectTerminal(tab.id);
      requestAnimationFrame(() => tabRefs.current.get(tab.id)?.focus());
    },
    [selectTerminal, state.activeId, state.tabs],
  );

  const selectTerminalIndex = useCallback(
    (index: number) => {
      const tab = state.tabs[index];
      if (!tab) return;

      selectTerminal(tab.id);
      requestAnimationFrame(() => tabRefs.current.get(tab.id)?.focus());
    },
    [selectTerminal, state.tabs],
  );

  useEffect(() => {
    const handleNext = () => navigateTerminal('next');
    const handlePrevious = () => navigateTerminal('previous');
    const handleClose = () => {
      if (state.activeId) closeTerminalTab(state.activeId);
    };
    const handleIndex = (event: Event) => {
      const index = (event as CustomEvent<number>).detail;
      if (Number.isInteger(index)) selectTerminalIndex(index);
    };

    window.addEventListener('qedit:terminal-next', handleNext);
    window.addEventListener('qedit:terminal-previous', handlePrevious);
    window.addEventListener('qedit:terminal-close', handleClose);
    window.addEventListener('qedit:terminal-tab', handleIndex);

    return () => {
      window.removeEventListener('qedit:terminal-next', handleNext);
      window.removeEventListener('qedit:terminal-previous', handlePrevious);
      window.removeEventListener('qedit:terminal-close', handleClose);
      window.removeEventListener('qedit:terminal-tab', handleIndex);
    };
  }, [closeTerminalTab, navigateTerminal, selectTerminalIndex, state.activeId]);

  const beginRename = useCallback((tab: TerminalTabState) => {
    renameCancelledRef.current = false;
    setRenamingId(tab.id);
    setRenameValue(tab.name);
  }, []);

  const commitRename = useCallback(() => {
    if (!renamingId) return;

    if (renameCancelledRef.current) {
      renameCancelledRef.current = false;
      setRenamingId(null);
      return;
    }

    const name = renameValue.trim();
    if (name) dispatch({ type: 'rename', id: renamingId, name });
    setRenamingId(null);
  }, [renameValue, renamingId]);

  const cancelRename = useCallback(() => {
    renameCancelledRef.current = true;
    setRenamingId(null);
  }, []);

  const activeIndex = state.activeId
    ? terminalTabIndexForId(state.tabs, state.activeId)
    : -1;

  return (
    <section className="flex h-52 min-h-0 flex-col border-t bg-[#111318]">
      <div className="flex h-8 shrink-0 items-stretch border-b border-white/10 text-[11px] font-medium text-slate-300">
        <div className="flex shrink-0 items-center gap-2 px-3">
          <TerminalIcon className="h-3.5 w-3.5" />
          <span className="sr-only">Integrated terminals</span>
        </div>
        <div
          className="flex min-w-0 flex-1 items-stretch overflow-x-auto"
          role="tablist"
          aria-label="Integrated terminals"
        >
          {state.tabs.map((tab, index) => (
            <TerminalTab
              key={tab.id}
              tab={tab}
              isActive={tab.id === state.activeId}
              isRenaming={tab.id === renamingId}
              renameValue={renameValue}
              setRef={(node) => {
                if (node) tabRefs.current.set(tab.id, node);
                else tabRefs.current.delete(tab.id);
              }}
              onSelect={() => selectTerminal(tab.id)}
              onClose={() => closeTerminalTab(tab.id)}
              onRename={() => beginRename(tab)}
              onRenameChange={setRenameValue}
              onRenameCommit={commitRename}
              onRenameCancel={cancelRename}
              onNavigate={(direction) => {
                const nextIndex =
                  direction === 'next'
                    ? (index + 1) % state.tabs.length
                    : direction === 'previous'
                      ? (index - 1 + state.tabs.length) % state.tabs.length
                      : direction === 'first'
                        ? 0
                        : state.tabs.length - 1;
                const nextTab = state.tabs[nextIndex];
                if (nextTab) {
                  selectTerminal(nextTab.id);
                  requestAnimationFrame(() =>
                    tabRefs.current.get(nextTab.id)?.focus(),
                  );
                }
              }}
              onReorder={(delta) => reorderTerminal(tab.id, index + delta)}
              onDropTab={(sourceId) => {
                const sourceIndex = terminalTabIndexForId(state.tabs, sourceId);
                if (sourceIndex < 0) return;

                reorderTerminal(
                  sourceId,
                  sourceIndex < index ? index - 1 : index,
                );
              }}
            />
          ))}
        </div>
        <button
          type="button"
          className="flex shrink-0 items-center border-l border-white/10 px-2 text-slate-400 hover:bg-white/5 hover:text-slate-100"
          onClick={addTerminal}
          aria-label="New terminal"
          title="New terminal"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>
      <div className="flex h-5 shrink-0 items-center gap-2 px-3 text-[10px] text-slate-500">
        {activeTab && <span>{statusLabel(activeTab.status)}</span>}
        {activeTab?.error && (
          <span className="flex min-w-0 items-center gap-1 text-red-300">
            <AlertCircle className="h-3 w-3" />
            <span className="truncate">{activeTab.error}</span>
          </span>
        )}
        {workspaceRoot && (
          <span className="min-w-0 truncate">{workspaceRoot}</span>
        )}
        {activeIndex >= 0 && (
          <span className="ml-auto">
            {activeIndex + 1}/{state.tabs.length}
          </span>
        )}
      </div>
      <div className="relative min-h-0 flex-1">
        {state.tabs.map((tab) => (
          <TerminalInstance
            key={tab.id}
            id={tab.id}
            isActive={tab.id === state.activeId}
            workspaceRoot={workspaceRoot}
            onStatus={setStatus}
            onOutput={markActivity}
          />
        ))}
      </div>
    </section>
  );
}

function statusLabel(status: TerminalStatus): string {
  switch (status) {
    case 'starting':
      return 'Starting';
    case 'running':
      return 'Running';
    case 'exited':
      return 'Closed';
    case 'error':
      return 'Unavailable';
  }
}

function TerminalTab({
  tab,
  isActive,
  isRenaming,
  renameValue,
  setRef,
  onSelect,
  onClose,
  onRename,
  onRenameChange,
  onRenameCommit,
  onRenameCancel,
  onNavigate,
  onReorder,
  onDropTab,
}: {
  tab: TerminalTabState;
  isActive: boolean;
  isRenaming: boolean;
  renameValue: string;
  setRef: (node: HTMLDivElement | null) => void;
  onSelect: () => void;
  onClose: () => void;
  onRename: () => void;
  onRenameChange: (value: string) => void;
  onRenameCommit: () => void;
  onRenameCancel: () => void;
  onNavigate: (direction: 'next' | 'previous' | 'first' | 'last') => void;
  onReorder: (delta: number) => void;
  onDropTab: (sourceId: string) => void;
}) {
  const renameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isRenaming) {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    }
  }, [isRenaming]);

  const statusClass =
    tab.status === 'error'
      ? 'text-red-300'
      : tab.status === 'exited'
        ? 'text-slate-500'
        : tab.status === 'running'
          ? 'text-emerald-300'
          : 'text-amber-300';

  return (
    <div
      ref={setRef}
      draggable={!isRenaming}
      data-terminal-tab={tab.id}
      className={`group flex h-full min-w-28 max-w-56 shrink-0 cursor-pointer items-center gap-1.5 border-r border-white/10 px-2.5 text-[11px] transition-colors ${
        isActive
          ? 'border-t-2 border-t-primary bg-[#181b22] text-slate-100'
          : 'text-slate-400 hover:bg-white/5 hover:text-slate-200'
      }`}
      onClick={onSelect}
      onDoubleClick={(event) => {
        event.preventDefault();
        onRename();
      }}
      onDragStart={(event) => {
        event.dataTransfer.effectAllowed = 'move';
        event.dataTransfer.setData('text/qedit-terminal', tab.id);
      }}
      onDragOver={(event) => {
        event.preventDefault();
        event.dataTransfer.dropEffect = 'move';
      }}
      onDrop={(event) => {
        event.preventDefault();
        const sourceId = event.dataTransfer.getData('text/qedit-terminal');
        if (sourceId) onDropTab(sourceId);
      }}
      onKeyDown={(event) => {
        if (event.key === 'F2') {
          event.preventDefault();
          onRename();
          return;
        }
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect();
          return;
        }
        if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
          event.preventDefault();
          const direction = event.key === 'ArrowLeft' ? -1 : 1;
          if (event.metaKey || event.ctrlKey) {
            onReorder(direction);
          } else {
            onNavigate(direction < 0 ? 'previous' : 'next');
          }
          return;
        }
        if (event.key === 'Home' || event.key === 'End') {
          event.preventDefault();
          onNavigate(event.key === 'Home' ? 'first' : 'last');
        }
      }}
      role="tab"
      aria-selected={isActive}
      aria-label={`${tab.name}, ${statusLabel(tab.status)}${tab.isDirty ? ', unread output' : ''}`}
      aria-current={isActive ? 'page' : undefined}
      tabIndex={isActive ? 0 : -1}
    >
      <Circle className={`h-2 w-2 shrink-0 fill-current ${statusClass}`} />
      {tab.isDirty && (
        <span
          className="h-1.5 w-1.5 shrink-0 rounded-full bg-sky-300"
          aria-label="Unread output"
        />
      )}
      {isRenaming ? (
        <input
          ref={renameInputRef}
          value={renameValue}
          onChange={(event) => onRenameChange(event.target.value)}
          onClick={(event) => event.stopPropagation()}
          onKeyDown={(event) => {
            event.stopPropagation();
            if (event.key === 'Enter') {
              event.preventDefault();
              onRenameCommit();
            } else if (event.key === 'Escape') {
              event.preventDefault();
              onRenameCancel();
            }
          }}
          onBlur={onRenameCommit}
          className="min-w-0 flex-1 rounded border border-primary/60 bg-background px-1 text-[11px] text-foreground outline-none"
          aria-label={`Rename ${tab.name}`}
        />
      ) : (
        <span className="min-w-0 flex-1 truncate text-left">{tab.name}</span>
      )}
      {!isRenaming && (
        <span className="flex shrink-0 items-center opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
          <span className="sr-only">Close </span>
          <button
            type="button"
            className="rounded p-0.5 hover:bg-white/10"
            onClick={(event) => {
              event.stopPropagation();
              onClose();
            }}
            aria-label={`Close ${tab.name}`}
          >
            <X className="h-3 w-3" />
          </button>
        </span>
      )}
      <span className="sr-only">
        Use Ctrl/Cmd+Arrow Left or Right to reorder; F2 to rename.
      </span>
    </div>
  );
}

function TerminalInstance({
  id,
  isActive,
  workspaceRoot,
  onStatus,
  onOutput,
}: {
  id: string;
  isActive: boolean;
  workspaceRoot: string | null;
  onStatus: (id: string, status: TerminalStatus, error?: string | null) => void;
  onOutput: (id: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<import('@xterm/xterm').Terminal | null>(null);
  const fitRef = useRef<import('@xterm/addon-fit').FitAddon | null>(null);
  const sessionRef = useRef<number | null>(null);
  const initialWorkspaceRootRef = useRef(workspaceRoot);
  const activeRef = useRef(isActive);
  activeRef.current = isActive;

  useEffect(() => {
    let disposed = false;
    let inputDisposable: { dispose: () => void } | undefined;
    const disposers: Array<() => void> = [];

    const runDisposers = () => {
      while (disposers.length > 0) disposers.pop()?.();
    };

    const closeSession = (sessionId: number) => {
      void closeTerminal(sessionId).catch((cause: unknown) => {
        console.error('Failed to close terminal:', cause);
      });
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
        fitRef.current = fit;
        terminalRef.current = terminal;
        if (activeRef.current) fit.fit();
        disposers.push(() => {
          terminal.dispose();
          terminalRef.current = null;
          fitRef.current = null;
        });

        // A terminal owns its cwd for its entire lifetime. New tabs receive
        // the current workspace root, but opening another folder must not
        // restart existing shells or change their independent process state.
        const cwd =
          initialWorkspaceRootRef.current ?? (await getHomeForTerminal());
        if (disposed) {
          runDisposers();
          return;
        }
        if (!cwd) {
          onStatus(
            id,
            'error',
            'Could not determine a safe terminal directory.',
          );
          runDisposers();
          return;
        }

        // The PTY reader starts emitting as soon as the session is registered,
        // which can be before spawnTerminal resolves with its id. Buffer per
        // session id so the first prompt is replayed instead of dropped.
        let ownSessionId: number | null = null;
        const buffered = new Map<number, TerminalEvent[]>();

        const applyEvent = (event: TerminalEvent) => {
          if (event.kind === 'output') {
            terminal.write(event.data);
            onOutput(id);
            return;
          }

          terminal.write(
            `\r\n[process exited${event.code === null ? '' : ` with code ${event.code}`} ]\r\n`,
          );
          sessionRef.current = null;
          terminal.options.disableStdin = true;
          inputDisposable?.dispose();
          inputDisposable = undefined;
          onStatus(id, 'exited');
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
          closeSession(sessionId);
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
        if (!exitedBeforeAttach) onStatus(id, 'running');

        if (!exitedBeforeAttach) {
          inputDisposable = terminal.onData((data) => {
            void writeTerminal(sessionId, data).catch((cause: unknown) =>
              onStatus(
                id,
                'error',
                cause instanceof Error ? cause.message : String(cause),
              ),
            );
          });
          disposers.push(() => {
            inputDisposable?.dispose();
            inputDisposable = undefined;
          });
        }

        const resize = () => {
          if (!activeRef.current) return;

          fit.fit();
          void resizeTerminal(sessionId, terminal.cols, terminal.rows).catch(
            (cause: unknown) =>
              console.error('Failed to resize terminal:', cause),
          );
        };
        window.addEventListener('resize', resize);
        disposers.push(() => window.removeEventListener('resize', resize));
        resize();
      } catch (cause) {
        runDisposers();
        const sessionId = sessionRef.current;
        sessionRef.current = null;
        if (sessionId !== null) closeSession(sessionId);
        if (!disposed) {
          onStatus(
            id,
            'error',
            cause instanceof Error ? cause.message : String(cause),
          );
        }
      }
    }

    onStatus(id, 'starting');
    void start();

    return () => {
      disposed = true;
      runDisposers();
      const sessionId = sessionRef.current;
      sessionRef.current = null;
      if (sessionId !== null) closeSession(sessionId);
    };
  }, [id, onOutput, onStatus]);

  useEffect(() => {
    const focusTerminal = () => {
      if (activeRef.current) terminalRef.current?.focus();
    };

    window.addEventListener('qedit:focus-terminal', focusTerminal);
    if (isActive) requestAnimationFrame(focusTerminal);

    return () =>
      window.removeEventListener('qedit:focus-terminal', focusTerminal);
  }, [isActive]);

  useEffect(() => {
    if (!isActive || !terminalRef.current || !fitRef.current) return;

    fitRef.current.fit();
    const sessionId = sessionRef.current;
    if (sessionId !== null) {
      void resizeTerminal(
        sessionId,
        terminalRef.current.cols,
        terminalRef.current.rows,
      ).catch((cause: unknown) =>
        console.error('Failed to resize active terminal:', cause),
      );
    }
  }, [isActive]);

  return (
    <div
      ref={containerRef}
      className={`absolute inset-0 min-h-0 px-2 py-1 ${isActive ? '' : 'hidden'}`}
      aria-hidden={!isActive}
    />
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
