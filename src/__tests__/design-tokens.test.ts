import { describe, expect, it, vi } from 'vitest';

import { designToken, withThemeTokens } from '../lib/design-tokens';

describe('design tokens', () => {
  it('expands three-digit hex colors for canvas consumers', () => {
    expect(designToken('#fff')).toBe('#ffffff');
    expect(designToken('#abc')).toBe('#aabbcc');
  });

  it('passes six-digit light-theme colors to consumers', () => {
    const getComputedStyleSpy = vi
      .spyOn(window, 'getComputedStyle')
      .mockReturnValue({
        getPropertyValue: () => '#fff',
      } as unknown as CSSStyleDeclaration);

    expect(
      withThemeTokens('light', (read) => read('--qedit-bg-editor', '#000000')),
    ).toBe('#ffffff');

    getComputedStyleSpy.mockRestore();
  });
});
