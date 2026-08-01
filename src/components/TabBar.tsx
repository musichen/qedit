import { Save, Terminal as TerminalIcon, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import type { DragEvent } from 'react';

import { useEditor } from './EditorContext';
import type { OpenTab } from './EditorContext';

import { logInfo } from '#/lib/app-logging';
import {
  getTerminalDragId,
  isTerminalDragOver,
  setTerminalDragData,
} from '#/lib/terminal-drag';

export function TabBar({
  onOpenTerminal,
  onOpenTerminalEditor,
  terminalVisible = false,
  terminalEditorVisible = false,
  terminalEditorId,
  onDropTerminal,
  onCloseTerminalEditor,
}: {
  onOpenTerminal?: () => void;
  onOpenTerminalEditor?: () => void;
  terminalVisible?: boolean;
  terminalEditorVisible?: boolean;
  onDropTerminal?: (terminalId: string) => void;
  onCloseTerminalEditor?: () => void;
  terminalEditorId?: string | null;
}) {
  const {
    openTabs,
    activeFilePath,
    setActiveFile,
    closeTab,
    saveActiveFileAs,
  } = useEditor();
  const tabRefs = useRef(new Map<string, HTMLDivElement>());
  const [terminalDropActive, setTerminalDropActive] = useState(false);

  useEffect(() => {
    const handlePointerDrag = (event: Event) => {
      const detail = (
        event as CustomEvent<{
          id?: unknown;
          phase?: unknown;
          x?: unknown;
          y?: unknown;
        }>
      ).detail;
      if (
        typeof detail?.id !== 'string' ||
        typeof detail.x !== 'number' ||
        typeof detail.y !== 'number'
      )
        return;

      const target = document
        .elementFromPoint(detail.x, detail.y)
        ?.closest('[data-qedit-terminal-drop-target="true"]');
      const isTarget = target !== null;
      setTerminalDropActive(isTarget && detail.phase === 'move');

      if (isTarget && detail.phase === 'drop') {
        setTerminalDropActive(false);
        void logInfo(
          `terminal session pointer-dropped into file tab bar id=${detail.id}`,
        );
        onDropTerminal?.(detail.id);
      }
    };

    window.addEventListener('qedit:terminal-pointer-drag', handlePointerDrag);

    return () =>
      window.removeEventListener(
        'qedit:terminal-pointer-drag',
        handlePointerDrag,
      );
  }, [onDropTerminal]);

  const handleTerminalDrop = (event: DragEvent) => {
    const terminalId = getTerminalDragId(event.dataTransfer);
    if (!terminalId) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    setTerminalDropActive(false);
    void logInfo(`terminal session dropped into file tab bar id=${terminalId}`);
    onDropTerminal?.(terminalId);
  };

  const handleTerminalDragOver = (event: DragEvent) => {
    if (!isTerminalDragOver(event.dataTransfer)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    setTerminalDropActive(true);
  };

  const navigateTab = (index: number) => {
    const tab = openTabs[index];

    if (!tab) return;

    setActiveFile(tab.path);
    requestAnimationFrame(() => tabRefs.current.get(tab.path)?.focus());
  };

  return (
    <div
      className="flex h-tab items-stretch border-b border-border-subtle bg-tab"
      data-qedit-terminal-drop-target="true"
      onDragEnter={handleTerminalDragOver}
      onDragOver={handleTerminalDragOver}
      onDragLeave={() => setTerminalDropActive(false)}
      onDrop={handleTerminalDrop}
    >
      <div
        className={`flex min-w-0 flex-1 items-stretch overflow-x-auto ${
          terminalDropActive ? 'bg-accent/10 ring-1 ring-inset ring-accent' : ''
        }`}
        role="tablist"
        aria-label="Open files"
      >
        {openTabs.length === 0 && !terminalEditorVisible ? (
          <span className="flex min-w-0 flex-1 items-center px-4 text-xs text-text-muted">
            No open files
          </span>
        ) : (
          openTabs.map((tab, index) => (
            <Tab
              key={tab.path}
              tab={tab}
              isActive={tab.path === activeFilePath}
              onSelect={() => setActiveFile(tab.path)}
              onClose={() => closeTab(tab.path)}
              setRef={(node) => {
                if (node) tabRefs.current.set(tab.path, node);
                else tabRefs.current.delete(tab.path);
              }}
              onNavigate={(direction) => {
                const nextIndex =
                  direction === 'next'
                    ? (index + 1) % openTabs.length
                    : direction === 'previous'
                      ? (index - 1 + openTabs.length) % openTabs.length
                      : direction === 'first'
                        ? 0
                        : openTabs.length - 1;

                navigateTab(nextIndex);
              }}
            />
          ))
        )}
        {terminalEditorVisible && (
          <TerminalEditorTab
            terminalId={terminalEditorId}
            onSelect={onOpenTerminalEditor}
            onClose={onCloseTerminalEditor}
          />
        )}
      </div>
      <button
        type="button"
        draggable={!terminalEditorVisible}
        className={`flex h-full shrink-0 items-center gap-1.5 border-r border-border-subtle px-3 text-xs transition-colors ${
          terminalVisible || terminalEditorVisible
            ? 'border-t-2 border-t-accent bg-tab-active text-text-primary'
            : 'cursor-grab text-text-secondary hover:bg-hover hover:text-text-primary active:cursor-grabbing'
        }`}
        onClick={onOpenTerminal}
        onDragStart={(event) => {
          event.dataTransfer.effectAllowed = 'move';
          setTerminalDragData(event.dataTransfer, 'active');
          void logInfo('terminal action drag started from file tab bar');
        }}
        aria-label="Open Terminal"
        aria-pressed={terminalVisible || terminalEditorVisible}
        title="Open Terminal panel (Ctrl+Shift+`; drag into the file tabs to pin)"
      >
        <TerminalIcon className="h-3.5 w-3.5" />
        <span>Terminal</span>
      </button>
      <button
        type="button"
        className={`flex h-full shrink-0 items-center gap-1.5 border-r border-border-subtle px-3 text-xs transition-colors ${
          terminalEditorVisible
            ? 'border-t-2 border-t-accent bg-tab-active text-text-primary'
            : 'text-text-secondary hover:bg-hover hover:text-text-primary'
        }`}
        onClick={onOpenTerminalEditor}
        aria-label="Open Terminal in editor tab"
        aria-pressed={terminalEditorVisible}
        title="Open Terminal in editor tab"
      >
        <TerminalIcon className="h-3.5 w-3.5" />
        <span>Editor</span>
      </button>
      <div className="flex shrink-0 items-center gap-0.5 border-l border-border-subtle px-1">
        <button
          type="button"
          className="rounded p-1 text-text-muted hover:bg-hover hover:text-text-primary disabled:pointer-events-none disabled:opacity-40"
          onClick={() => void saveActiveFileAs()}
          disabled={!activeFilePath}
          aria-label="Save file as"
          title="Save As (Cmd/Ctrl+Shift+S)"
        >
          <Save className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  );
}

function TerminalEditorTab({
  terminalId,
  onSelect,
  onClose,
}: {
  terminalId?: string | null;
  onSelect?: () => void;
  onClose?: () => void;
}) {
  return (
    <div
      className="group flex h-full min-w-32 max-w-56 shrink-0 cursor-pointer items-center gap-1.5 border-r border-t-2 border-t-accent bg-tab-active px-3 text-sm text-text-primary"
      onClick={onSelect}
      role="tab"
      aria-label="Terminal editor tab"
      aria-selected="true"
      aria-current="page"
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect?.();
        }
      }}
    >
      <TerminalIcon className="h-3.5 w-3.5 shrink-0" />
      <span className="min-w-0 flex-1 truncate">
        {terminalId?.replace(/^terminal-/, 'Terminal ') || 'Terminal'}
      </span>
      <button
        type="button"
        className="rounded p-0.5 opacity-0 transition-opacity hover:bg-hover group-hover:opacity-100 group-focus-within:opacity-100"
        onClick={(event) => {
          event.stopPropagation();
          onClose?.();
        }}
        aria-label="Close terminal editor tab"
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}

