import { db } from '@qedit/db';
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
} from 'react';
import type { ReactNode } from 'react';

import {
  basenameFromPath,
  errorMessage,
  readNativeTextFile,
  saveNativeFile,
  writeNativeTextFile,
} from '#/lib/workspace-bridge';

export interface CursorPosition {
  line: number;
  column: number;
}

export interface OpenTab {
  path: string;
  name: string;
  isModified: boolean;
  isUntitled?: boolean;
}

export type FileStatus =
  | { kind: 'loading' }
  | { kind: 'loaded' }
  | { kind: 'error'; message: string };

interface EditorContextValue {
  openTabs: OpenTab[];
  activeFilePath: string | null;
  fileContents: Map<string, string>;
  fileStatus: Map<string, FileStatus>;
  cursorPosition: CursorPosition;
  indentation: number;
  language: string;
  saving: boolean;
  saveError: string | null;
  // True when the visible failure belongs to the file the user is looking at.
  // Only such a failure may drive the active buffer's status chip; a Save As
  // that resolved against another tab is shown as a message only.
  saveErrorOwnsActiveFile: boolean;
  hasDirtyTabs: boolean;
  dirtyTabCount: number;
  openFile: (path: string, name: string, language?: string) => void;
  createUntitledFile: () => void;
  closeTab: (path: string, discardUnsaved?: boolean) => boolean;
  closeAllTabs: () => boolean;
  reopenLastClosedTab: () => void;
  reloadActiveFile: () => void;
  setActiveFile: (path: string) => void;
  renameFilePath: (oldPath: string, newPath: string) => void;
  setCursorPosition: (pos: CursorPosition) => void;
  markModified: (path: string, modified: boolean) => void;
  setLanguage: (language: string) => void;
  setSaving: (saving: boolean) => void;
  updateFileContent: (path: string, content: string) => void;
  saveActiveFile: () => Promise<void>;
  saveActiveFileAs: (defaultPath?: string) => Promise<void>;
  isActiveFileUntitled: boolean;
}

const EditorContext = createContext<EditorContextValue | null>(null);

const languageFromPath = (path: string): string => {
  const ext = path.split('.').pop()?.toLowerCase();

  const map: Record<string, string> = {
    ts: 'typescript',
    tsx: 'typescript',
    js: 'javascript',
    jsx: 'javascript',
    json: 'json',
    html: 'html',
    css: 'css',
    md: 'markdown',
    rs: 'rust',
    toml: 'toml',
    yaml: 'yaml',
    yml: 'yaml',
    py: 'python',
    sh: 'shell',
    bash: 'shell',
    zsh: 'shell',
    sql: 'sql',
    xml: 'xml',
    svg: 'xml',
    graphql: 'graphql',
    gql: 'graphql',
  };

  return map[ext ?? ''] ?? 'plaintext';
};

const browserConfirm = (message: string): boolean => {
  if (typeof window === 'undefined') return false;

  return window.confirm(message);
};

const withoutKey = <T,>(map: Map<string, T>, key: string): Map<string, T> => {
  if (!map.has(key)) return map;

  const next = new Map(map);
  next.delete(key);

  return next;
};

