import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { EditorProvider } from '../components/EditorContext';
import { TerminalPanel } from '../components/TerminalPanel';
import { WorkspaceProvider } from '../components/WorkspaceContext';

import { TERMINAL_DRAG_TYPE } from '#/lib/terminal-drag';

const { spawnTerminal } = vi.hoisted(() => ({
  spawnTerminal: vi.fn(() => Promise.resolve(1)),
}));

vi.mock('#/lib/terminal-bridge', () => ({
  closeTerminal: vi.fn(() => Promise.resolve()),
  listenTerminalExit: vi.fn(() => Promise.resolve(() => {})),
  listenTerminalOutput: vi.fn(() => Promise.resolve(() => {})),
  resizeTerminal: vi.fn(() => Promise.resolve()),
  spawnTerminal,
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

  it('keeps the same PTY session when moving between panel and editor mode', async () => {
    const { container, rerender } = render(<TerminalPanel visible />, {
      wrapper,
    });

    await waitFor(() =>
      expect(container.querySelector('.xterm')).not.toBeNull(),
    );
    const terminalElement = container.querySelector('.xterm');

    rerender(<TerminalPanel visible editorMode />);
    rerender(<TerminalPanel visible={false} />);

    expect(container.querySelector('.xterm')).toBe(terminalElement);
  });

  it('makes each bottom terminal session tab draggable to the editor topbar', () => {
    render(<TerminalPanel visible />, { wrapper });
    const values = new Map<string, string>();
    const dataTransfer = {
      effectAllowed: 'none',
      setData: (type: string, value: string) => values.set(type, value),
    } as unknown as DataTransfer;

    fireEvent.dragStart(screen.getByRole('tab', { name: /Terminal 1/ }), {
      dataTransfer,
    });

    expect(values.get(TERMINAL_DRAG_TYPE)).toBe('terminal-1');
  });
});
