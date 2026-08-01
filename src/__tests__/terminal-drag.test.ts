import { describe, expect, it } from 'vitest';

import {
  getTerminalDragId,
  isTerminalDragOver,
  setTerminalDragData,
} from '#/lib/terminal-drag';

function dataTransfer(): DataTransfer {
  const values = new Map<string, string>();
  const types: string[] = [];

  return {
    types,
    setData(type: string, value: string) {
      values.set(type, value);
      types.push(type);
    },
    getData(type: string) {
      return values.get(type) ?? '';
    },
  } as unknown as DataTransfer;
}

describe('terminal drag payload', () => {
  it('keeps a portable session id for the topbar drop zone', () => {
    const transfer = dataTransfer();
    setTerminalDragData(transfer, 'terminal-2');

    expect(isTerminalDragOver(transfer)).toBe(true);
    expect(getTerminalDragId(transfer)).toBe('terminal-2');
  });

  it('accepts the text fallback used by WebKit drag events', () => {
    const transfer = {
      types: ['text/plain'],
      getData: (type: string) =>
        type === 'text/plain' ? 'qedit-terminal:terminal-3' : '',
    } as unknown as DataTransfer;

    expect(isTerminalDragOver(transfer)).toBe(true);
    expect(getTerminalDragId(transfer)).toBe('terminal-3');
  });
});
