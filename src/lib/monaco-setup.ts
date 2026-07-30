import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker';
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker';
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';

import { designToken } from './design-tokens';
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
  const token = (name: string, fallback: string) => designToken(name, fallback);
  const ruleColor = (name: string, fallback: string) =>
    token(name, fallback).replace(/^#/, '');
  const editorBackground = token('--color-bg-editor', '#303040');
  const editorForeground = token('--color-text-primary', '#d4d4d4');
  const muted = token('--color-text-muted', '#808080');
  const secondary = token('--color-text-secondary', '#b0b0b0');
  const accent = token('--color-text-accent', '#70a0ff');
  const selection = token('--color-bg-selection', '#2050c0');
  const border = token('--color-border-default', '#3c3c4c');
  const subtleBorder = token('--color-border-subtle', '#2a2a3a');

  monaco.editor.defineTheme(MONACO_THEMES.dark, {
    base: 'vs-dark',
    inherit: false,
    rules: [
      {
        token: 'comment',
        foreground: ruleColor('--color-syntax-comment', '#6a9955'),
        fontStyle: 'italic',
      },
      {
        token: 'keyword',
        foreground: ruleColor('--color-syntax-keyword', '#569cd6'),
      },
      {
        token: 'string',
        foreground: ruleColor('--color-syntax-string', '#ce9178'),
      },
      {
        token: 'number',
        foreground: ruleColor('--color-syntax-number', '#b5cea8'),
      },
      {
        token: 'type',
        foreground: ruleColor('--color-syntax-type', '#4ec9b0'),
      },
      {
        token: 'function',
        foreground: ruleColor('--color-syntax-function', '#dcdcaa'),
      },
      {
        token: 'variable',
        foreground: ruleColor('--color-syntax-variable', '#9cdcfe'),
      },
      { token: 'identifier', foreground: editorForeground.replace('#', '') },
    ],
    colors: {
      'editor.background': editorBackground,
      'editor.foreground': editorForeground,
      'editorLineNumber.foreground': muted,
      'editorLineNumber.activeForeground': secondary,
      'editorCursor.foreground': accent,
      'editor.selectionBackground': selection,
      'editor.inactiveSelectionBackground': subtleBorder,
      'editor.lineHighlightBackground': token(
        '--color-bg-tab-active',
        '#404050',
      ),
      'editorIndentGuide.background': subtleBorder,
      'editorIndentGuide.activeBackground': border,
      'editorWidget.background': token('--color-bg-dropdown', '#101010'),
      'editorWidget.border': border,
      'editorSuggestWidget.background': token(
        '--color-bg-command-palette',
        '#202030',
      ),
      'editorSuggestWidget.border': border,
    },
  });

  monaco.editor.defineTheme(MONACO_THEMES.light, {
    base: 'vs-dark',
    inherit: false,
    rules: [
      {
        token: 'comment',
        foreground: ruleColor('--color-syntax-comment', '#6a9955'),
        fontStyle: 'italic',
      },
      {
        token: 'keyword',
        foreground: ruleColor('--color-syntax-keyword', '#569cd6'),
      },
      {
        token: 'string',
        foreground: ruleColor('--color-syntax-string', '#ce9178'),
      },
      {
        token: 'number',
        foreground: ruleColor('--color-syntax-number', '#b5cea8'),
      },
      {
        token: 'type',
        foreground: ruleColor('--color-syntax-type', '#4ec9b0'),
      },
      {
        token: 'function',
        foreground: ruleColor('--color-syntax-function', '#dcdcaa'),
      },
      {
        token: 'variable',
        foreground: ruleColor('--color-syntax-variable', '#9cdcfe'),
      },
      { token: 'identifier', foreground: editorForeground.replace('#', '') },
    ],
    colors: {
      'editor.background': editorBackground,
      'editor.foreground': editorForeground,
      'editorLineNumber.foreground': muted,
      'editorLineNumber.activeForeground': secondary,
      'editorCursor.foreground': accent,
      'editor.selectionBackground': selection,
      'editor.inactiveSelectionBackground': subtleBorder,
      'editor.lineHighlightBackground': token(
        '--color-bg-tab-active',
        '#404050',
      ),
      'editorIndentGuide.background': subtleBorder,
      'editorIndentGuide.activeBackground': border,
      'editorWidget.background': token('--color-bg-dropdown', '#101010'),
      'editorWidget.border': border,
      'editorSuggestWidget.background': token(
        '--color-bg-command-palette',
        '#202030',
      ),
      'editorSuggestWidget.border': border,
    },
  });
}
