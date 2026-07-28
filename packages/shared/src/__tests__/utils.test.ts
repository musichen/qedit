import { describe, it, expect } from 'vitest';

import { cn } from '../utils';

describe('utils', () => {
  it('cn joins class names', () => {
    expect(cn('a', 'b')).toBe('a b');
    expect(cn('a', undefined, 'b')).toBe('a b');
  });
});
