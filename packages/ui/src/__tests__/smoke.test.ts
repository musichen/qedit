import { describe, it, expect } from 'vitest';

describe('@qedit/ui', () => {
  it('exports cn utility from shadcn index', async () => {
    const { cn } = await import('../shadcn');

    expect(cn).toBeDefined();
    expect(typeof cn).toBe('function');
  });

  it('exports cn from lib utils', async () => {
    const { cn } = await import('../lib/utils');

    expect(cn).toBeDefined();
    expect(cn('foo', 'bar')).toBe('foo bar');
  });
});
