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
    expect(
      shortcutActionForEvent(event({ key: 'o', shiftKey: true })),
    ).toBe('open-folder');
    expect(shortcutActionForEvent(event({ key: 's' }))).toBe('save');
    expect(
      shortcutActionForEvent(event({ key: 's', shiftKey: true })),
    ).toBe('save-as');
  });

  it('dispatches close, quick-open, and find shortcuts', () => {
    expect(shortcutActionForEvent(event({ key: 'w' }))).toBe('close-tab');
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

  it('ignores unrelated keys and events without a modifier', () => {
    expect(
      shortcutActionForEvent(event({ key: 'o', ctrlKey: false })),
    ).toBeNull();
    expect(shortcutActionForEvent(event({ key: 'q' }))).toBeNull();
  });
});
