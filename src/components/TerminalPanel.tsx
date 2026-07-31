import {
  AlertCircle,
  Circle,
  Plus,
  Terminal as TerminalIcon,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useReducer, useRef, useState } from 'react';

import { useSettingsOrDefault } from './SettingsContext';
import { useWorkspace } from './WorkspaceContext';

import { withThemeTokens } from '#/lib/design-tokens';
import type { TokenTheme } from '#/lib/design-tokens';
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

const TERMINAL_FALLBACKS: Record<TokenTheme, Record<string, string>> = {
  dark: {
    '--qedit-bg-editor': '#303040',
    '--qedit-text-primary': '#d4d4d4',
    '--qedit-text-accent': '#70a0ff',
    '--qedit-bg-selection': '#2050c0',
    '--qedit-ansi-black': '#1a1a24',
    '--qedit-ansi-red': '#f07178',
    '--qedit-ansi-green': '#98c379',
    '--qedit-ansi-yellow': '#e5c07b',
    '--qedit-ansi-blue': '#569cd6',
    '--qedit-ansi-magenta': '#c586c0',
    '--qedit-ansi-cyan': '#4ec9b0',
    '--qedit-ansi-white': '#d4d4d4',
    '--qedit-ansi-bright-black': '#808080',
    '--qedit-ansi-bright-red': '#ff8b92',
    '--qedit-ansi-bright-green': '#b5e890',
    '--qedit-ansi-bright-yellow': '#ffd894',
    '--qedit-ansi-bright-blue': '#7cb8f0',
    '--qedit-ansi-bright-magenta': '#dda0d8',
    '--qedit-ansi-bright-cyan': '#6fe3c9',
    '--qedit-ansi-bright-white': '#ffffff',
  },
  light: {
    '--qedit-bg-editor': '#ffffff',
    '--qedit-text-primary': '#1f1f28',
    '--qedit-text-accent': '#1a56c4',
    '--qedit-bg-selection': '#add6ff',
    '--qedit-ansi-black': '#24242e',
    '--qedit-ansi-red': '#c7262f',
    '--qedit-ansi-green': '#237d31',
    '--qedit-ansi-yellow': '#8a6100',
    '--qedit-ansi-blue': '#0f4ecc',
    '--qedit-ansi-magenta': '#9b2fae',
    '--qedit-ansi-cyan': '#0e6f78',
    '--qedit-ansi-white': '#55555f',
    '--qedit-ansi-bright-black': '#6b6b80',
    '--qedit-ansi-bright-red': '#a81f27',
    '--qedit-ansi-bright-green': '#1a6626',
    '--qedit-ansi-bright-yellow': '#6d4d00',
    '--qedit-ansi-bright-blue': '#0a3ea8',
    '--qedit-ansi-bright-magenta': '#7d2490',
    '--qedit-ansi-bright-cyan': '#0a5a62',
    '--qedit-ansi-bright-white': '#8a8a99',
  },
};

/**
 * ANSI slots are absolute ink, not UI surfaces: a shell that asks for color 0
 * or bright white means those colors whatever the theme is. They therefore read
 * dedicated `--qedit-ansi-*` tokens, so neither palette can invert them into
 * background-on-background.
 */
function terminalTheme(theme: TokenTheme) {
  const fallbacks = TERMINAL_FALLBACKS[theme];

  return withThemeTokens(theme, (read) => {
    const token = (name: string) => read(name, fallbacks[name] ?? '#000000');

    return {
      background: token('--qedit-bg-editor'),
      foreground: token('--qedit-text-primary'),
      cursor: token('--qedit-text-accent'),
      selectionBackground: token('--qedit-bg-selection'),
      black: token('--qedit-ansi-black'),
      red: token('--qedit-ansi-red'),
      green: token('--qedit-ansi-green'),
      yellow: token('--qedit-ansi-yellow'),
      blue: token('--qedit-ansi-blue'),
      magenta: token('--qedit-ansi-magenta'),
      cyan: token('--qedit-ansi-cyan'),
      white: token('--qedit-ansi-white'),
      brightBlack: token('--qedit-ansi-bright-black'),
      brightRed: token('--qedit-ansi-bright-red'),
      brightGreen: token('--qedit-ansi-bright-green'),
      brightYellow: token('--qedit-ansi-bright-yellow'),
      brightBlue: token('--qedit-ansi-bright-blue'),
      brightMagenta: token('--qedit-ansi-bright-magenta'),
      brightCyan: token('--qedit-ansi-bright-cyan'),
      brightWhite: token('--qedit-ansi-bright-white'),
    };
  });
}

