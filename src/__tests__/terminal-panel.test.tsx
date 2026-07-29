import { render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { EditorProvider } from '../components/EditorContext';
import { TerminalPanel } from '../components/TerminalPanel';
import { WorkspaceProvider } from '../components/WorkspaceContext';

vi.mock('#/lib/terminal-bridge', () => ({
  closeTerminal: vi.fn(() => Promise.resolve()),
  listenTerminalExit: vi.fn(() => Promise.resolve(() => {})),
  listenTerminalOutput: vi.fn(() => Promise.resolve(() => {})),
  resizeTerminal: vi.fn(() => Promise.resolve()),
  spawnTerminal: vi.fn(() => Promise.resolve(1)),
  writeTerminal: vi.fn(() => Promise.resolve()),
}));

const wrapper = ({ children }: { children: ReactNode }) => (
  <EditorProvider>
    <WorkspaceProvider>{children}</WorkspaceProvider>
  </EditorProvider>
);

describe('TerminalPanel discoverability', () => {
  it('shows the terminal strip, active tab, and visible new-terminal action', () => {
    render(<TerminalPanel />, { wrapper });

    expect(screen.getByText('Terminal')).toBeTruthy();
    expect(screen.getByRole('tab', { name: /Terminal 1/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'New terminal' })).toBeTruthy();
  });
});
