import MonacoEditor, { type OnMount } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { useCallback, useEffect, useRef } from 'react';

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
  const {
    language,
    activeFilePath,
    fileContents,
    setCursorPosition,
    markModified,
    updateFileContent,
  } = useEditor();

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

  // When the active tab changes and content is available, set the editor model
  useEffect(() => {
    const editor = editorRef.current;

    if (!editor) return;

    if (activeFilePath && fileContents.has(activeFilePath)) {
      const content = fileContents.get(activeFilePath) ?? '';
      const model = editor.getModel();

      if (model && model.getValue() !== content) {
        model.setValue(content);
      }
    }
  }, [activeFilePath, fileContents]);

  // Determine what value to show in Monaco
  const currentValue =
    activeFilePath && fileContents.has(activeFilePath)
      ? (fileContents.get(activeFilePath) ?? '')
      : DEFAULT_CODE;

  return (
    <MonacoEditor
      height="100%"
      defaultLanguage="typescript"
      defaultValue={DEFAULT_CODE}
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
