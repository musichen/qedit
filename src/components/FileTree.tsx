import {
  AlertCircle,
  ChevronRight,
  Clock,
  File,
  FilePlus,
  FilePlus2,
  Folder,
  FolderOpen,
  FolderPlus,
  Home,
  Pencil,
  RefreshCw,
  Search,
  Trash2,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';

import { useEditor } from './EditorContext';
import { useWorkspace } from './WorkspaceContext';

import type { WorkspaceEntry } from '#/lib/workspace-bridge';
import {
  basenameFromPath,
  readWorkspaceDirectory,
} from '#/lib/workspace-bridge';

export function FileTree() {
  const { activeFilePath } = useEditor();
  const {
    workspaceRoot,
    rootEntries,
    recentFiles,
    recentProjects,
    loading,
    error,
    openFileDialog,
    openFolderDialog,
    createFile,
    renameFile,
    deleteFile,
    refreshWorkspace,
    workspaceVersion,
    openWorkspaceFile,
    openRecentProject,
    registerEntries,
    retryWorkspace,
  } = useWorkspace();

  const handleOpenFile = useCallback(
    (filePath: string, displayName: string) => {
      void openWorkspaceFile(filePath, displayName);
    },
    [openWorkspaceFile],
  );

  return (
    <aside className="flex h-full min-h-0 flex-col border-r bg-muted/30">
      <div className="flex items-center justify-between border-b px-3 py-2">
        <div className="flex min-w-0 items-center gap-2">
          <Home className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
          <span className="truncate text-xs font-medium">Explorer</span>
        </div>
        <div className="flex items-center gap-0.5">
          <button
            type="button"
            className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={() => void openFileDialog()}
            aria-label="Open file"
            title="Open File (Cmd/Ctrl+O)"
          >
            <FilePlus2 className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={() => void createFile()}
            aria-label="New file"
            title="New File in Workspace"
          >
            <FilePlus className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={() => window.dispatchEvent(new Event('qedit:quick-open'))}
            aria-label="Search files"
            title="Search Files (Cmd/Ctrl+P)"
          >
            <Search className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={() => void openFolderDialog()}
            aria-label="Open folder"
            title="Open Folder or Project (Cmd/Ctrl+Shift+O)"
          >
            <FolderPlus className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        {error && (
          <div className="m-2 rounded border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive">
            <div className="flex items-start gap-1.5">
              <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <span className="min-w-0 break-words">{error}</span>
            </div>
            {workspaceRoot && (
              <button
                type="button"
                className="mt-2 inline-flex items-center gap-1 text-[11px] font-medium underline underline-offset-2"
                onClick={retryWorkspace}
              >
                <RefreshCw className="h-3 w-3" />
                Retry
              </button>
            )}
          </div>
        )}

        <section className="border-b px-2 py-2">
          <div className="mb-1 flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            <Clock className="h-3 w-3" />
            Recent
          </div>
          {recentProjects.length === 0 && recentFiles.length === 0 ? (
            <p className="px-1 text-xs text-muted-foreground/60">
              No recent items
            </p>
          ) : (
            <div className="space-y-0.5">
              {recentProjects.slice(0, 5).map((project) => (
                <button
                  key={project.projectPath}
                  type="button"
                  className="flex w-full min-w-0 items-center gap-1.5 rounded px-1.5 py-1 text-left text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
                  onClick={() => void openRecentProject(project.projectPath)}
                  title={project.projectPath}
                >
                  <Folder className="h-3 w-3 shrink-0 text-amber-500" />
                  <span className="truncate">{project.displayName}</span>
                </button>
              ))}
              {recentFiles.slice(0, 5).map((file) => (
                <button
                  key={file.filePath}
                  type="button"
                  className={`flex w-full min-w-0 items-center gap-1.5 rounded px-1.5 py-1 text-left text-xs hover:bg-muted ${
                    activeFilePath === file.filePath
                      ? 'bg-muted text-foreground'
                      : 'text-muted-foreground'
                  }`}
                  onClick={() =>
                    handleOpenFile(file.filePath, file.displayName)
                  }
                  title={file.filePath}
                >
                  <File className="h-3 w-3 shrink-0 text-blue-500" />
                  <span className="truncate">{file.displayName}</span>
                </button>
              ))}
            </div>
          )}
        </section>

        <section className="min-h-0">
          <div className="flex items-center gap-1 border-b px-3 py-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            {workspaceRoot ? (
              <>
                <FolderOpen className="h-3 w-3 text-amber-500" />
                <span className="truncate" title={workspaceRoot}>
                  {basenameFromPath(workspaceRoot)}
                </span>
                <button
                  type="button"
                  className="ml-auto rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
                  onClick={() => void refreshWorkspace()}
                  aria-label="Refresh workspace"
                  title="Refresh Workspace"
                >
                  <RefreshCw className="h-3 w-3" />
                </button>
              </>
            ) : (
              <span>Workspace</span>
            )}
          </div>
          {!workspaceRoot ? (
            <div className="space-y-2 px-3 py-3 text-xs text-muted-foreground">
              <p>Open a folder to browse its files.</p>
              <button
                type="button"
                className="inline-flex items-center gap-1.5 rounded border px-2 py-1.5 font-medium text-foreground hover:bg-muted"
                onClick={() => void openFolderDialog()}
              >
                <FolderPlus className="h-3.5 w-3.5" />
                Open Folder
              </button>
            </div>
          ) : loading ? (
            <p className="px-3 py-3 text-xs text-muted-foreground">
              Loading workspace...
            </p>
          ) : rootEntries.length === 0 && !error ? (
            <p className="px-3 py-3 text-xs text-muted-foreground">
              No visible files in this folder.
            </p>
          ) : (
            <div
              className="py-1"
              role="tree"
              aria-label={`${basenameFromPath(workspaceRoot)} files`}
            >
              <div key={workspaceRoot}>
                {rootEntries.map((entry) => (
                  <TreeNode
                    key={entry.path}
                    entry={entry}
                    depth={0}
                    activeFilePath={activeFilePath}
                    onOpenFile={handleOpenFile}
                    onEntriesLoaded={(entries) =>
                      registerEntries(entries, workspaceRoot)
                    }
                    refreshVersion={workspaceVersion}
                    onRename={(filePath) => {
                      const currentName = basenameFromPath(filePath);
                      const nextName = window.prompt(
                        `Rename ${currentName}`,
                        currentName,
                      );
                      if (nextName !== null)
                        void renameFile(filePath, nextName);
                    }}
                    onDelete={(filePath) => {
                      const name = basenameFromPath(filePath);
                      if (
                        window.confirm(`Delete ${name}? This cannot be undone.`)
                      ) {
                        void deleteFile(filePath);
                      }
                    }}
                  />
                ))}
              </div>
            </div>
          )}
        </section>
      </div>
    </aside>
  );
}