export function EditorProvider({ children }: { children: ReactNode }) {
  const [openTabs, setOpenTabs] = useState<OpenTab[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const [fileContents, setFileContents] = useState<Map<string, string>>(
    new Map(),
  );
  const [fileStatus, setFileStatus] = useState<Map<string, FileStatus>>(
    new Map(),
  );
  const [cursorPosition, setCursorPosition] = useState<CursorPosition>({
    line: 1,
    column: 1,
  });
  const [language, setLanguage] = useState('typescript');
  const [saving, setSaving] = useState(false);
  const [saveFailure, setSaveFailure] = useState<{
    path: string;
    message: string;
    // A 'file' failure belongs to one buffer and is hidden with it. A Save As
    // failure belongs to the operation, not to whichever tab happens to be
    // focused when the native dialog resolves, so it stays visible.
    scope: 'file' | 'operation';
  } | null>(null);
  const loadedRef = useRef<Set<string>>(new Set());
  // Keep a small, disk-backed history for Cmd/Ctrl+Shift+T. Dirty buffers are
  // deliberately not retained because closing them requires explicit
  // confirmation to discard their edits.
  const closedTabsRef = useRef<(OpenTab & { content?: string })[]>([]);
  // Request ids are monotonic across the whole provider, never per path, so a
  // path whose entry is dropped on close (or replaced by Save As) can never
  // reuse an id an in-flight read is still waiting to match.
  const nextLoadRequestRef = useRef(1);
  const nextUntitledRef = useRef(1);
  const loadRequestRef = useRef<Map<string, number>>(new Map());
  const activeFileRef = useRef<string | null>(null);
  const openTabsRef = useRef<OpenTab[]>([]);
  activeFileRef.current = activeFilePath;
  openTabsRef.current = openTabs;

  const loadFile = useCallback((path: string) => {
    if (loadedRef.current.has(path)) return;

    loadedRef.current.add(path);
    const requestId = nextLoadRequestRef.current++;
    loadRequestRef.current.set(path, requestId);
    setFileStatus((prev) => new Map(prev).set(path, { kind: 'loading' }));

    void readNativeTextFile(path).then(
      (content) => {
        if (loadRequestRef.current.get(path) !== requestId) return;

        setFileContents((prev) => new Map(prev).set(path, content));
        setFileStatus((prev) => new Map(prev).set(path, { kind: 'loaded' }));
      },
      (error: unknown) => {
        if (loadRequestRef.current.get(path) !== requestId) return;

        loadedRef.current.delete(path);
        setFileContents((prev) => withoutKey(prev, path));
        setFileStatus((prev) =>
          new Map(prev).set(path, {
            kind: 'error',
            message: errorMessage(error),
          }),
        );
      },
    );
  }, []);

  const openFile = useCallback(
    (path: string, name: string, lang?: string) => {
      const resolvedLang = lang ?? languageFromPath(path);

      setOpenTabs((prev) => {
        if (prev.some((tab) => tab.path === path)) return prev;

        return [...prev, { path, name, isModified: false }];
      });
      setActiveFilePath(path);
      setLanguage(resolvedLang);
      setSaveFailure(null);
      db.addRecentFile(path, name);
      loadFile(path);
    },
    [loadFile],
  );

  const createUntitledFile = useCallback(() => {
    const number = nextUntitledRef.current++;
    const path = `qedit://untitled-${number}`;
    const name = `Untitled-${number}`;

    setOpenTabs((prev) => [
      ...prev,
      { path, name, isModified: false, isUntitled: true },
    ]);
    setActiveFilePath(path);
    setLanguage('plaintext');
    setFileContents((prev) => new Map(prev).set(path, ''));
    setFileStatus((prev) => new Map(prev).set(path, { kind: 'loaded' }));
    setSaveFailure(null);
  }, []);

  const closeTab = useCallback(
    (path: string, discardUnsaved = false): boolean => {
      const closedTab = openTabs.find((tab) => tab.path === path);

      if (!closedTab) return false;

      if (
        closedTab.isModified &&
        !discardUnsaved &&
        !browserConfirm(
          `${closedTab.name} has unsaved changes. Close it and discard them?`,
        )
      ) {
        return false;
      }

      const closedIdx = openTabs.findIndex((tab) => tab.path === path);
      const remaining = openTabs.filter((tab) => tab.path !== path);

      closedTabsRef.current = [
        ...closedTabsRef.current,
        {
          path: closedTab.path,
          name: closedTab.name,
          isModified: false,
          isUntitled: closedTab.isUntitled,
          content: closedTab.isUntitled
            ? (fileContents.get(closedTab.path) ?? '')
            : undefined,
        },
      ].slice(-20);
      setOpenTabs(remaining);
      loadedRef.current.delete(path);
      loadRequestRef.current.delete(path);
      setFileContents((prev) => withoutKey(prev, path));
      setFileStatus((prev) => withoutKey(prev, path));

      if (path !== activeFilePath) return true;

      const nextActive =
        remaining[Math.max(0, Math.min(closedIdx, remaining.length - 1))];

      setActiveFilePath(nextActive?.path ?? null);
      if (nextActive) setLanguage(languageFromPath(nextActive.path));

      return true;
    },
    [activeFilePath, openTabs, fileContents],
  );

  const closeAllTabs = useCallback((): boolean => {
    const dirtyTabs = openTabs.filter((tab) => tab.isModified);

    if (
      dirtyTabs.length > 0 &&
      !browserConfirm(
        dirtyTabs.length === 1
          ? '1 file has unsaved changes. Close it and discard those changes?'
          : `${dirtyTabs.length} files have unsaved changes. Close them and discard those changes?`,
      )
    ) {
      return false;
    }

    closedTabsRef.current = [
      ...closedTabsRef.current,
      ...openTabs.map((tab) => ({
        ...tab,
        isModified: false,
        content: tab.isUntitled ? (fileContents.get(tab.path) ?? '') : undefined,
      })),
    ].slice(-20);
    setOpenTabs([]);
    setActiveFilePath(null);
    loadedRef.current.clear();
    loadRequestRef.current.clear();
    setFileContents(new Map());
    setFileStatus(new Map());

    return true;
  }, [openTabs, fileContents]);

  const reopenLastClosedTab = useCallback(() => {
    const lastClosed = closedTabsRef.current.pop();

    if (!lastClosed) return;

    if (lastClosed.isUntitled) {
      const { path, name, content } = lastClosed;

      setOpenTabs((prev) => {
        if (prev.some((tab) => tab.path === path)) return prev;

        return [...prev, { path, name, isModified: false, isUntitled: true }];
      });
      setActiveFilePath(path);
      setLanguage('plaintext');
      loadedRef.current.add(path);
      setFileContents((prev) => new Map(prev).set(path, content ?? ''));
      setFileStatus((prev) => new Map(prev).set(path, { kind: 'loaded' }));
      setSaveFailure(null);

      return;
    }

    openFile(lastClosed.path, lastClosed.name);
  }, [openFile]);

  const reloadActiveFile = useCallback(() => {
    if (!activeFilePath) return;

    const activeTab = openTabs.find((tab) => tab.path === activeFilePath);

    if (
      activeTab?.isModified &&
      !browserConfirm(
        `${activeTab.name} has unsaved changes. Reload it and discard them?`,
      )
    ) {
      return;
    }

    loadedRef.current.delete(activeFilePath);
    setSaveFailure(null);
    setFileContents((prev) => withoutKey(prev, activeFilePath));
    setOpenTabs((prev) =>
      prev.map((tab) =>
        tab.path === activeFilePath ? { ...tab, isModified: false } : tab,
      ),
    );
    loadFile(activeFilePath);
  }, [activeFilePath, loadFile, openTabs]);

  const setActiveFile = useCallback(
    (path: string) => {
      setActiveFilePath(path);
      setLanguage(languageFromPath(path));
      setSaveFailure(null);
      loadFile(path);
    },
    [loadFile],
  );

  const renameFilePath = useCallback((oldPath: string, newPath: string) => {
    setOpenTabs((prev) =>
      prev.map((tab) =>
        tab.path === oldPath
          ? { ...tab, path: newPath, name: basenameFromPath(newPath) }
          : tab,
      ),
    );
    setFileContents((prev) => {
      if (!prev.has(oldPath)) return prev;

      const next = new Map(prev);
      const content = next.get(oldPath);
      next.delete(oldPath);
      if (content !== undefined) next.set(newPath, content);

      return next;
    });
    setFileStatus((prev) => {
      if (!prev.has(oldPath)) return prev;

      const next = new Map(prev);
      const status = next.get(oldPath);
      next.delete(oldPath);
      if (status) next.set(newPath, status);

      return next;
    });
    if (loadedRef.current.delete(oldPath)) loadedRef.current.add(newPath);
    const requestId = loadRequestRef.current.get(oldPath);
    loadRequestRef.current.delete(oldPath);
    if (requestId !== undefined) loadRequestRef.current.set(newPath, requestId);
    setActiveFilePath((current) => (current === oldPath ? newPath : current));
    if (activeFileRef.current === oldPath)
      setLanguage(languageFromPath(newPath));
  }, []);

  const markModified = useCallback((path: string, modified: boolean) => {
    setOpenTabs((prev) =>
      prev.map((tab) =>
        tab.path === path ? { ...tab, isModified: modified } : tab,
      ),
    );
  }, []);

  const updateFileContent = useCallback(
    (path: string, content: string) => {
      if (fileStatus.get(path)?.kind !== 'loaded') return;

      setFileContents((prev) => new Map(prev).set(path, content));
    },
    [fileStatus],
  );

  const saveActiveFile = useCallback(async () => {
    if (!activeFilePath) return;

    const status = fileStatus.get(activeFilePath);

    if (status?.kind !== 'loaded') {
      setSaveFailure({
        path: activeFilePath,
        message:
          status?.kind === 'error'
            ? `Refusing to save ${activeFilePath}: the file was never read successfully`
            : `Refusing to save ${activeFilePath}: still loading`,
        scope: 'file',
      });

      return;
    }

    const content = fileContents.get(activeFilePath);

    if (content === undefined) {
      setSaveFailure({
        path: activeFilePath,
        message: `Refusing to save ${activeFilePath}: no buffer is cached for this file`,
        scope: 'file',
      });

      return;
    }

    setSaving(true);
    setSaveFailure(null);

    try {
      await writeNativeTextFile(activeFilePath, content);
      markModified(activeFilePath, false);
    } catch (error) {
      setSaveFailure({
        path: activeFilePath,
        message: errorMessage(error),
        scope: 'file',
      });
    } finally {
      setSaving(false);
    }
  }, [activeFilePath, fileContents, fileStatus, markModified]);

  const saveActiveFileAs = useCallback(
    async (defaultPath?: string) => {
      if (!activeFilePath) return;

      const status = fileStatus.get(activeFilePath);
      const content = fileContents.get(activeFilePath);

      if (status?.kind !== 'loaded' || content === undefined) {
        setSaveFailure({
          path: activeFilePath,
          message: `Refusing to save ${activeFilePath}: the file is not ready`,
          scope: 'file',
        });

        return;
      }

      const sourcePath = activeFilePath;
      let targetPath: string | null;

      try {
        const nativeDefaultPath =
          defaultPath ??
          (openTabsRef.current.find((tab) => tab.path === sourcePath)
            ?.isUntitled
            ? undefined
            : sourcePath);
        targetPath = await saveNativeFile(nativeDefaultPath);
      } catch (error) {
        setSaveFailure({
          path: sourcePath,
          message: `Could not save ${sourcePath} as a new file: ${errorMessage(error)}`,
          scope: 'operation',
        });

        return;
      }

      if (!targetPath) {
        setSaveFailure(null);

        return;
      }

      // The dialog can stay open while the user switches or closes tabs. The
      // content and the source path are captured above, so a tab switch is
      // harmless, but a closed source tab means there is no buffer left to save.
      if (!openTabsRef.current.some((tab) => tab.path === sourcePath)) {
        setSaveFailure({
          path: sourcePath,
          message: `Could not save as ${targetPath}: ${sourcePath} was closed before the save completed`,
          scope: 'operation',
        });

        return;
      }

      if (
        targetPath !== sourcePath &&
        openTabsRef.current.some((tab) => tab.path === targetPath)
      ) {
        setSaveFailure({
          path: sourcePath,
          message: `Could not save as ${targetPath}: that file is already open`,
          scope: 'operation',
        });

        return;
      }

      setSaving(true);
      setSaveFailure(null);

      try {
        await writeNativeTextFile(targetPath, content);
        window.dispatchEvent(new Event('qedit:workspace-refresh'));
        const targetName = basenameFromPath(targetPath);

        // The file exists on disk now, so it belongs in Recent regardless of what
        // happened to the tabs while the native write was in flight.
        db.addRecentFile(targetPath, targetName);

        // Keep the source tab's rename scoped to the tab that initiated Save As.
        // The user may have closed it while the write was in flight, in which
        // case there is no tab left to rename.
        const sourceStillOpen = openTabsRef.current.some(
          (tab) => tab.path === sourcePath,
        );
        if (!sourceStillOpen) return;

        setOpenTabs((prev) =>
          prev.map((tab) =>
            tab.path === sourcePath
              ? { path: targetPath, name: targetName, isModified: false }
              : tab,
          ),
        );
        setFileContents((prev) => {
          const next = new Map(prev);
          next.delete(sourcePath);
          next.set(targetPath, content);

          return next;
        });
        setFileStatus((prev) => {
          const next = new Map(prev);
          next.delete(sourcePath);
          next.set(targetPath, { kind: 'loaded' });

          return next;
        });
        loadedRef.current.delete(sourcePath);
        loadRequestRef.current.delete(sourcePath);
        loadedRef.current.add(targetPath);
        loadRequestRef.current.set(targetPath, nextLoadRequestRef.current++);
        setActiveFilePath((current) =>
          current === sourcePath ? targetPath : current,
        );
        if (activeFileRef.current === sourcePath) {
          setLanguage(languageFromPath(targetPath));
        }
      } catch (error) {
        setSaveFailure({
          path: sourcePath,
          message: `Could not save as ${targetPath}: ${errorMessage(error)}`,
          scope: 'operation',
        });
      } finally {
        setSaving(false);
      }
    },
    [activeFilePath, fileContents, fileStatus],
  );

  const activeTab = openTabs.find((tab) => tab.path === activeFilePath);
  const isActiveFileUntitled = activeTab?.isUntitled === true;

  const visibleFailure =
    saveFailure &&
    (saveFailure.scope === 'operation' || saveFailure.path === activeFilePath)
      ? saveFailure
      : null;
  const saveError = visibleFailure?.message ?? null;
  const saveErrorOwnsActiveFile =
    visibleFailure !== null && visibleFailure.path === activeFilePath;
  const dirtyTabCount = openTabs.filter((tab) => tab.isModified).length;
  const hasDirtyTabs = dirtyTabCount > 0;

  const value = useMemo<EditorContextValue>(
    () => ({
      openTabs,
      activeFilePath,
      fileContents,
      fileStatus,
      cursorPosition,
      indentation: 2,
      language,
      saving,
      saveError,
      saveErrorOwnsActiveFile,
      hasDirtyTabs,
      dirtyTabCount,
      openFile,
      createUntitledFile,
      closeTab,
      closeAllTabs,
      reopenLastClosedTab,
      reloadActiveFile,
      setActiveFile,
      renameFilePath,
      setCursorPosition,
      markModified,
      setLanguage,
      setSaving,
      updateFileContent,
      saveActiveFile,
      saveActiveFileAs,
      isActiveFileUntitled,
    }),
    [
      openTabs,
      activeFilePath,
      fileContents,
      fileStatus,
      cursorPosition,
      language,
      saving,
      saveError,
      saveErrorOwnsActiveFile,
      hasDirtyTabs,
      dirtyTabCount,
      openFile,
      createUntitledFile,
      closeTab,
      closeAllTabs,
      reopenLastClosedTab,
      reloadActiveFile,
      setActiveFile,
      renameFilePath,
      markModified,
      updateFileContent,
      saveActiveFile,
      saveActiveFileAs,
      isActiveFileUntitled,
    ],
  );

  return (
    <EditorContext.Provider value={value}>{children}</EditorContext.Provider>
  );
}

export function useEditor(): EditorContextValue {
  const ctx = useContext(EditorContext);

  if (!ctx) {
    throw new Error('useEditor must be used within an EditorProvider');
  }

  return ctx;
}
