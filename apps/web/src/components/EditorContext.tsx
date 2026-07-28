import { createContext, useCallback, useContext, useState } from 'react';
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

export function EditorProvider({ children }: { children: ReactNode }) {
  const [openTabs, setOpenTabs] = useState<OpenTab[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string | null>(null);
  const [cursorPosition, setCursorPosition] = useState<CursorPosition>({
    line: 1,
    column: 1,
  });
  const [language, setLanguage] = useState('typescript');
  const [saving, setSaving] = useState(false);

  const openFile = useCallback((path: string, name: string, lang?: string) => {
    setOpenTabs((prev) => {
      const existing = prev.find((t) => t.path === path);

      if (existing) {
        return prev;
      }

      return [...prev, { path, name, isModified: false }];
    });
    setActiveFilePath(path);
    setLanguage(lang ?? languageFromPath(path));
  }, []);

  const closeTab = useCallback(
    (path: string) => {
      setOpenTabs((prev) => {
        const next = prev.filter((t) => t.path !== path);
        // If closing the active tab, switch to another
        if (path === activeFilePath && next.length > 0) {
          const idx = prev.findIndex((t) => t.path === path);
          const newActive = next[Math.min(idx, next.length - 1)];

          if (newActive) {
            setActiveFilePath(newActive.path);
            setLanguage(languageFromPath(newActive.path));
          }
        }

        if (next.length === 0) {
          setActiveFilePath(null);
        }

        return next;
      });
    },
    [activeFilePath],
  );

  const setActiveFile = useCallback((path: string) => {
    setActiveFilePath(path);
    setLanguage(languageFromPath(path));
  }, []);

  const markModified = useCallback((path: string, modified: boolean) => {
    setOpenTabs((prev) =>
      prev.map((t) => (t.path === path ? { ...t, isModified: modified } : t)),
    );
  }, []);

  return (
    <EditorContext.Provider
      value={{
        openTabs,
        activeFilePath,
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
      }}
    >
      {children}
    </EditorContext.Provider>
  );
}

export function useEditor(): EditorContextValue {
  const ctx = useContext(EditorContext);

  if (!ctx) {
    throw new Error('useEditor must be used within an EditorProvider');
  }

  return ctx;
}
