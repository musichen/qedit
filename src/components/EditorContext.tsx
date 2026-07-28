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
  hasDirtyTabs: boolean;
  openFile: (path: string, name: string, language?: string) => void;
  closeTab: (path: string) => boolean;
  closeAllTabs: () => boolean;
  setActiveFile: (path: string) => void;
  setCursorPosition: (pos: CursorPosition) => void;
  markModified: (path: string, modified: boolean) => void;
  setLanguage: (language: string) => void;
  setSaving: (saving: boolean) => void;
  updateFileContent: (path: string, content: string) => void;
  saveActiveFile: () => Promise<void>;
  saveActiveFileAs: () => Promise<void>;
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

async function readFileContent(filePath: string): Promise<string> {
  const { readTextFile } = await import('@tauri-apps/plugin-fs');

  return await readTextFile(filePath);
}

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
  } | null>(null);
  const loadedRef = useRef<Set<string>>(new Set());

  const loadFile = useCallback((path: string) => {
    if (loadedRef.current.has(path)) return;

    loadedRef.current.add(path);
    setFileStatus((prev) => new Map(prev).set(path, { kind: 'loading' }));

    void readFileContent(path).then(
      (content) => {
        setFileContents((prev) => new Map(prev).set(path, content));
        setFileStatus((prev) => new Map(prev).set(path, { kind: 'loaded' }));
      },
      (error: unknown) => {
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

  const closeTab = useCallback(
    (path: string): boolean => {
      const closedTab = openTabs.find((tab) => tab.path === path);

      if (!closedTab) return false;

      if (
        closedTab.isModified &&
        !browserConfirm(
          `${closedTab.name} has unsaved changes. Close it and discard them?`,
        )
      ) {
        return false;
      }

      const closedIdx = openTabs.findIndex((tab) => tab.path === path);
      const remaining = openTabs.filter((tab) => tab.path !== path);

      setOpenTabs(remaining);
      loadedRef.current.delete(path);
      setFileContents((prev) => withoutKey(prev, path));
      setFileStatus((prev) => withoutKey(prev, path));

      if (path !== activeFilePath) return true;

      const nextActive =
        remaining[Math.max(0, Math.min(closedIdx, remaining.length - 1))];

      setActiveFilePath(nextActive?.path ?? null);
      if (nextActive) setLanguage(languageFromPath(nextActive.path));

      return true;
    },
    [activeFilePath, openTabs],
  );

  const closeAllTabs = useCallback((): boolean => {
    const dirtyTabs = openTabs.filter((tab) => tab.isModified);

    if (
      dirtyTabs.length > 0 &&
      !browserConfirm(
        `${dirtyTabs.length} file${dirtyTabs.length === 1 ? '' : 's'} have unsaved changes. Close them and discard those changes?`,
      )
    ) {
      return false;
    }

    setOpenTabs([]);
    setActiveFilePath(null);
    loadedRef.current.clear();
    setFileContents(new Map());
    setFileStatus(new Map());

    return true;
  }, [openTabs]);

  const setActiveFile = useCallback(
    (path: string) => {
      setActiveFilePath(path);
      setLanguage(languageFromPath(path));
      setSaveFailure(null);
      loadFile(path);
    },
    [loadFile],
  );

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
      });

      return;
    }

    const content = fileContents.get(activeFilePath);

    if (content === undefined) {
      setSaveFailure({
        path: activeFilePath,
        message: `Refusing to save ${activeFilePath}: no buffer is cached for this file`,
      });

      return;
    }

    setSaving(true);
    setSaveFailure(null);

    try {
      const { writeTextFile } = await import('@tauri-apps/plugin-fs');
      await writeTextFile(activeFilePath, content);
      markModified(activeFilePath, false);
    } catch (error) {
      setSaveFailure({ path: activeFilePath, message: errorMessage(error) });
    } finally {
      setSaving(false);
    }
  }, [activeFilePath, fileContents, fileStatus, markModified]);

  const saveActiveFileAs = useCallback(async () => {
    if (!activeFilePath) return;

    const status = fileStatus.get(activeFilePath);
    const content = fileContents.get(activeFilePath);

    if (status?.kind !== 'loaded' || content === undefined) {
      setSaveFailure({
        path: activeFilePath,
        message: `Refusing to save ${activeFilePath}: the file is not ready`,
      });

      return;
    }

    let targetPath: string | null;

    try {
      targetPath = await saveNativeFile(activeFilePath);
    } catch (error) {
      setSaveFailure({ path: activeFilePath, message: errorMessage(error) });

      return;
    }

    if (!targetPath) return;

    if (
      targetPath !== activeFilePath &&
      openTabs.some((tab) => tab.path === targetPath)
    ) {
      setSaveFailure({
        path: activeFilePath,
        message: `Could not save as ${targetPath}: that file is already open`,
      });

      return;
    }

    setSaving(true);
    setSaveFailure(null);

    try {
      await writeNativeTextFile(targetPath, content);
      const targetName = basenameFromPath(targetPath);

      setOpenTabs((prev) =>
        prev.map((tab) =>
          tab.path === activeFilePath
            ? { path: targetPath, name: targetName, isModified: false }
            : tab,
        ),
      );
      setFileContents((prev) => {
        const next = new Map(prev);
        next.delete(activeFilePath);
        next.set(targetPath, content);

        return next;
      });
      setFileStatus((prev) => {
        const next = new Map(prev);
        next.delete(activeFilePath);
        next.set(targetPath, { kind: 'loaded' });

        return next;
      });
      loadedRef.current.delete(activeFilePath);
      loadedRef.current.add(targetPath);
      setActiveFilePath(targetPath);
      setLanguage(languageFromPath(targetPath));
      db.addRecentFile(targetPath, targetName);
    } catch (error) {
      setSaveFailure({ path: activeFilePath, message: errorMessage(error) });
    } finally {
      setSaving(false);
    }
  }, [activeFilePath, fileContents, fileStatus, openTabs]);

  const saveError =
    saveFailure && saveFailure.path === activeFilePath
      ? saveFailure.message
      : null;
  const hasDirtyTabs = openTabs.some((tab) => tab.isModified);

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
      hasDirtyTabs,
      openFile,
      closeTab,
      closeAllTabs,
      setActiveFile,
      setCursorPosition,
      markModified,
      setLanguage,
      setSaving,
      updateFileContent,
      saveActiveFile,
      saveActiveFileAs,
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
      hasDirtyTabs,
      openFile,
      closeTab,
      closeAllTabs,
      setActiveFile,
      markModified,
      updateFileContent,
      saveActiveFile,
      saveActiveFileAs,
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
