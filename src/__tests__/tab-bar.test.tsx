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
});
