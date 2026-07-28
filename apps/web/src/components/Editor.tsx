import MonacoEditor, { type OnMount } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { useCallback, useEffect, useRef, useState } from 'react';

import { useEditor } from './EditorContext';

const DEFAULT_CODE = `// Welcome to qedit
// A lightweight file editor

function greet(name: string): string {
  return \`Hello, \${name}!\`;
}

console.log(greet('World'));
`;

export function Editor() {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const [monacoReady, setMonacoReady] = useState(false);
  const {
    language,
    activeFilePath,
    fileContents,
    fileErrors,
    setCursorPosition,
    markModified,
    updateFileContent,
  } = useEditor();

  // Configure monaco from the bundled package (no CDN) before first render
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

  const handleEditorMount: OnMount = useCallback(
    (editor, _monaco) => {
      editorRef.current = editor;
      editor.focus();

      const pos = editor.getPosition();

      if (pos) {
        setCursorPosition({ line: pos.lineNumber, column: pos.column });
      }

      editor.onDidChangeCursorPosition((e) => {
        setCursorPosition({
          line: e.position.lineNumber,
          column: e.position.column,
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

  const readError = activeFilePath ? fileErrors.get(activeFilePath) : undefined;

  if (readError && activeFilePath) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 bg-background p-6 text-center">
        <p className="text-sm font-medium text-destructive">
          Could not open {activeFilePath}
        </p>
        <p className="max-w-lg text-xs text-muted-foreground">{readError}</p>
      </div>
    );
  }

  if (!monacoReady) {
    return (
      <div className="flex h-full items-center justify-center bg-background">
        <span className="text-xs text-muted-foreground">Loading editor…</span>
      </div>
    );
  }

  // Determine what value to show in Monaco
  const currentValue =
    activeFilePath && fileContents.has(activeFilePath)
      ? (fileContents.get(activeFilePath) ?? '')
      : DEFAULT_CODE;

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