function Tab({
  tab,
  isActive,
  onSelect,
  onClose,
  setRef,
  onNavigate,
}: {
  tab: OpenTab;
  isActive: boolean;
  onSelect: () => void;
  onClose: () => boolean;
  setRef: (node: HTMLDivElement | null) => void;
  onNavigate: (direction: 'next' | 'previous' | 'first' | 'last') => void;
}) {
  return (
    <div
      ref={setRef}
      className={`group flex h-full min-w-0 max-w-56 cursor-pointer items-center gap-1.5 border-r px-3 text-sm transition-colors ${
        isActive
          ? 'border-t-2 border-t-accent bg-tab-active text-text-primary'
          : 'bg-tab text-text-secondary hover:bg-hover'
      }`}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect();
          return;
        }

        if (event.key === 'ArrowRight') {
          event.preventDefault();
          onNavigate('next');
        } else if (event.key === 'ArrowLeft') {
          event.preventDefault();
          onNavigate('previous');
        } else if (event.key === 'Home') {
          event.preventDefault();
          onNavigate('first');
        } else if (event.key === 'End') {
          event.preventDefault();
          onNavigate('last');
        }
      }}
      role="tab"
      aria-selected={isActive}
      tabIndex={isActive ? 0 : -1}
    >
      <span className="flex min-w-0 items-center gap-1">
        {tab.isModified && (
          <span
            className="inline-block h-2 w-2 shrink-0 rounded-full bg-accent-text"
            aria-label="Unsaved changes"
          />
        )}
        <span className="truncate">{tab.name}</span>
      </span>
      <button
        type="button"
        className="ml-1 shrink-0 rounded p-0.5 opacity-0 transition-opacity hover:bg-hover group-hover:opacity-100 group-focus-within:opacity-100"
        onClick={(event) => {
          event.stopPropagation();
          onClose();
        }}
        aria-label={`Close ${tab.name}${tab.isModified ? ' and discard unsaved changes' : ''}`}
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}
