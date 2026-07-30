import { Save, X } from 'lucide-react';
import { useRef } from 'react';

import { useEditor } from './EditorContext';
import type { OpenTab } from './EditorContext';

export function TabBar() {
  const {
    openTabs,
    activeFilePath,
    setActiveFile,
    closeTab,
    saveActiveFileAs,
  } = useEditor();
  const tabRefs = useRef(new Map<string, HTMLDivElement>());

  const navigateTab = (index: number) => {
    const tab = openTabs[index];

    if (!tab) return;

    setActiveFile(tab.path);
    requestAnimationFrame(() => tabRefs.current.get(tab.path)?.focus());
  };

  return (
    <div className="flex h-9 items-stretch border-b bg-muted/40">
      {openTabs.length === 0 ? (
        <span className="flex min-w-0 flex-1 items-center px-4 text-xs text-muted-foreground">
          No open files
        </span>
      ) : (
        <div
          className="flex min-w-0 flex-1 items-stretch overflow-x-auto"
          role="tablist"
          aria-label="Open files"
        >
          {openTabs.map((tab, index) => (
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
          ))}
        </div>
      )}
      <div className="flex shrink-0 items-center gap-0.5 border-l px-1">
        <button
          type="button"
          className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
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
          ? 'border-t-2 border-t-primary bg-background text-foreground'
          : 'bg-muted/40 text-muted-foreground hover:bg-muted/70'
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
            className="inline-block h-2 w-2 shrink-0 rounded-full bg-accent-foreground"
            aria-label="Unsaved changes"
          />
        )}
        <span className="truncate">{tab.name}</span>
      </span>
      <button
        type="button"
        className="ml-1 shrink-0 rounded p-0.5 opacity-0 transition-opacity hover:bg-muted-foreground/20 group-hover:opacity-100 group-focus-within:opacity-100"
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
