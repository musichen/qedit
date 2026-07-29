import { describe, expect, it } from 'vitest';

import { shortcutActionForEvent } from '../lib/shortcuts';

const event = (overrides: Partial<KeyboardEvent>): KeyboardEvent =>
  ({
    key: 's',
    metaKey: false,
    ctrlKey: true,
    shiftKey: false,
    target: document.body,
    ...overrides,
  }) as KeyboardEvent;

describe('shortcutActionForEvent', () => {
  it('dispatches file, folder, save, and save-as shortcuts', () => {
    expect(shortcutActionForEvent(event({ key: 'o' }))).toBe('open-file');
    expect(shortcutActionForEvent(event({ key: 'o', shiftKey: true }))).toBe(
      'open-folder',
    );
    expect(shortcutActionForEvent(event({ key: 's' }))).toBe('save');
    expect(shortcutActionForEvent(event({ key: 's', shiftKey: true }))).toBe(
      'save-as',
    );
  });

  it('dispatches close, reopen, reload, focus, and search shortcuts', () => {
    expect(shortcutActionForEvent(event({ key: 'w' }))).toBe('close-tab');
    expect(shortcutActionForEvent(event({ key: 't', shiftKey: true }))).toBe(
      'reopen-tab',
    );
    expect(shortcutActionForEvent(event({ key: 'r', shiftKey: true }))).toBe(
      'reload-file',
    );
    expect(shortcutActionForEvent(event({ key: 'PageDown' }))).toBe('next-tab');
    expect(
      shortcutActionForEvent(event({ key: 'PageUp', shiftKey: true })),
    ).toBe('previous-tab');
    expect(shortcutActionForEvent(event({ key: '`' }))).toBe('focus-terminal');
    expect(shortcutActionForEvent(event({ key: 'p' }))).toBe('quick-open');
    expect(shortcutActionForEvent(event({ key: 'f' }))).toBe('find');
  });

  it('accepts either platform modifier', () => {
    expect(
      shortcutActionForEvent(
        event({ key: 'o', ctrlKey: false, metaKey: true }),
      ),
    ).toBe('open-file');
  });

  it('does not steal shortcuts from text fields', () => {
    const input = document.createElement('input');
    const contentEditable = document.createElement('div');
    contentEditable.contentEditable = 'true';

    expect(
      shortcutActionForEvent(event({ key: 'o', target: input })),
    ).toBeNull();
    expect(
      shortcutActionForEvent(event({ key: 'w', target: contentEditable })),
    ).toBeNull();
  });

  it('leaves terminal input shortcuts to xterm but keeps a way back out', () => {
    const terminal = document.createElement('textarea');
    terminal.className = 'xterm-helper-textarea';

    expect(shortcutActionForEvent(event({ key: 's', target: terminal }))).toBe(
      null,
    );
    expect(shortcutActionForEvent(event({ key: 'w', target: terminal }))).toBe(
      null,
    );
    expect(shortcutActionForEvent(event({ key: '`', target: terminal }))).toBe(
      'focus-editor',
    );
    expect(
      shortcutActionForEvent(event({ key: 'Tab', target: terminal })),
    ).toBe('next-terminal');
    expect(
      shortcutActionForEvent(
        event({ key: 'Tab', shiftKey: true, target: terminal }),
      ),
    ).toBe('previous-terminal');
    expect(shortcutActionForEvent(event({ key: '2', target: terminal }))).toBe(
      'terminal-2',
    );
  });

  it('toggles focus back to the editor from anywhere inside the terminal', () => {
    const terminalRoot = document.createElement('div');
    terminalRoot.className = 'xterm';
    const screen = document.createElement('div');
    terminalRoot.append(screen);

    expect(shortcutActionForEvent(event({ key: '`', target: screen }))).toBe(
      'focus-editor',
    );
  });

  it('keeps terminal-tab navigation scoped to terminal tabs', () => {
    const terminalTab = document.createElement('button');
    terminalTab.dataset.terminalTab = 'terminal-1';

    expect(
      shortcutActionForEvent(event({ key: 'w', target: terminalTab })),
    ).toBe('close-terminal');
    expect(
      shortcutActionForEvent(event({ key: '3', target: terminalTab })),
    ).toBe('terminal-3');
  });

  it('keeps workspace shortcuts alive inside the Monaco editor', () => {
    const editor = document.createElement('div');
    editor.className = 'monaco-editor';
    const inputArea = document.createElement('textarea');
    inputArea.className = 'inputarea';
    editor.append(inputArea);

    expect(shortcutActionForEvent(event({ key: 's', target: inputArea }))).toBe(
      'save',
    );
    expect(
      shortcutActionForEvent(
        event({ key: 's', shiftKey: true, target: inputArea }),
      ),
    ).toBe('save-as');
    expect(shortcutActionForEvent(event({ key: 'o', target: inputArea }))).toBe(
      'open-file',
    );
    expect(
      shortcutActionForEvent(
        event({ key: 'o', shiftKey: true, target: inputArea }),
      ),
    ).toBe('open-folder');
    expect(shortcutActionForEvent(event({ key: 'w', target: inputArea }))).toBe(
      'close-tab',
    );
    expect(shortcutActionForEvent(event({ key: 'p', target: inputArea }))).toBe(
      'quick-open',
    );
    expect(shortcutActionForEvent(event({ key: 'f', target: inputArea }))).toBe(
      'find',
    );
  });

  it('ignores unrelated keys and events without a modifier', () => {
    expect(
      shortcutActionForEvent(event({ key: 'o', ctrlKey: false })),
    ).toBeNull();
    expect(shortcutActionForEvent(event({ key: 'q' }))).toBeNull();
  });
});
