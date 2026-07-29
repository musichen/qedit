import { render, screen, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import { EditorProvider, useEditor } from '../components/EditorContext';
import { StatusBar } from '../components/StatusBar';

const readTextFile = vi.fn<(path: string) => Promise<string>>();
const writeTextFile = vi.fn<(path: string, content: string) => Promise<void>>();
const saveDialog = vi.fn<(options: unknown) => Promise<string | null>>();

vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: (path: string) => readTextFile(path),
  writeTextFile: (path: string, content: string) =>
    writeTextFile(path, content),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: (options: unknown) => saveDialog(options),
}));

vi.mock('@tauri-apps/api/path', () => ({
  homeDir: () => Promise.resolve('/home'),
}));

type Editor = ReturnType<typeof useEditor>;

let editor: Editor;

function EditorHandle() {
  editor = useEditor();

  return null;
}

const renderStatusBar = () =>
  render(
    <EditorProvider>
      <EditorHandle />
      <StatusBar />
    </EditorProvider>,
  );

const openLoaded = async (path: string) => {
  const name = path.split('/').pop() ?? path;

  act(() => {
    editor.openFile(path, name);
  });
  await waitFor(() => {
    expect(editor.fileStatus.get(path)?.kind).toBe('loaded');
  });
};

/** The status chip the user reads for the buffer currently on screen. */
const stateChip = () => screen.getByRole('status').textContent;
const alertText = () => screen.getByRole('alert').textContent ?? '';

/**
 * A native Save As dialog the test drives by hand: `opened` settles once the
 * app has actually opened it, and `choose` answers it later, so the test can
 * change tabs while the dialog is still on screen.
 */
function deferredSaveDialog() {
  let choose: (path: string | null) => void = () => {};
  const opened = new Promise<void>((markOpened) => {
    saveDialog.mockImplementation(
      () =>
        new Promise<string | null>((settle) => {
          choose = settle;
          markOpened();
        }),
    );
  });

  return { opened, choose: (path: string | null) => choose(path) };
}

beforeEach(() => {
  readTextFile.mockReset();
  writeTextFile.mockReset();
  saveDialog.mockReset();
  saveDialog.mockResolvedValue(null);
  readTextFile.mockResolvedValue('original');
  writeTextFile.mockResolvedValue(undefined);
});

describe('StatusBar save failure ownership', () => {
  it('shows "Save failed" when the failure belongs to the file on screen', async () => {
    writeTextFile.mockRejectedValue(new Error('disk full'));

    renderStatusBar();
    await openLoaded('/home/a.ts');

    act(() => {
      editor.updateFileContent('/home/a.ts', 'edited');
      editor.markModified('/home/a.ts', true);
    });
    await act(async () => {
      await editor.saveActiveFile();
    });

    expect(stateChip()).toBe('Save failed');
    expect(alertText()).toContain('disk full');
  });

  it('keeps a Save As failure visible without mislabelling the tab the user switched to', async () => {
    renderStatusBar();
    await openLoaded('/home/a.ts');
    await openLoaded('/home/b.ts');

    act(() => {
      editor.setActiveFile('/home/a.ts');
      editor.updateFileContent('/home/a.ts', 'edited');
      editor.markModified('/home/a.ts', true);
    });

    const dialog = deferredSaveDialog();
    writeTextFile.mockRejectedValue(new Error('disk full'));

    // Start Save As and let the dialog open; it stays open until `choose`.
    let saveAs = Promise.resolve();
    act(() => {
      saveAs = editor.saveActiveFileAs();
    });
    await act(async () => {
      await dialog.opened;
    });
    act(() => {
      editor.setActiveFile('/home/b.ts');
    });
    await act(async () => {
      dialog.choose('/home/copy.ts');
      await saveAs;
    });

    // b.ts is clean and untouched by the failure, so its chip must not read
    // "Save failed"; the failure itself still has to reach the user.
    expect(stateChip()).toBe('Saved');
    expect(alertText()).toContain('Could not save as /home/copy.ts');
    expect(alertText()).toContain('disk full');
    expect(editor.openTabs.map((tab) => tab.path)).toEqual([
      '/home/a.ts',
      '/home/b.ts',
    ]);
  });

  it('reports a Save As whose source tab was closed while the dialog was open', async () => {
    renderStatusBar();
    await openLoaded('/home/a.ts');
    await openLoaded('/home/b.ts');

    act(() => {
      editor.setActiveFile('/home/a.ts');
    });

    const dialog = deferredSaveDialog();

    // Start Save As and let the dialog open; it stays open until `choose`.
    let saveAs = Promise.resolve();
    act(() => {
      saveAs = editor.saveActiveFileAs();
    });
    await act(async () => {
      await dialog.opened;
    });
    act(() => {
      editor.closeTab('/home/a.ts');
    });
    await act(async () => {
      dialog.choose('/home/copy.ts');
      await saveAs;
    });

    expect(writeTextFile).not.toHaveBeenCalled();
    expect(alertText()).toContain(
      '/home/a.ts was closed before the save completed',
    );
    expect(stateChip()).toBe('Saved');
  });
});
