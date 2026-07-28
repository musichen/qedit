import MonacoEditor, { type OnMount } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { useCallback, useRef } from 'react';

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
  const { language, setCursorPosition, markModified, activeFilePath } =
    useEditor();

  const handleEditorMount: OnMount = useCallback(
    (editor, _monaco) => {
      editorRef.current = editor;
      editor.focus();

      // Report initial cursor position
      const pos = editor.getPosition();

      if (pos) {
        setCursorPosition({ line: pos.lineNumber, column: pos.column });
      }

      // Listen for cursor changes
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
    (_value: string | undefined) => {
      if (activeFilePath) {
        markModified(activeFilePath, true);
      }
    },
    [activeFilePath, markModified],
  );

  return (
    <MonacoEditor
      height="100%"
      defaultLanguage="typescript"
      defaultValue={DEFAULT_CODE}
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
