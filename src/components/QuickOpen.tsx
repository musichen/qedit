import { File } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { useWorkspace } from './WorkspaceContext';

import { dirnameFromPath } from '#/lib/workspace-bridge';

/** Quick open only ever lists files: directories are not openable buffers. */
interface QuickEntry {
  name: string;
  path: string;
}

export function QuickOpen({ onClose }: { onClose: () => void }) {
  const {
    knownFiles,
    recentFiles,
    openWorkspaceFile,
    discoverWorkspaceFiles,
    workspaceRoot,
  } = useWorkspace();
  const inputRef = useRef<HTMLInputElement>(null);
  const optionRefs = useRef(new Map<string, HTMLButtonElement>());
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [discovering, setDiscovering] = useState(false);

  const entries = useMemo<QuickEntry[]>(() => {
    const byPath = new Map<string, QuickEntry>();

    for (const entry of knownFiles) {
      byPath.set(entry.path, { name: entry.name, path: entry.path });
    }
    for (const file of recentFiles) {
      if (!byPath.has(file.filePath)) {
        byPath.set(file.filePath, {
          name: file.displayName,
          path: file.filePath,
        });
      }
    }

    return [...byPath.values()];
  }, [knownFiles, recentFiles]);

  const filtered = useMemo(() => {
    const normalized = query.trim().toLowerCase();

    if (!normalized) return entries.slice(0, 30);

    return entries
      .filter(
        (entry) =>
          entry.name.toLowerCase().includes(normalized) ||
          entry.path.toLowerCase().includes(normalized),
      )
      .slice(0, 30);
  }, [entries, query]);

  useEffect(() => {
    inputRef.current?.focus();
    if (!workspaceRoot) return;

    setDiscovering(true);
    void discoverWorkspaceFiles().finally(() => setDiscovering(false));
  }, [discoverWorkspaceFiles, workspaceRoot]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useEffect(() => {
    const selected = filtered[selectedIndex];

    if (selected) {
      optionRefs.current.get(selected.path)?.scrollIntoView?.({
        block: 'nearest',
      });
    }
  }, [filtered, selectedIndex]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const handleSelect = useCallback(
    (path: string, name: string) => {
      void openWorkspaceFile(path, name);
      onClose();
    },
    [openWorkspaceFile, onClose],
  );

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-overlay pt-[15vh]"
      onClick={onClose}
      role="dialog"
      aria-label="Search files"
    >
      <div
        className="w-full max-w-lg rounded border border-border-default bg-command-palette shadow-xl"
        onClick={(event) => event.stopPropagation()}
        role="listbox"
        aria-label="Files"
      >
        <div className="border-b px-3 py-2">
          <div className="mb-1 text-xs font-semibold text-text-secondary">
            Search files
          </div>
          <input
            ref={inputRef}
            type="text"
            className="w-full bg-transparent text-sm text-text-primary outline-none placeholder:text-text-muted"
            placeholder="Search files by name or path..."
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') {
                event.preventDefault();
                onClose();
                return;
              }

              if (event.key === 'ArrowDown' && filtered.length > 0) {
                event.preventDefault();
                setSelectedIndex((index) =>
                  Math.min(index + 1, filtered.length - 1),
                );
                return;
              }

              if (event.key === 'ArrowUp' && filtered.length > 0) {
                event.preventDefault();
                setSelectedIndex((index) => Math.max(index - 1, 0));
                return;
              }

              if (event.key === 'Home' && filtered.length > 0) {
                event.preventDefault();
                setSelectedIndex(0);
                return;
              }

              if (event.key === 'End' && filtered.length > 0) {
                event.preventDefault();
                setSelectedIndex(filtered.length - 1);
                return;
              }

              if (event.key === 'Enter' && filtered.length > 0) {
                event.preventDefault();
                const entry = filtered[selectedIndex] ?? filtered[0];

                if (entry) handleSelect(entry.path, entry.name);
              }
            }}
            aria-label="Search files"
            aria-controls="qedit-quick-open-options"
            aria-activedescendant={
              filtered[selectedIndex]
                ? `quick-open-${encodeURIComponent(filtered[selectedIndex].path)}`
                : undefined
            }
          />
        </div>
        <div
          id="qedit-quick-open-options"
          className="max-h-64 overflow-auto p-1"
        >
          {discovering ? (
            <p className="px-2 py-4 text-center text-xs text-text-secondary">
              Searching workspace...
            </p>
          ) : filtered.length === 0 ? (
            <p className="px-2 py-4 text-center text-xs text-text-secondary">
              {query ? 'No matching files' : 'No recent or workspace files'}
            </p>
          ) : (
            filtered.map((entry, index) => (
              <button
                key={entry.path}
                ref={(node) => {
                  if (node) optionRefs.current.set(entry.path, node);
                  else optionRefs.current.delete(entry.path);
                }}
                type="button"
                className={`flex w-full min-w-0 items-center gap-2 rounded px-2 py-1.5 text-left text-sm ${
                  index === selectedIndex
                    ? 'bg-hover text-text-primary'
                    : 'hover:bg-hover'
                }`}
                onMouseEnter={() => setSelectedIndex(index)}
                onClick={() => handleSelect(entry.path, entry.name)}
                title={entry.path}
                role="option"
                aria-selected={index === selectedIndex}
                id={`quick-open-${encodeURIComponent(entry.path)}`}
              >
                <File className="h-3.5 w-3.5 shrink-0 text-syntax-variable" />
                <span className="min-w-0 flex-1 truncate">{entry.name}</span>
                <span className="ml-auto max-w-[55%] min-w-0 truncate text-[11px] text-text-muted">
                  {dirnameFromPath(entry.path)}
                </span>
              </button>
            ))
          )}
        </div>
        <div className="border-t border-border-default px-3 py-1.5 text-[11px] text-text-muted">
          <kbd className="rounded border border-border-subtle bg-hover px-1">
            ↑↓
          </kbd>{' '}
          navigate
          <span className="mx-2">·</span>
          <kbd className="rounded border border-border-subtle bg-hover px-1">
            Enter
          </kbd>{' '}
          open
          <span className="mx-2">·</span>
          <kbd className="rounded border border-border-subtle bg-hover px-1">
            Esc
          </kbd>{' '}
          close
        </div>
      </div>
    </div>
  );
}
