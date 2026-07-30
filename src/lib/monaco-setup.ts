import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker';
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker';
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';

import { MONACO_THEMES } from './monaco-themes';

declare global {
  interface Window {
    MonacoEnvironment?: monaco.Environment;
  }
}

let configured = false;

/**
 * Point @monaco-editor/react at the bundled monaco-editor package instead of
 * its default CDN loader, so the packaged desktop app needs no network access.
 * Must run in the browser only — monaco touches DOM globals at import time.
 */
export function configureMonaco(): void {
  if (configured) return;
  configured = true;

  window.MonacoEnvironment = {
    getWorker(_workerId: string, label: string) {
      switch (label) {
        case 'json':
          return new jsonWorker();
        case 'css':
        case 'scss':
        case 'less':
          return new cssWorker();
        case 'html':
        case 'handlebars':
        case 'razor':
          return new htmlWorker();
        case 'typescript':
        case 'javascript':
          return new tsWorker();
        default:
          return new editorWorker();
      }
    },
  };

  loader.config({ monaco });
  defineQEditThemes();
}

function defineQEditThemes(): void {
  monaco.editor.defineTheme(MONACO_THEMES.dark, {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '707782', fontStyle: 'italic' },
      { token: 'keyword', foreground: 'c6a0f6' },
      { token: 'string', foreground: 'a6d189' },
      { token: 'number', foreground: 'ef9f76' },
      { token: 'type', foreground: '8caaee' },
      { token: 'identifier', foreground: 'd5d8e0' },
    ],
    colors: {
      'editor.background': '#17181c',
      'editor.foreground': '#d5d8e0',
      'editorLineNumber.foreground': '#626875',
      'editorLineNumber.activeForeground': '#b8beca',
      'editorCursor.foreground': '#c6a0f6',
      'editor.selectionBackground': '#343842',
      'editor.inactiveSelectionBackground': '#282b32',
      'editor.lineHighlightBackground': '#1c1e23',
      'editorIndentGuide.background': '#25282f',
      'editorIndentGuide.activeBackground': '#343842',
      'editorWidget.background': '#202229',
      'editorWidget.border': '#343842',
      'editorSuggestWidget.background': '#202229',
      'editorSuggestWidget.border': '#343842',
    },
  });

  monaco.editor.defineTheme(MONACO_THEMES.light, {
    base: 'vs',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '7b8190', fontStyle: 'italic' },
      { token: 'keyword', foreground: '7c3aed' },
      { token: 'string', foreground: '3f7d37' },
      { token: 'number', foreground: 'c05a21' },
      { token: 'type', foreground: '2f5f9e' },
      { token: 'identifier', foreground: '30333b' },
    ],
    colors: {
      'editor.background': '#fbfbfc',
      'editor.foreground': '#30333b',
      'editorLineNumber.foreground': '#a0a5b0',
      'editorLineNumber.activeForeground': '#626875',
      'editorCursor.foreground': '#7c3aed',
      'editor.selectionBackground': '#ddd8eb',
      'editor.inactiveSelectionBackground': '#eceaf0',
      'editor.lineHighlightBackground': '#f5f4f7',
      'editorIndentGuide.background': '#e7e5ea',
      'editorIndentGuide.activeBackground': '#d6d2dc',
      'editorWidget.background': '#ffffff',
      'editorWidget.border': '#dedce3',
      'editorSuggestWidget.background': '#ffffff',
      'editorSuggestWidget.border': '#dedce3',
    },
  });
}
