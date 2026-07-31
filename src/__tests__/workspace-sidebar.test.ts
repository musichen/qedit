import { describe, expect, it } from 'vitest';

import { shouldOpenSidebarForWorkspace } from '#/lib/workspace-sidebar';

describe('workspace sidebar behavior', () => {
  it('opens when a new workspace root is selected', () => {
    expect(shouldOpenSidebarForWorkspace(null, '/home/project')).toBe(true);
    expect(
      shouldOpenSidebarForWorkspace('/home/old-project', '/home/project'),
    ).toBe(true);
  });

  it('does not reopen after manual hiding until the workspace changes', () => {
    expect(
      shouldOpenSidebarForWorkspace('/home/project', '/home/project'),
    ).toBe(false);
    expect(shouldOpenSidebarForWorkspace('/home/project', null)).toBe(false);
  });
});
