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
  cursorPosition: CursorPosition;
  indentation: number;
  language: string;
  saving: boolean;
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

async function readFileContent(filePath: string): Promise<string> {
  try {
    const { readTextFile } = await import('@tauri-apps/plugin-fs');

    return await readTextFile(filePath);
  } catch {
    // Fallback for browser dev mode
    return `// Could not read: ${filePath}`;
  }
}

async function writeFileContent(
  filePath: string,
  content: string,
): Promise<void> {
  try {
    const { writeTextFile } = await import('@tauri-apps/plugin-fs');
    await writeTextFile(filePath, content);
  } catch {
    throw new Error(`Could not write: ${filePath}`);
  }
}

export function EditorProvider({ children }: { children: ReactNode }) {
  const [openTabs, setOpenTabs] = useState<OpenTab[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const [fileContents, setFileContents] = useState<Map<string, string>>(
    new Map(),
  );
  const [cursorPosition, setCursorPosition] = useState<CursorPosition>({
    line: 1,
    column: 1,
  });
  const [language, setLanguage] = useState('typescript');
  const [saving, setSaving] = useState(false);
  // Track which paths have already been loaded to avoid double-fetches
  const loadedRef = useRef<Set<string>>(new Set());

  const openFile = useCallback((path: string, name: string, lang?: string) => {
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

    // Load file content if not already loaded
    if (!loadedRef.current.has(path)) {
      loadedRef.current.add(path);
      readFileContent(path)
        .then((content) => {
          setFileContents((prev) => {
            const next = new Map(prev);
            next.set(path, content);

            return next;
          });
        })
        .catch(() => {
          // File couldn't be read — silently handle
        });
    }
  }, []);

  const closeTab = useCallback(
    (path: string) => {
      setOpenTabs((prev) => prev.filter((t) => t.path !== path));

      if (path === activeFilePath) {
        setOpenTabs((prevOpenTabs) => {
          if (prevOpenTabs.length === 0) {
            setActiveFilePath(null);

            return prevOpenTabs;
          }

          // Find the tab adjacent to the closed one
          const idx = Math.min(
            prevOpenTabs.findIndex((t) => t.path === path),
            prevOpenTabs.length - 1,
          );
          const newActive = prevOpenTabs[idx];

          if (newActive) {
            setActiveFilePath(newActive.path);
            setLanguage(languageFromPath(newActive.path));
          }

          return prevOpenTabs;
        });
      }
    },
    [activeFilePath],
  );

  const setActiveFile = useCallback((path: string) => {
    setActiveFilePath(path);
    setLanguage(languageFromPath(path));

    // Load content if not yet loaded
    if (!loadedRef.current.has(path)) {
      loadedRef.current.add(path);
      readFileContent(path)
        .then((content) => {
          setFileContents((prev) => {
            const next = new Map(prev);
            next.set(path, content);

            return next;
          });
        })
        .catch(() => {
          // Silently handle
        });
    }
  }, []);

  const markModified = useCallback((path: string, modified: boolean) => {
    setOpenTabs((prev) =>
      prev.map((t) => (t.path === path ? { ...t, isModified: modified } : t)),
    );
  }, []);

  const updateFileContent = useCallback((path: string, content: string) => {
    setFileContents((prev) => {
      const next = new Map(prev);
      next.set(path, content);

      return next;
    });
  }, []);

  const saveActiveFile = useCallback(async () => {
    if (!activeFilePath) return;

    const content = fileContents.get(activeFilePath);

    if (content === undefined) return;

    setSaving(true);

    try {
      await writeFileContent(activeFilePath, content);
      markModified(activeFilePath, false);
    } catch {
      // Save failed — keep modified state
    } finally {
      setSaving(false);
    }
  }, [activeFilePath, fileContents, markModified]);

  const value = useMemo<EditorContextValue>(
    () => ({
      openTabs,
      activeFilePath,
      fileContents,
      cursorPosition,
      indentation: 2,
      language,
      saving,
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
      cursorPosition,
      language,
      saving,
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
