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
});