function TreeNode({
  entry,
  depth,
  activeFilePath,
  onOpenFile,
  onEntriesLoaded,
  refreshVersion,
  onRename,
  onDelete,
}: {
  entry: WorkspaceEntry;
  depth: number;
  activeFilePath: string | null;
  onOpenFile: (path: string, name: string) => void;
  onEntriesLoaded: (entries: WorkspaceEntry[]) => void;
  refreshVersion: number;
  onRename: (path: string) => void;
  onDelete: (path: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [children, setChildren] = useState<WorkspaceEntry[] | null>(null);
  const [loadingChildren, setLoadingChildren] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  const loadChildren = useCallback(async () => {
    setLoadingChildren(true);
    setLoadError(null);

    try {
      const entries = await readWorkspaceDirectory(entry.path);
      setChildren(entries);
      onEntriesLoaded(entries);
    } catch (cause) {
      setLoadError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoadingChildren(false);
    }
  }, [entry.path, onEntriesLoaded]);

  useEffect(() => {
    if (expanded) void loadChildren();
  }, [expanded, loadChildren, refreshVersion]);

  const handleToggle = useCallback(async () => {
    if (!entry.isDirectory) {
      onOpenFile(entry.path, entry.name);

      return;
    }

    if (!expanded && children === null) await loadChildren();

    setExpanded((value) => !value);
  }, [children, entry, expanded, loadChildren, onOpenFile]);

  const isActive = activeFilePath === entry.path;
  const paddingLeft = 8 + depth * 16;

  return (
    <div>
      <div
        className={`group flex cursor-pointer items-center gap-1 px-2 py-1 text-xs transition-colors hover:bg-muted ${
          isActive ? 'bg-muted text-foreground' : 'text-muted-foreground'
        }`}
        style={{ paddingLeft: `${paddingLeft}px` }}
        onClick={() => void handleToggle()}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            void handleToggle();
          }
        }}
        role="treeitem"
        aria-expanded={entry.isDirectory ? expanded : undefined}
        aria-selected={isActive}
        tabIndex={0}
      >
        {entry.isDirectory ? (
          <ChevronRight
            className={`h-3 w-3 shrink-0 transition-transform ${
              expanded ? 'rotate-90' : ''
            }`}
          />
        ) : (
          <span className="w-3 shrink-0" />
        )}
        {entry.isDirectory ? (
          expanded ? (
            <FolderOpen className="h-3.5 w-3.5 shrink-0 text-amber-500" />
          ) : (
            <Folder className="h-3.5 w-3.5 shrink-0 text-amber-500" />
          )
        ) : (
          <File className="h-3.5 w-3.5 shrink-0 text-blue-500" />
        )}
        <span className="min-w-0 truncate">{entry.name}</span>
        {loadingChildren && (
          <span className="ml-auto text-[10px] text-muted-foreground">...</span>
        )}
        {entry.isFile && (
          <span className="ml-auto hidden items-center gap-0.5 group-hover:flex group-focus-within:flex">
            <button
              type="button"
              className="rounded p-0.5 hover:bg-muted-foreground/20"
              onClick={(event) => {
                event.stopPropagation();
                onRename(entry.path);
              }}
              aria-label={`Rename ${entry.name}`}
              title={`Rename ${entry.name}`}
            >
              <Pencil className="h-3 w-3" />
            </button>
            <button
              type="button"
              className="rounded p-0.5 text-destructive hover:bg-destructive/10"
              onClick={(event) => {
                event.stopPropagation();
                onDelete(entry.path);
              }}
              aria-label={`Delete ${entry.name}`}
              title={`Delete ${entry.name}`}
            >
              <Trash2 className="h-3 w-3" />
            </button>
          </span>
        )}
      </div>
      {expanded && entry.isDirectory && (
        <div role="group">
          {loadError ? (
            <button
              type="button"
              className="ml-8 flex items-center gap-1 px-2 py-1 text-left text-[11px] text-destructive hover:underline"
              onClick={() => void loadChildren()}
            >
              <RefreshCw className="h-3 w-3" />
              Retry folder
            </button>
          ) : children?.length === 0 ? (
            <p
              className="px-2 py-1 text-[11px] text-muted-foreground"
              style={{ paddingLeft: `${paddingLeft + 32}px` }}
            >
              Empty folder
            </p>
          ) : (
            children?.map((child) => (
              <TreeNode
                key={child.path}
                entry={child}
                depth={depth + 1}
                activeFilePath={activeFilePath}
                onOpenFile={onOpenFile}
                onEntriesLoaded={onEntriesLoaded}
                refreshVersion={refreshVersion}
                onRename={onRename}
                onDelete={onDelete}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}
