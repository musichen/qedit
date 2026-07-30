import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker';
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker';
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';

import { withThemeTokens } from './design-tokens';
import type { TokenReader, TokenTheme } from './design-tokens';
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

/**
 * Define both Monaco themes from the design tokens. Each theme resolves its
 * own palette through a detached probe, so a single definition pass stays
 * correct no matter which theme the document is currently showing.
 */
export function defineQEditThemes(): void {
  monaco.editor.defineTheme(MONACO_THEMES.dark, qeditTheme('dark'));
  monaco.editor.defineTheme(MONACO_THEMES.light, qeditTheme('light'));
}

const DARK_FALLBACKS: Record<string, string> = {
  '--qedit-bg-editor': '#303040',
  '--qedit-text-primary': '#d4d4d4',
  '--qedit-text-muted': '#808080',
  '--qedit-text-secondary': '#b0b0b0',
  '--qedit-text-accent': '#70a0ff',
  '--qedit-bg-selection': '#2050c0',
  '--qedit-border-default': '#3c3c4c',
  '--qedit-border-subtle': '#2a2a3a',
  '--qedit-bg-line-highlight': '#404050',
  '--qedit-bg-dropdown': '#101010',
  '--qedit-bg-command-palette': '#202030',
  '--qedit-syntax-comment': '#6a9955',
  '--qedit-syntax-keyword': '#569cd6',
  '--qedit-syntax-string': '#ce9178',
  '--qedit-syntax-number': '#b5cea8',
  '--qedit-syntax-function': '#dcdcaa',
  '--qedit-syntax-variable': '#9cdcfe',
  '--qedit-syntax-type': '#4ec9b0',
};

const LIGHT_FALLBACKS: Record<string, string> = {
  '--qedit-bg-editor': '#ffffff',
  '--qedit-text-primary': '#1f1f28',
  '--qedit-text-muted': '#6b6b80',
  '--qedit-text-secondary': '#45455a',
  '--qedit-text-accent': '#1a56c4',
  '--qedit-bg-selection': '#add6ff',
  '--qedit-border-default': '#c6c6d2',
  '--qedit-border-subtle': '#e1e1ea',
  '--qedit-bg-line-highlight': '#f1f1f6',
  '--qedit-bg-dropdown': '#ffffff',
  '--qedit-bg-command-palette': '#f7f7fa',
  '--qedit-syntax-comment': '#008000',
  '--qedit-syntax-keyword': '#0000ff',
  '--qedit-syntax-string': '#a31515',
  '--qedit-syntax-number': '#098658',
  '--qedit-syntax-function': '#795e26',
  '--qedit-syntax-variable': '#001080',
  '--qedit-syntax-type': '#267f99',
};

/** Token fallbacks matter in jsdom and before the stylesheet resolves, so each
 * theme carries its own defaults rather than sharing the dark palette. */
export function themeTokenFallbacks(theme: TokenTheme): Record<string, string> {
  return theme === 'dark' ? DARK_FALLBACKS : LIGHT_FALLBACKS;
}

function qeditTheme(theme: TokenTheme): monaco.editor.IStandaloneThemeData {
  const fallbacks = themeTokenFallbacks(theme);

  return withThemeTokens(theme, (read: TokenReader) => {
    const token = (name: string) => read(name, fallbacks[name] ?? '#000000');
    const ruleColor = (name: string) => token(name).replace(/^#/, '');
    const editorBackground = token('--qedit-bg-editor');
    const editorForeground = token('--qedit-text-primary');
    const muted = token('--qedit-text-muted');
    const secondary = token('--qedit-text-secondary');
    const accent = token('--qedit-text-accent');
    const selection = token('--qedit-bg-selection');
    const border = token('--qedit-border-default');
    const subtleBorder = token('--qedit-border-subtle');

    return {
      base: theme === 'dark' ? 'vs-dark' : 'vs',
      inherit: false,
      rules: [
        {
          token: 'comment',
          foreground: ruleColor('--qedit-syntax-comment'),
          fontStyle: 'italic',
        },
        { token: 'keyword', foreground: ruleColor('--qedit-syntax-keyword') },
        { token: 'string', foreground: ruleColor('--qedit-syntax-string') },
        { token: 'number', foreground: ruleColor('--qedit-syntax-number') },
        { token: 'type', foreground: ruleColor('--qedit-syntax-type') },
        { token: 'function', foreground: ruleColor('--qedit-syntax-function') },
        { token: 'variable', foreground: ruleColor('--qedit-syntax-variable') },
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
        'editor.lineHighlightBackground': token('--qedit-bg-line-highlight'),
        'editorIndentGuide.background': subtleBorder,
        'editorIndentGuide.activeBackground': border,
        'editorWidget.background': token('--qedit-bg-dropdown'),
        'editorWidget.border': border,
        'editorSuggestWidget.background': token('--qedit-bg-command-palette'),
        'editorSuggestWidget.border': border,
      },
    };
  });
}