/**
 * The panel stays mounted while hidden. Unmounting it would tear down every
 * PTY session along with its tabs, names, and scrollback, so visibility is a
 * presentation concern only - hiding just collapses the panel and parks every
 * terminal as inactive so it is refit and refocused on the way back.
 */
export function TerminalPanel({ visible = true }: { visible?: boolean }) {
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
    const handleNew = () => addTerminal();
    const handleClose = () => {
      if (state.activeId) closeTerminalTab(state.activeId);
    };
    const handleIndex = (event: Event) => {
      const index = (event as CustomEvent<number>).detail;
      if (Number.isInteger(index)) selectTerminalIndex(index);
    };

    window.addEventListener('qedit:terminal-next', handleNext);
    window.addEventListener('qedit:terminal-previous', handlePrevious);
    window.addEventListener('qedit:terminal-new', handleNew);
    window.addEventListener('qedit:terminal-close', handleClose);
    window.addEventListener('qedit:terminal-tab', handleIndex);

    return () => {
      window.removeEventListener('qedit:terminal-next', handleNext);
      window.removeEventListener('qedit:terminal-previous', handlePrevious);
      window.removeEventListener('qedit:terminal-new', handleNew);
      window.removeEventListener('qedit:terminal-close', handleClose);
      window.removeEventListener('qedit:terminal-tab', handleIndex);
    };
  }, [
    addTerminal,
    closeTerminalTab,
    navigateTerminal,
    selectTerminalIndex,
    state.activeId,
  ]);

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
    <section
      className={`h-full min-h-0 flex-col border-t border-border-default bg-editor ${
        visible ? 'flex' : 'hidden'
      }`}
      aria-label="Terminal panel"
      aria-hidden={!visible}
    >
      <div className="flex h-tab shrink-0 items-stretch border-b border-border-subtle text-xs font-medium text-text-secondary">
        <div className="flex shrink-0 items-center gap-2 border-r border-border-subtle px-3">
          <TerminalIcon className="h-3.5 w-3.5" />
          <span>Terminal</span>
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
          className="flex shrink-0 items-center gap-1.5 border-l border-border-subtle px-3 text-text-secondary hover:bg-hover hover:text-text-primary"
          onClick={addTerminal}
          aria-label="New terminal"
          title="New terminal"
        >
          <Plus className="h-3.5 w-3.5" />
          <span>New Terminal</span>
        </button>
      </div>
      <div className="flex h-5 shrink-0 items-center gap-2 px-3 text-[10px] text-text-dimmed">
        {activeTab && <span>{statusLabel(activeTab.status)}</span>}
        {activeTab?.error && (
          <span className="flex min-w-0 items-center gap-1 text-danger">
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
            isActive={visible && tab.id === state.activeId}
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
      ? 'text-danger'
      : tab.status === 'exited'
        ? 'text-text-dimmed'
        : tab.status === 'running'
          ? 'text-success'
          : 'text-warning';

  return (
    <div
      ref={setRef}
      draggable={!isRenaming}
      data-terminal-tab={tab.id}
      className={`group flex h-full min-w-28 max-w-56 shrink-0 cursor-pointer items-center gap-1.5 border-r px-2.5 text-[11px] transition-colors ${
        isActive
          ? 'border-t-2 border-t-accent bg-tab-active text-text-primary'
          : 'text-text-secondary hover:bg-hover hover:text-text-primary'
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
          className="h-1.5 w-1.5 shrink-0 rounded-full bg-text-accent"
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
          className="min-w-0 flex-1 rounded border border-focus bg-input-shell px-1 text-[11px] text-text-primary outline-none"
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
            className="rounded p-0.5 hover:bg-hover"
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
  const { settings, resolvedTheme } = useSettingsOrDefault();
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<import('@xterm/xterm').Terminal | null>(null);
  const fitRef = useRef<import('@xterm/addon-fit').FitAddon | null>(null);
  const sessionRef = useRef<number | null>(null);
  const initialWorkspaceRootRef = useRef(workspaceRoot);
  const activeRef = useRef(isActive);
  const settingsRef = useRef(settings);
  const resolvedThemeRef = useRef(resolvedTheme);
  activeRef.current = isActive;
  settingsRef.current = settings;
  resolvedThemeRef.current = resolvedTheme;

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
          fontSize: settingsRef.current.terminalFontSize,
          theme: terminalTheme(resolvedThemeRef.current),
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
    const terminal = terminalRef.current;
    if (!terminal) return;

    terminal.options.fontSize = settings.terminalFontSize;
    terminal.options.theme = terminalTheme(resolvedTheme);
    if (activeRef.current) fitRef.current?.fit();
  }, [resolvedTheme, settings.terminalFontSize]);

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
