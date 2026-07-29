import MonacoEditor, { type OnMount } from '@monaco-editor/react';
import { Edit3, Eye } from 'lucide-react';
import type { editor } from 'monaco-editor';
import { useCallback, useEffect, useRef, useState } from 'react';

import { useEditor } from './EditorContext';
import { EmptyState } from './EmptyState';

import { MarkdownPreview } from '#/lib/markdown';

export function Editor() {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const [monacoReady, setMonacoReady] = useState(false);
  const [markdownPreview, setMarkdownPreview] = useState(false);
  const {
    language,
    activeFilePath,
    fileContents,
    fileStatus,
    setCursorPosition,
    markModified,
    updateFileContent,
  } = useEditor();

  useEffect(() => {
    setMarkdownPreview(language === 'markdown');
  }, [activeFilePath, language]);

  useEffect(() => {
    let cancelled = false;

    void import('#/lib/monaco-setup').then(({ configureMonaco }) => {
      configureMonaco();

      if (!cancelled) setMonacoReady(true);
    });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const handleFind = () => {
      editorRef.current?.trigger('keyboard', 'actions.find', null);
    };

    const handleFocus = () => {
      editorRef.current?.focus();
    };

    window.addEventListener('qedit:find', handleFind);
    window.addEventListener('qedit:focus-editor', handleFocus);

    return () => {
      window.removeEventListener('qedit:find', handleFind);
      window.removeEventListener('qedit:focus-editor', handleFocus);
    };
  }, []);

  const handleEditorMount: OnMount = useCallback(
    (mountedEditor, _monaco) => {
      editorRef.current = mountedEditor;
      mountedEditor.focus();

      const pos = mountedEditor.getPosition();

      if (pos) {
        setCursorPosition({ line: pos.lineNumber, column: pos.column });
      }

      mountedEditor.onDidChangeCursorPosition((event) => {
        setCursorPosition({
          line: event.position.lineNumber,
          column: event.position.column,
        });
      });
    },
    [setCursorPosition],
  );

  const handleChange = useCallback(
    (value: string | undefined) => {
      if (activeFilePath) {
        markModified(activeFilePath, true);
        updateFileContent(activeFilePath, value ?? '');
      }
    },
    [activeFilePath, markModified, updateFileContent],
  );

  const status = activeFilePath ? fileStatus.get(activeFilePath) : undefined;

  if (!activeFilePath) return <EmptyState />;

  if (status?.kind === 'error') {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 bg-background p-6 text-center">
        <p className="text-sm font-medium text-destructive">
          Could not open {activeFilePath}
        </p>
        <p className="max-w-lg text-xs text-muted-foreground">
          {status.message}
        </p>
      </div>
    );
  }

  if (!monacoReady) {
    return (
      <div className="flex h-full items-center justify-center bg-background">
        <span className="text-xs text-muted-foreground">Loading editor...</span>
      </div>
    );
  }

  // Never mount Monaco over a file whose read has not completed. The fallback
  // content must not be editable into an unread buffer.
  if (status?.kind !== 'loaded') {
    return (
      <div className="flex h-full items-center justify-center bg-background">
        <span className="text-xs text-muted-foreground">
          Loading {activeFilePath}...
        </span>
      </div>
    );
  }

  const currentValue = fileContents.get(activeFilePath) ?? '';

  if (language === 'markdown') {
    return (
      <div className="flex h-full min-h-0 flex-col bg-background">
        <div className="flex h-9 shrink-0 items-center justify-between border-b px-3 text-xs">
          <span className="font-medium text-muted-foreground">Markdown</span>
          <div className="flex items-center gap-1 rounded border p-0.5">
            <button
              type="button"
              className={`inline-flex items-center gap-1 rounded px-2 py-1 ${
                !markdownPreview
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground'
              }`}
              onClick={() => setMarkdownPreview(false)}
              aria-pressed={!markdownPreview}
            >
              <Edit3 className="h-3 w-3" />
              Edit
            </button>
            <button
              type="button"
              className={`inline-flex items-center gap-1 rounded px-2 py-1 ${
                markdownPreview
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground'
              }`}
              onClick={() => setMarkdownPreview(true)}
              aria-pressed={markdownPreview}
            >
              <Eye className="h-3 w-3" />
              Preview
            </button>
          </div>
        </div>
        {markdownPreview ? (
          <div className="min-h-0 flex-1 overflow-auto">
            <MarkdownPreview content={currentValue} />
          </div>
        ) : (
          <div className="min-h-0 flex-1">
            <MonacoEditor
              height="100%"
              defaultLanguage="markdown"
              value={currentValue}
              language={language}
              onChange={handleChange}
              onMount={handleEditorMount}
              theme="vs-dark"
              options={{
                fontSize: 14,
                lineNumbers: 'on',
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                wordWrap: 'on',
                tabSize: 2,
                automaticLayout: true,
              }}
            />
          </div>
        )}
      </div>
    );
  }

  return (
    <MonacoEditor
      height="100%"
      defaultLanguage="typescript"
      value={currentValue}
      language={language}
      onChange={handleChange}
      onMount={handleEditorMount}
      theme="vs-dark"
      options={{
        fontSize: 14,
        lineNumbers: 'on',
        minimap: { enabled: true },
        scrollBeyondLastLine: false,
        wordWrap: 'on',
        tabSize: 2,
        automaticLayout: true,
      }}
    />
  );
}
