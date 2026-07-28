import { File, Folder } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';

import { useWorkspace } from './WorkspaceContext';

import { dirnameFromPath } from '#/lib/workspace-bridge';

export function QuickOpen({ onClose }: { onClose: () => void }) {
  const { knownFiles, recentFiles, openWorkspaceFile } = useWorkspace();
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState('');

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

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

  const filtered = query
    ? knownFiles.filter((entry) =>
        entry.name.toLowerCase().includes(query.toLowerCase()),
      )
    : recentFiles.slice(0, 15).map((refile) => ({
        name: refile.displayName,
        path: refile.filePath,
        isDirectory: false,
        isFile: true,
      }));

  const handleSelect = useCallback(
    (path: string, name: string) => {
      void openWorkspaceFile(path, name);
      onClose();
    },
    [openWorkspaceFile, onClose],
  );

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/40 pt-[15vh]"
      onClick={onClose}
      role="dialog"
      aria-label="Quick open"
    >
      <div
        className="w-full max-w-lg rounded-lg border bg-background shadow-xl"
        onClick={(event) => event.stopPropagation()}
        role="listbox"
      >
        <div className="border-b px-3 py-2">
          <input
            ref={inputRef}
            type="text"
            className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground"
            placeholder="Search files by name..."
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') onClose();

              if (event.key === 'Enter' && filtered.length > 0) {
                const entry = filtered[0];

                if (entry) handleSelect(entry.path, entry.name);
              }
            }}
            aria-label="Search files"
          />
        </div>
        <div className="max-h-64 overflow-auto p-1">
          {filtered.length === 0 ? (
            <p className="px-2 py-4 text-center text-xs text-muted-foreground">
              {query ? 'No matching files' : 'No files open in workspace'}
            </p>
          ) : (
            filtered.map((entry) => (
              <button
                key={entry.path}
                type="button"
                className="flex w-full min-w-0 items-center gap-2 rounded px-2 py-1.5 text-left text-sm hover:bg-muted"
                onClick={() => handleSelect(entry.path, entry.name)}
                title={entry.path}
                role="option"
                aria-selected={false}
              >
                {entry.isDirectory ? (
                  <Folder className="h-3.5 w-3.5 shrink-0 text-amber-500" />
                ) : (
                  <File className="h-3.5 w-3.5 shrink-0 text-blue-500" />
                )}
                <span className="truncate">{entry.name}</span>
                <span className="ml-auto shrink-0 truncate text-[11px] text-muted-foreground">
                  {dirnameFromPath(entry.path)}
                </span>
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
