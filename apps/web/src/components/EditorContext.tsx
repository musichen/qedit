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

export interface CursorPosition {
  line: number;
  column: number;
}

export interface OpenTab {
  path: string;
  name: string;
  isModified: boolean;
}

interface EditorContextValue {
  openTabs: OpenTab[];
  activeFilePath: string | null;
  fileContents: Map<string, string>;
  fileErrors: Map<string, string>;
  cursorPosition: CursorPosition;
  indentation: number;
  language: string;
  saving: boolean;
  saveError: string | null;
  openFile: (path: string, name: string, language?: string) => void;
  closeTab: (path: string) => void;
  setActiveFile: (path: string) => void;
  setCursorPosition: (pos: CursorPosition) => void;
  markModified: (path: string, modified: boolean) => void;
  setLanguage: (language: string) => void;
  setSaving: (saving: boolean) => void;
  updateFileContent: (path: string, content: string) => void;
  saveActiveFile: () => Promise<void>;
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

const errorMessage = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

async function readFileContent(filePath: string): Promise<string> {
  const { readTextFile } = await import('@tauri-apps/plugin-fs');

  return await readTextFile(filePath);
}

async function writeFileContent(
  filePath: string,
  content: string,
): Promise<void> {
  try {
    const { writeTextFile } = await import('@tauri-apps/plugin-fs');
    await writeTextFile(filePath, content);
  } catch (cause) {
    throw new Error(`Could not write ${filePath}: ${errorMessage(cause)}`, {
      cause,
    });
  }
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
  const [fileErrors, setFileErrors] = useState<Map<string, string>>(new Map());
  const [cursorPosition, setCursorPosition] = useState<CursorPosition>({
    line: 1,
    column: 1,
  });
  const [language, setLanguage] = useState('typescript');
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  // Track which paths have already been loaded to avoid double-fetches
  const loadedRef = useRef<Set<string>>(new Set());

  const loadFile = useCallback((path: string) => {
    if (loadedRef.current.has(path)) return;

    loadedRef.current.add(path);

    void readFileContent(path).then(
      (content) => {
        setFileContents((prev) => new Map(prev).set(path, content));
        setFileErrors((prev) => withoutKey(prev, path));
      },
      (error: unknown) => {
        // Keep no content cached: an unread buffer must never be savable.
        loadedRef.current.delete(path);
        setFileContents((prev) => withoutKey(prev, path));
        setFileErrors((prev) => new Map(prev).set(path, errorMessage(error)));
      },
    );
  }, []);

  const openFile = useCallback(
    (path: string, name: string, lang?: string) => {
      const resolvedLang = lang ?? languageFromPath(path);

      // Add tab if new
      setOpenTabs((prev) => {
        const existing = prev.find((t) => t.path === path);

        if (existing) return prev;

        return [...prev, { path, name, isModified: false }];
      });

      setActiveFilePath(path);
      setLanguage(resolvedLang);

      // Track in recent files
      db.addRecentFile(path, name);

      loadFile(path);
    },
    [loadFile],
  );

  const closeTab = useCallback(
    (path: string) => {
      const closedIdx = openTabs.findIndex((t) => t.path === path);
      const remaining = openTabs.filter((t) => t.path !== path);

      setOpenTabs(remaining);

      // Drop the cached buffer so reopening re-reads from disk
      loadedRef.current.delete(path);
      setFileContents((prev) => withoutKey(prev, path));
      setFileErrors((prev) => withoutKey(prev, path));

      if (path !== activeFilePath) return;

      if (remaining.length === 0) {
        setActiveFilePath(null);

        return;
      }

      const nextIdx = Math.max(0, Math.min(closedIdx, remaining.length - 1));
      const nextActive = remaining[nextIdx];

      if (nextActive) {
        setActiveFilePath(nextActive.path);
        setLanguage(languageFromPath(nextActive.path));
      }
    },
    [activeFilePath, openTabs],
  );

  const setActiveFile = useCallback(
    (path: string) => {
      setActiveFilePath(path);
      setLanguage(languageFromPath(path));

      loadFile(path);
    },
    [loadFile],
  );

  const markModified = useCallback((path: string, modified: boolean) => {
    setOpenTabs((prev) =>
      prev.map((t) => (t.path === path ? { ...t, isModified: modified } : t)),
    );
  }, []);

  const updateFileContent = useCallback((path: string, content: string) => {
    setFileContents((prev) => new Map(prev).set(path, content));
  }, []);

  const saveActiveFile = useCallback(async () => {
    if (!activeFilePath) return;

    if (fileErrors.has(activeFilePath)) {
      setSaveError(
        `Refusing to save ${activeFilePath}: the file was never read successfully`,
      );

      return;
    }

    const content = fileContents.get(activeFilePath);

    if (content === undefined) return;

    setSaving(true);
    setSaveError(null);

    try {
      await writeFileContent(activeFilePath, content);
      markModified(activeFilePath, false);
    } catch (error) {
      // Keep the modified state so the buffer is not presumed persisted
      setSaveError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  }, [activeFilePath, fileContents, fileErrors, markModified]);

  const value = useMemo<EditorContextValue>(
    () => ({
      openTabs,
      activeFilePath,
      fileContents,
      fileErrors,
      cursorPosition,
      indentation: 2,
      language,
      saving,
      saveError,
      openFile,
      closeTab,
      setActiveFile,
      setCursorPosition,
      markModified,
      setLanguage,
      setSaving,
      updateFileContent,
      saveActiveFile,
    }),
    [
      openTabs,
      activeFilePath,
      fileContents,
      fileErrors,
      cursorPosition,
      language,
      saving,
      saveError,
      openFile,
      closeTab,
      setActiveFile,
      markModified,
      updateFileContent,
      saveActiveFile,
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
