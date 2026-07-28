import MonacoEditor, { type OnMount } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { useCallback, useEffect, useRef, useState } from 'react';

import { useEditor } from './EditorContext';
import { EmptyState } from './EmptyState';

export function Editor() {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const [monacoReady, setMonacoReady] = useState(false);
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

    window.addEventListener('qedit:find', handleFind);

    return () => window.removeEventListener('qedit:find', handleFind);
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
