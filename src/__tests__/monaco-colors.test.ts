import { describe, expect, it } from 'vitest';

import { toMonacoTokenColor } from '../lib/monaco-colors';

describe('Monaco token colors', () => {
  it('converts valid hex colors to Monaco token format', () => {
    expect(toMonacoTokenColor('#abcdef')).toBe('abcdef');
  });

  it('does not pass named CSS colors to Monaco token rules', () => {
    expect(toMonacoTokenColor('green', '#6a9955')).toBe('6a9955');
  });

  it('falls back when a token is not a valid RGB hex color', () => {
    expect(toMonacoTokenColor('rgb(0 0 0)', '#123456')).toBe('123456');
  });
});
