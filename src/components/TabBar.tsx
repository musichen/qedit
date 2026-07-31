import { Save, Terminal as TerminalIcon, X } from 'lucide-react';
import { useRef, useState } from 'react';
import type { DragEvent } from 'react';

import { useEditor } from './EditorContext';
import type { OpenTab } from './EditorContext';

export function TabBar({
  onOpenTerminal,
  terminalVisible = false,
  terminalEditorVisible = false,
  onDropTerminal,
  onCloseTerminalEditor,
}: {
  onOpenTerminal?: () => void;
  terminalVisible?: boolean;
  terminalEditorVisible?: boolean;
  onDropTerminal?: () => void;
  onCloseTerminalEditor?: () => void;
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

  const handleTerminalDrop = (event: DragEvent) => {
    if (!event.dataTransfer.types.includes('text/qedit-terminal')) return;

    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    setTerminalDropActive(false);
    onDropTerminal?.();
  };

  const navigateTab = (index: number) => {
    const tab = openTabs[index];

    if (!tab) return;

    setActiveFile(tab.path);
    requestAnimationFrame(() => tabRefs.current.get(tab.path)?.focus());
  };

  return (
    <div className="flex h-tab items-stretch border-b border-border-subtle bg-tab">
      <div
        className={`flex min-w-0 flex-1 items-stretch overflow-x-auto ${
          terminalDropActive ? 'bg-accent/10 ring-1 ring-inset ring-accent' : ''
        }`}
        role="tablist"
        aria-label="Open files"
        onDragOver={(event) => {
          if (!event.dataTransfer.types.includes('text/qedit-terminal')) return;
          event.preventDefault();
          event.dataTransfer.dropEffect = 'move';
          setTerminalDropActive(true);
        }}
        onDragLeave={() => setTerminalDropActive(false)}
        onDrop={handleTerminalDrop}
      >
        {openTabs.length === 0 ? (
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
          event.dataTransfer.setData('text/qedit-terminal', 'active');
        }}
        aria-label="Open Terminal"
        aria-pressed={terminalVisible || terminalEditorVisible}
        title="Open Terminal (drag into the tab bar to pin)"
      >
        <TerminalIcon className="h-3.5 w-3.5" />
        <span>Terminal</span>
        {terminalEditorVisible && (
          <span
            role="button"
            tabIndex={0}
            className="ml-1 rounded p-0.5 hover:bg-hover"
            onClick={(event) => {
              event.stopPropagation();
              onCloseTerminalEditor?.();
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                event.stopPropagation();
                onCloseTerminalEditor?.();
              }
            }}
            aria-label="Close terminal editor tab"
          >
            ×
          </span>
        )}
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
