import { Clock3, FilePlus2, FolderPlus, Sparkles } from 'lucide-react';

import { useWorkspace } from './WorkspaceContext';

import { dirnameFromPath } from '#/lib/workspace-bridge';

export function EmptyState() {
  const {
    workspaceRoot,
    recentFiles,
    recentProjects,
    openFileDialog,
    openFolderDialog,
    openWorkspaceFile,
    openRecentProject,
  } = useWorkspace();

  return (
    <div className="flex h-full flex-col items-center justify-center overflow-auto bg-background p-8 text-center">
      <div className="flex max-w-lg flex-col items-center">
        <div className="mb-4 rounded-2xl border bg-muted/40 p-3 text-primary">
          <Sparkles className="h-7 w-7" />
        </div>
        <h1 className="text-lg font-semibold">
          {workspaceRoot ? 'Your workspace is ready' : 'Welcome to qedit'}
        </h1>
        <p className="mt-2 max-w-md text-sm text-muted-foreground">
          {workspaceRoot
            ? 'Choose a file from the Explorer to start editing.'
            : 'Open a file or folder to start editing with a focused, lightweight workspace.'}
        </p>

        <div className="mt-6 flex flex-wrap justify-center gap-2">
          <button
            type="button"
            className="inline-flex items-center gap-2 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90"
            onClick={() => void openFileDialog()}
          >
            <FilePlus2 className="h-4 w-4" />
            Open File
          </button>
          <button
            type="button"
            className="inline-flex items-center gap-2 rounded-md border px-3 py-2 text-sm font-medium hover:bg-muted"
            onClick={() => void openFolderDialog()}
          >
            <FolderPlus className="h-4 w-4" />
            Open Folder
          </button>
        </div>

        <p className="mt-4 text-xs text-muted-foreground">
          <kbd className="rounded border bg-muted px-1.5 py-0.5 font-mono">
            ⌘/Ctrl O
          </kbd>{' '}
          file
          <span className="mx-2 text-muted-foreground/50">·</span>
          <kbd className="rounded border bg-muted px-1.5 py-0.5 font-mono">
            ⌘/Ctrl Shift O
          </kbd>{' '}
          folder
          <span className="mx-2 text-muted-foreground/50">·</span>
          <kbd className="rounded border bg-muted px-1.5 py-0.5 font-mono">
            ⌘/Ctrl P
          </kbd>{' '}
          quick open
        </p>
      </div>

      {(recentProjects.length > 0 || recentFiles.length > 0) && (
        <div className="mt-8 w-full max-w-md text-left">
          <div className="mb-2 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            <Clock3 className="h-3.5 w-3.5" />
            Recent
          </div>
          <div className="grid gap-1">
            {recentProjects.slice(0, 3).map((project) => (
              <button
                type="button"
                key={project.projectPath}
                className="flex items-center justify-between rounded-md border px-3 py-2 text-left text-sm hover:bg-muted"
                onClick={() => void openRecentProject(project.projectPath)}
              >
                <span className="truncate font-medium">
                  {project.displayName}
                </span>
                <span className="ml-3 truncate text-xs text-muted-foreground">
                  {project.projectPath}
                </span>
              </button>
            ))}
            {recentFiles.slice(0, 5).map((file) => (
              <button
                type="button"
                key={file.filePath}
                className="flex items-center justify-between rounded-md border px-3 py-2 text-left text-sm hover:bg-muted"
                onClick={() =>
                  void openWorkspaceFile(file.filePath, file.displayName)
                }
                title={file.filePath}
              >
                <span className="truncate font-medium">{file.displayName}</span>
                <span className="ml-3 truncate text-xs text-muted-foreground">
                  {dirnameFromPath(file.filePath)}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
