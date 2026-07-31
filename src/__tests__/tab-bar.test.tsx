import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { EditorProvider } from '../components/EditorContext';
import { TabBar } from '../components/TabBar';

describe('TabBar terminal action', () => {
  it('offers a distinct terminal tab action', () => {
    const onOpenTerminal = vi.fn();

    render(
      <EditorProvider>
        <TabBar onOpenTerminal={onOpenTerminal} />
      </EditorProvider>,
    );

    const terminalButton = screen.getByRole('button', {
      name: 'Open Terminal',
    });
    expect(terminalButton.textContent).toContain('Terminal');

    fireEvent.click(terminalButton);
    expect(onOpenTerminal).toHaveBeenCalledTimes(1);
  });

  it('accepts a dragged terminal action in the file tab area', () => {
    const onDropTerminal = vi.fn();

    render(
      <EditorProvider>
        <TabBar onDropTerminal={onDropTerminal} />
      </EditorProvider>,
    );

    const terminalButton = screen.getByRole('button', {
      name: 'Open Terminal',
    });
    const fileTabs = screen.getByRole('tablist', { name: 'Open files' });
    const data = {
      types: ['text/qedit-terminal'],
      dropEffect: 'none',
      getData: () => 'active',
      setData: () => undefined,
    } as unknown as DataTransfer;

    fireEvent.dragStart(terminalButton, { dataTransfer: data });
    fireEvent.drop(fileTabs, { dataTransfer: data });

    expect(onDropTerminal).toHaveBeenCalledTimes(1);
  });

  it('accepts a bottom terminal session tab payload and keeps its session id', () => {
    const onDropTerminal = vi.fn();

    render(
      <EditorProvider>
        <TabBar onDropTerminal={onDropTerminal} />
      </EditorProvider>,
    );

    const fileTabs = screen.getByRole('tablist', { name: 'Open files' });
    const data = {
      types: ['application/x-qedit-terminal'],
      dropEffect: 'none',
      getData: (type: string) =>
        type === 'application/x-qedit-terminal' ? 'terminal-2' : '',
      setData: () => undefined,
    } as unknown as DataTransfer;

    fireEvent.dragOver(fileTabs, { dataTransfer: data });
    fireEvent.drop(fileTabs, { dataTransfer: data });

    expect(onDropTerminal).toHaveBeenCalledWith('terminal-2');
  });

  it('offers a direct editor-area terminal affordance', () => {
    const onOpenTerminalEditor = vi.fn();

    render(
      <EditorProvider>
        <TabBar onOpenTerminalEditor={onOpenTerminalEditor} />
      </EditorProvider>,
    );

    fireEvent.click(
      screen.getByRole('button', { name: 'Open Terminal in editor tab' }),
    );

    expect(onOpenTerminalEditor).toHaveBeenCalledTimes(1);
  });

  it('renders the pinned terminal as an editor-area tab and can unpin it', () => {
    const onCloseTerminalEditor = vi.fn();

    render(
      <EditorProvider>
        <TabBar
          terminalEditorVisible
          onCloseTerminalEditor={onCloseTerminalEditor}
        />
      </EditorProvider>,
    );

    expect(
      screen.getByRole('tab', { name: 'Terminal editor tab' }),
    ).toBeTruthy();

    fireEvent.click(
      screen.getByRole('button', { name: 'Close terminal editor tab' }),
    );

    expect(onCloseTerminalEditor).toHaveBeenCalledTimes(1);
  });
});
