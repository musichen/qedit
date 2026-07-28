import {
  ChevronRight,
  Folder,
  FolderOpen,
  File,
  Clock,
  Home,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';

import { useEditor } from './EditorContext';

interface FileEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  children?: FileEntry[];
}

async function readDirectory(dirPath: string): Promise<FileEntry[]> {
  try {
    // Dynamic import — only works in Tauri context
    const { readDir } = await import('@tauri-apps/plugin-fs');
    const entries = await readDir(dirPath);

    const result: FileEntry[] = [];

    for (const entry of entries) {
      // Skip hidden files/folders
      if (entry.name.startsWith('.')) continue;
      // Skip node_modules and common large dirs
      if (
        entry.isDirectory &&
        ['node_modules', 'target', '.git', 'dist', '.turbo'].includes(
          entry.name,
        )
      )
        continue;

      result.push({
        name: entry.name,
        path: `${dirPath}/${entry.name}`,
        isDirectory: entry.isDirectory ?? false,
      });
    }

    // Sort: directories first, then alphabetically
    result.sort((a, b) => {
      if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;

      return a.name.localeCompare(b.name);
    });

    return result;
  } catch {
    // Fallback for browser dev mode — show empty tree
    return [];
  }
}

/** Resolve ~ to actual home directory */
function resolveHome(path: string): string {
  if (path.startsWith('~/')) {
    const home =
      typeof window !== 'undefined'
        ? (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
          ? // In Tauri: try common macOS home
            '/Users/' + (path.split('/')[0] === '~' ? '' : '')
          : ''
        : '';

    return home || '/home/user';
  }

  return path;
}

export function FileTree() {
  const { activeFilePath, openFile } = useEditor();
  const [rootEntries, setRootEntries] = useState<FileEntry[]>([]);
  const [rootPath, _setRootPath] = useState<string>(() =>
    typeof window !== 'undefined'
      ? ((import.meta as { env?: Record<string, string> }).env?.VITE_HOME_DIR ??
        '/Users/musichen')
      : '/Users/musichen',
  );
  const [loading, setLoading] = useState(false);

  // Load home directory on mount
  const loadRoot = useCallback(async () => {
    setLoading(true);

    try {
      const entries = await readDirectory(resolveHome(rootPath));
      setRootEntries(entries);
    } finally {
      setLoading(false);
    }
  }, [rootPath]);

  useEffect(() => {
    void loadRoot();
  }, [loadRoot]);

  return (
    <div className="flex h-full flex-col border-r bg-muted/30">
      {/* Header */}
      <div className="flex items-center gap-2 border-b px-3 py-2">
        <Home className="h-3.5 w-3.5 text-muted-foreground" />
        <span className="text-xs font-medium">Explorer</span>
      </div>

      {/* Recent Files */}
      <div className="border-b px-2 py-1.5">
        <div className="flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          <Clock className="h-3 w-3" />
          Recent
        </div>
        <div className="mt-1">
          {/* Recent files would be populated from DB — placeholder */}
          <p className="px-2 text-xs text-muted-foreground/60">
            No recent files
          </p>
        </div>
      </div>

      {/* Directory Tree */}
      <div className="flex-1 overflow-auto">
        {loading ? (
          <div className="px-3 py-2 text-xs text-muted-foreground">
            Loading...
          </div>
        ) : (
          <div className="py-1">
            {rootEntries.map((entry) => (
              <TreeNode
                key={entry.path}
                entry={entry}
                depth={0}
                activeFilePath={activeFilePath}
                onOpenFile={openFile}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function TreeNode({
  entry,
  depth,
  activeFilePath,
  onOpenFile,
}: {
  entry: FileEntry;
  depth: number;
  activeFilePath: string | null;
  onOpenFile: (path: string, name: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [children, setChildren] = useState<FileEntry[] | null>(null);
  const [loadingChildren, setLoadingChildren] = useState(false);

  const handleToggle = useCallback(async () => {
    if (!entry.isDirectory) {
      onOpenFile(entry.path, entry.name);

      return;
    }

    if (!expanded && children === null) {
      setLoadingChildren(true);

      try {
        const entries = await readDirectory(entry.path);
        setChildren(entries);
      } finally {
        setLoadingChildren(false);
      }
    }
    setExpanded(!expanded);
  }, [entry, expanded, children, onOpenFile]);

  const isActive = activeFilePath === entry.path;
  const paddingLeft = 8 + depth * 16;

  return (
    <div>
      <div
        className={`flex cursor-pointer items-center gap-1 px-2 py-0.5 text-xs transition-colors hover:bg-muted ${
          isActive ? 'bg-muted text-foreground' : 'text-muted-foreground'
        }`}
        style={{ paddingLeft: `${paddingLeft}px` }}
        onClick={() => void handleToggle()}
        onKeyDown={(e) => {
          if (e.key === 'Enter') void handleToggle();
        }}
        role="treeitem"
        aria-expanded={entry.isDirectory ? expanded : undefined}
        tabIndex={0}
      >
        {entry.isDirectory ? (
          <>
            <ChevronRight
              className={`h-3 w-3 shrink-0 transition-transform ${
                expanded ? 'rotate-90' : ''
              }`}
            />
            {expanded ? (
              <FolderOpen className="h-3.5 w-3.5 shrink-0 text-amber-500" />
            ) : (
              <Folder className="h-3.5 w-3.5 shrink-0 text-amber-500" />
            )}
          </>
        ) : (
          <>
            <span className="w-3 shrink-0" />
            <File className="h-3.5 w-3.5 shrink-0 text-blue-500" />
          </>
        )}
        <span className="truncate">{entry.name}</span>
      </div>
      {expanded && entry.isDirectory && (
        <div>
          {loadingChildren ? (
            <div
              className="px-2 py-0.5 text-xs text-muted-foreground"
              style={{ paddingLeft: `${paddingLeft + 16}px` }}
            >
              Loading...
            </div>
          ) : (
            children?.map((child) => (
              <TreeNode
                key={child.path}
                entry={child}
                depth={depth + 1}
                activeFilePath={activeFilePath}
                onOpenFile={onOpenFile}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}
