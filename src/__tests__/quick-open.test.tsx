import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { QuickOpen } from '../components/QuickOpen';

const openWorkspaceFile = vi.fn(() => Promise.resolve());

vi.mock('../components/WorkspaceContext', () => ({
  useWorkspace: () => ({
    knownFiles: [
      { name: 'README.md', path: '/home/project/docs/README.md', isFile: true },
      { name: 'main.ts', path: '/home/project/src/main.ts', isFile: true },
    ],
    recentFiles: [],
    openWorkspaceFile,
    discoverWorkspaceFiles: () => Promise.resolve(),
    workspaceRoot: '/home/project',
  }),
}));

describe('QuickOpen search', () => {
  it('filters by filename and path and gives a useful no-results state', async () => {
    render(<QuickOpen onClose={vi.fn()} />);
    const input = screen.getByRole('textbox', { name: 'Search files' });

    await waitFor(() => expect(screen.getByText('README.md')).toBeTruthy(), {
      timeout: 15000,
    });

    await act(async () => {
      fireEvent.change(input, { target: { value: 'docs' } });
    });
    expect(screen.getByRole('option', { name: /README\.md/ })).toBeTruthy();
    expect(screen.queryByRole('option', { name: /main\.ts/ })).toBeNull();

    await act(async () => {
      fireEvent.change(input, { target: { value: 'missing-file' } });
    });
    expect(screen.getByText('No matching files')).toBeTruthy();
  }, 20000);
});
