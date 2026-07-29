import { db } from '@qedit/db';
import type { recentFiles, recentProjects } from '@qedit/db/schema';
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
} from 'react';
import type { ReactNode } from 'react';

import { useEditor } from './EditorContext';

import {
  basenameFromPath,
  errorMessage,
  getNativeHomeDirectory,
  isPathWithinHome,
  openNativeFile,
  openNativeFolder,
  readWorkspaceDirectory,
  type WorkspaceEntry,
  WorkspaceBridgeError,
} from '#/lib/workspace-bridge';

export type RecentFile = typeof recentFiles.$inferSelect;
export type RecentProject = typeof recentProjects.$inferSelect;

interface WorkspaceContextValue {
  workspaceRoot: string | null;
  rootEntries: WorkspaceEntry[];
  knownFiles: WorkspaceEntry[];
  recentFiles: RecentFile[];
  recentProjects: RecentProject[];
  loading: boolean;
  error: string | null;
  openFileDialog: () => Promise<void>;
  openFolderDialog: () => Promise<void>;
  /** The single seam every file open goes through, so Recent stays in sync. */
  openWorkspaceFile: (filePath: string, displayName?: string) => Promise<void>;
  openRecentProject: (projectPath: string) => Promise<void>;
  registerEntries: (entries: WorkspaceEntry[], sourceRoot: string) => void;
  retryWorkspace: () => void;
}

const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

async function assertSafePath(path: string): Promise<void> {
  const home = await getNativeHomeDirectory();

  if (!home || !isPathWithinHome(path, home)) {
    throw new WorkspaceBridgeError(
      'For safety, qedit can only open files and folders in your home directory.',
    );
  }
}

export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const { openFile, closeAllTabs } = useEditor();
  const [workspaceRoot, setWorkspaceRoot] = useState<string | null>(null);
  const [rootEntries, setRootEntries] = useState<WorkspaceEntry[]>([]);
  const [knownFiles, setKnownFiles] = useState<WorkspaceEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [recentFiles, setRecentFiles] = useState<RecentFile[]>(() =>
    db.getRecentFiles(10),
  );
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(() =>
    db.getRecentProjects(10),
  );
  const loadGeneration = useRef(0);
  const workspaceRootRef = useRef<string | null>(null);
  workspaceRootRef.current = workspaceRoot;

  const refreshRecent = useCallback(() => {
    setRecentFiles(db.getRecentFiles(10));
    setRecentProjects(db.getRecentProjects(10));
  }, []);

  const loadWorkspace = useCallback(
    (root: string, initialEntries?: WorkspaceEntry[]) => {
      const generation = ++loadGeneration.current;
      setWorkspaceRoot(root);
      setRootEntries([]);
      setKnownFiles([]);
      setLoading(true);
      setError(null);

      void (
        initialEntries
          ? Promise.resolve(initialEntries)
          : readWorkspaceDirectory(root)
      ).then(
        (entries) => {
          if (generation !== loadGeneration.current) return;

          setRootEntries(entries);
          setKnownFiles(entries.filter((entry) => entry.isFile));
          setLoading(false);
        },
        (cause: unknown) => {
          if (generation !== loadGeneration.current) return;

          setLoading(false);
          setError(errorMessage(cause));
        },
      );
    },
    [],
  );

  const openFileDialog = useCallback(async () => {
    setError(null);

    try {
      const selected = await openNativeFile();

      if (!selected) return;

      openFile(selected, basenameFromPath(selected));
      refreshRecent();
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }, [openFile, refreshRecent]);

  const openFolderDialog = useCallback(async () => {
    setError(null);

    try {
      const selected = await openNativeFolder();

      if (!selected) return;

      // Validate the new workspace before closing the current tabs. A stale,
      // unreadable, or rejected folder must never destroy the user's buffers.
      const entries = await readWorkspaceDirectory(selected);
      if (!closeAllTabs()) return;

      loadWorkspace(selected, entries);
      db.addRecentProject(selected, basenameFromPath(selected));
      refreshRecent();
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }, [closeAllTabs, loadWorkspace, refreshRecent]);

  const openWorkspaceFile = useCallback(
    async (filePath: string, displayName?: string) => {
      setError(null);

      try {
        await assertSafePath(filePath);
        openFile(filePath, displayName ?? basenameFromPath(filePath));
        refreshRecent();
      } catch (cause) {
        setError(errorMessage(cause));
      }
    },
    [openFile, refreshRecent],
  );

  const openRecentProject = useCallback(
    async (projectPath: string) => {
      setError(null);

      try {
        await assertSafePath(projectPath);
        // Revalidate a recent project before closing the current tabs because it
        // may have been moved or become unreadable since it was recorded.
        const entries = await readWorkspaceDirectory(projectPath);
        if (!closeAllTabs()) return;

        loadWorkspace(projectPath, entries);
        db.addRecentProject(projectPath, basenameFromPath(projectPath));
        refreshRecent();
      } catch (cause) {
        setError(errorMessage(cause));
      }
    },
    [closeAllTabs, loadWorkspace, refreshRecent],
  );

  const registerEntries = useCallback(
    (entries: WorkspaceEntry[], sourceRoot: string) => {
      setKnownFiles((previous) => {
        // A folder expansion can finish after the user switches workspaces.
        // Ignore that result so Quick Open never mixes two projects.
        if (
          !workspaceRootRef.current ||
          sourceRoot !== workspaceRootRef.current
        )
          return previous;

        const next = new Map(previous.map((entry) => [entry.path, entry]));

        for (const entry of entries) {
          if (entry.isFile) next.set(entry.path, entry);
        }

        return [...next.values()];
      });
    },
    [],
  );

  const retryWorkspace = useCallback(() => {
    if (workspaceRoot) loadWorkspace(workspaceRoot);
  }, [loadWorkspace, workspaceRoot]);

  const value = useMemo<WorkspaceContextValue>(
    () => ({
      workspaceRoot,
      rootEntries,
      knownFiles,
      recentFiles,
      recentProjects,
      loading,
      error,
      openFileDialog,
      openFolderDialog,
      openWorkspaceFile,
      openRecentProject,
      registerEntries,
      retryWorkspace,
    }),
    [
      workspaceRoot,
      rootEntries,
      knownFiles,
      recentFiles,
      recentProjects,
      loading,
      error,
      openFileDialog,
      openFolderDialog,
      openWorkspaceFile,
      openRecentProject,
      registerEntries,
      retryWorkspace,
    ],
  );

  return (
    <WorkspaceContext.Provider value={value}>
      {children}
    </WorkspaceContext.Provider>
  );
}

export function useWorkspace(): WorkspaceContextValue {
  const context = useContext(WorkspaceContext);

  if (!context) {
    throw new Error('useWorkspace must be used within a WorkspaceProvider');
  }

  return context;
}
