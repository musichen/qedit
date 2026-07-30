import MonacoEditor, { type OnMount } from '@monaco-editor/react';
import { Edit3, Eye } from 'lucide-react';
import type { editor } from 'monaco-editor';
import { useCallback, useEffect, useRef, useState } from 'react';

import { useEditor } from './EditorContext';
import { EmptyState } from './EmptyState';
import { useSettings } from './SettingsContext';

import { MarkdownPreview } from '#/lib/markdown';
import { EDITOR_MENU_COMMANDS } from '#/lib/menu-actions';
import { MONACO_THEMES } from '#/lib/monaco-themes';

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
  const { settings, resolvedTheme } = useSettings();

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
    const handleCommand = (event: Event) => {
      const action = (event as CustomEvent<string>).detail;
      const command = EDITOR_MENU_COMMANDS[action];

      if (command) editorRef.current?.trigger('menu', command, null);
    };

    window.addEventListener('qedit:editor-command', handleCommand);

    return () =>
      window.removeEventListener('qedit:editor-command', handleCommand);
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
      <div className="flex h-full flex-col items-center justify-center gap-2 bg-editor p-6 text-center">
        <p className="text-sm font-medium text-danger">
          Could not open {activeFilePath}
        </p>
        <p className="max-w-lg text-xs text-text-secondary">{status.message}</p>
      </div>
    );
  }

  if (!monacoReady) {
    return (
      <div className="flex h-full items-center justify-center bg-editor">
        <span className="text-xs text-text-secondary">Loading editor...</span>
      </div>
    );
  }

  // Never mount Monaco over a file whose read has not completed. The fallback
  // content must not be editable into an unread buffer.
  if (status?.kind !== 'loaded') {
    return (
      <div className="flex h-full items-center justify-center bg-editor">
        <span className="text-xs text-text-secondary">
          Loading {activeFilePath}...
        </span>
      </div>
    );
  }

  const currentValue = fileContents.get(activeFilePath) ?? '';

  if (language === 'markdown') {
    return (
      <div className="flex h-full min-h-0 flex-col bg-editor">
        <div className="flex h-tab shrink-0 items-center justify-between border-b border-border-subtle px-3 text-xs">
          <span className="font-medium text-text-secondary">Markdown</span>
          <div className="flex items-center gap-1 rounded border border-border-default p-0.5">
            <button
              type="button"
              className={`inline-flex items-center gap-1 rounded px-2 py-1 ${
                !markdownPreview
                  ? 'bg-hover text-text-primary'
                  : 'text-text-secondary'
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
                  ? 'bg-hover text-text-primary'
                  : 'text-text-secondary'
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
              theme={MONACO_THEMES[resolvedTheme]}
              options={{
                fontSize: settings.fontSize,
                lineNumbers: settings.lineNumbers ? 'on' : 'off',
                minimap: { enabled: settings.minimap },
                scrollBeyondLastLine: false,
                wordWrap: settings.wordWrap,
                tabSize: settings.tabSize,
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
      theme={MONACO_THEMES[resolvedTheme]}
      options={{
        fontSize: settings.fontSize,
        lineNumbers: settings.lineNumbers ? 'on' : 'off',
        minimap: { enabled: settings.minimap },
        scrollBeyondLastLine: false,
        wordWrap: settings.wordWrap,
        tabSize: settings.tabSize,
        automaticLayout: true,
      }}
    />
  );
}
