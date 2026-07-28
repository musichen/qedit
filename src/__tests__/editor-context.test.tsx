import { renderHook, act, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import { EditorProvider, useEditor } from '../components/EditorContext';

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

const pending = <T,>(): Promise<T> => new Promise<T>(() => {});

const wrapper = ({ children }: { children: ReactNode }) => (
  <EditorProvider>{children}</EditorProvider>
);

beforeEach(() => {
  readTextFile.mockReset();
  writeTextFile.mockReset();
  saveDialog.mockReset();
  saveDialog.mockResolvedValue(null);
  // Reads hang unless a test opts into a resolved or rejected read, so
  // every file starts in a deterministic 'loading' state.
  readTextFile.mockImplementation(() => pending<string>());
  writeTextFile.mockResolvedValue(undefined);
});

describe('EditorContext', () => {
  describe('openFile', () => {
    it('adds a new tab when opening a file', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      expect(result.current.openTabs).toHaveLength(1);
      expect(result.current.openTabs[0]?.path).toBe('/home/test.ts');
      expect(result.current.openTabs[0]?.name).toBe('test.ts');
      expect(result.current.activeFilePath).toBe('/home/test.ts');
    });

    it('does not duplicate an already-open tab', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      expect(result.current.openTabs).toHaveLength(1);
    });

    it('switches active file to the newly opened file', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/a.ts', 'a.ts');
        result.current.openFile('/home/b.ts', 'b.ts');
      });

      expect(result.current.activeFilePath).toBe('/home/b.ts');
      expect(result.current.openTabs).toHaveLength(2);
    });

    it('caches the file contents once the read resolves', async () => {
      readTextFile.mockResolvedValue('file body');

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')).toEqual({
          kind: 'loaded',
        });
      });
      expect(result.current.fileContents.get('/home/test.ts')).toBe(
        'file body',
      );
    });

    it('ignores a stale read that resolves after the tab was closed and reopened', async () => {
      const resolvers: Array<(content: string) => void> = [];
      readTextFile.mockImplementation(
        () => new Promise<string>((resolve) => resolvers.push(resolve)),
      );

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/big.ts', 'big.ts');
      });

      await waitFor(() => expect(resolvers).toHaveLength(1));

      act(() => {
        expect(result.current.closeTab('/home/big.ts')).toBe(true);
      });

      act(() => {
        result.current.openFile('/home/big.ts', 'big.ts');
      });

      await waitFor(() => expect(resolvers).toHaveLength(2));

      await act(async () => {
        resolvers[1]?.('current disk content');
        resolvers[0]?.('stale buffer');
      });

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/big.ts')).toEqual({
          kind: 'loaded',
        });
      });
      expect(result.current.fileContents.get('/home/big.ts')).toBe(
        'current disk content',
      );
    });

    it('records an error status when the read rejects', async () => {
      readTextFile.mockRejectedValue(new Error('permission denied'));

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')).toEqual({
          kind: 'error',
          message: 'Could not read /home/test.ts: permission denied',
        });
      });
      expect(result.current.fileContents.has('/home/test.ts')).toBe(false);
    });
  });

  describe('closeTab', () => {
    it('removes the tab from the list', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/a.ts', 'a.ts');
        result.current.openFile('/home/b.ts', 'b.ts');
      });

      act(() => {
        result.current.closeTab('/home/a.ts');
      });

      expect(result.current.openTabs).toHaveLength(1);
      expect(result.current.openTabs[0]?.path).toBe('/home/b.ts');
    });

    it('sets active file to null when closing the last tab', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/a.ts', 'a.ts');
      });

      act(() => {
        result.current.closeTab('/home/a.ts');
      });

      expect(result.current.openTabs).toHaveLength(0);
      expect(result.current.activeFilePath).toBeNull();
    });
  });

  describe('setActiveFile', () => {
    it('changes the active file path', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/a.ts', 'a.ts');
        result.current.openFile('/home/b.ts', 'b.ts');
      });

      act(() => {
        result.current.setActiveFile('/home/a.ts');
      });

      expect(result.current.activeFilePath).toBe('/home/a.ts');
    });
  });

  describe('dirty tab protection and history', () => {
    it('keeps a dirty tab open when close is cancelled', () => {
      const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
        result.current.markModified('/home/test.ts', true);
      });

      act(() => {
        expect(result.current.closeTab('/home/test.ts')).toBe(false);
      });

      expect(result.current.openTabs).toHaveLength(1);
      expect(confirm).toHaveBeenCalledWith(
        'test.ts has unsaved changes. Close it and discard them?',
      );
      confirm.mockRestore();
    });

    it('reopens the last cleanly closed tab', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });
      act(() => {
        expect(result.current.closeTab('/home/test.ts')).toBe(true);
      });
      act(() => {
        result.current.reopenLastClosedTab();
      });

      expect(result.current.openTabs[0]?.path).toBe('/home/test.ts');
      expect(result.current.activeFilePath).toBe('/home/test.ts');
    });
  });

  describe('markModified', () => {
    it('marks a file as modified', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      act(() => {
        result.current.markModified('/home/test.ts', true);
      });

      const tab = result.current.openTabs.find(
        (t) => t.path === '/home/test.ts',
      );
      expect(tab?.isModified).toBe(true);
    });

    it('clears modified state', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
        result.current.markModified('/home/test.ts', true);
      });

      act(() => {
        result.current.markModified('/home/test.ts', false);
      });

      const tab = result.current.openTabs.find(
        (t) => t.path === '/home/test.ts',
      );
      expect(tab?.isModified).toBe(false);
    });
  });

  describe('updateFileContent', () => {
    it('does not update content for a file that is still loading', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      act(() => {
        result.current.updateFileContent('/home/test.ts', 'new content');
      });

      expect(result.current.fileStatus.get('/home/test.ts')).toEqual({
        kind: 'loading',
      });
      expect(result.current.fileContents.get('/home/test.ts')).toBeUndefined();
    });

    it('updates content once the file is loaded', async () => {
      readTextFile.mockResolvedValue('original');

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')).toEqual({
          kind: 'loaded',
        });
      });

      act(() => {
        result.current.updateFileContent('/home/test.ts', 'edited');
      });

      expect(result.current.fileContents.get('/home/test.ts')).toBe('edited');
    });
  });

  describe('saveActiveFile', () => {
    it('does nothing when no file is active', async () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      await act(async () => {
        await result.current.saveActiveFile();
      });

      expect(result.current.saveError).toBeNull();
      expect(writeTextFile).not.toHaveBeenCalled();
    });

    it('reports error when trying to save a still-loading file', async () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      await act(async () => {
        await result.current.saveActiveFile();
      });

      expect(result.current.saveError).toContain('still loading');
      expect(writeTextFile).not.toHaveBeenCalled();
    });

    it('refuses to save a file whose read failed', async () => {
      readTextFile.mockRejectedValue(new Error('permission denied'));

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')?.kind).toBe(
          'error',
        );
      });

      await act(async () => {
        await result.current.saveActiveFile();
      });

      expect(result.current.saveError).toContain('was never read successfully');
      expect(writeTextFile).not.toHaveBeenCalled();
    });

    it('writes the buffer and clears the dirty flag on success', async () => {
      readTextFile.mockResolvedValue('original');

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')?.kind).toBe(
          'loaded',
        );
      });

      act(() => {
        result.current.updateFileContent('/home/test.ts', 'edited');
        result.current.markModified('/home/test.ts', true);
      });

      await act(async () => {
        await result.current.saveActiveFile();
      });

      expect(writeTextFile).toHaveBeenCalledWith('/home/test.ts', 'edited');
      expect(result.current.saveError).toBeNull();
      expect(result.current.openTabs[0]?.isModified).toBe(false);
    });

    it('saves through Save As and moves the active tab to the new path', async () => {
      readTextFile.mockResolvedValue('original');
      saveDialog.mockResolvedValue('/home/renamed.ts');

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });
      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')?.kind).toBe(
          'loaded',
        );
      });

      act(() => {
        result.current.updateFileContent('/home/test.ts', 'renamed');
        result.current.markModified('/home/test.ts', true);
      });
      await act(async () => {
        await result.current.saveActiveFileAs();
      });

      expect(saveDialog).toHaveBeenCalledWith(
        expect.objectContaining({ defaultPath: '/home/test.ts' }),
      );
      expect(writeTextFile).toHaveBeenCalledWith('/home/renamed.ts', 'renamed');
      expect(result.current.activeFilePath).toBe('/home/renamed.ts');
      expect(result.current.openTabs[0]).toEqual({
        path: '/home/renamed.ts',
        name: 'renamed.ts',
        isModified: false,
      });
    });

    it('rejects a Save As destination outside the home directory', async () => {
      readTextFile.mockResolvedValue('original');
      saveDialog.mockResolvedValue('/tmp/renamed.ts');

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });
      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')?.kind).toBe(
          'loaded',
        );
      });

      await act(async () => {
        await result.current.saveActiveFileAs();
      });

      expect(result.current.saveError).toContain('only open files and folders');
      expect(writeTextFile).not.toHaveBeenCalled();
      expect(result.current.activeFilePath).toBe('/home/test.ts');
    });

    it('keeps the dirty flag and reports the error when the write fails', async () => {
      readTextFile.mockResolvedValue('original');
      writeTextFile.mockRejectedValue(new Error('disk full'));

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')?.kind).toBe(
          'loaded',
        );
      });

      act(() => {
        result.current.updateFileContent('/home/test.ts', 'edited');
        result.current.markModified('/home/test.ts', true);
      });

      await act(async () => {
        await result.current.saveActiveFile();
      });

      expect(result.current.saveError).toContain('disk full');
      expect(result.current.openTabs[0]?.isModified).toBe(true);
    });

    it('clears a previous save failure when the file is reloaded', async () => {
      readTextFile.mockResolvedValue('original');
      writeTextFile.mockRejectedValue(new Error('disk full'));

      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')?.kind).toBe(
          'loaded',
        );
      });

      act(() => {
        result.current.updateFileContent('/home/test.ts', 'edited');
        result.current.markModified('/home/test.ts', true);
      });

      await act(async () => {
        await result.current.saveActiveFile();
      });

      expect(result.current.saveError).toContain('disk full');

      const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);

      act(() => {
        result.current.reloadActiveFile();
      });

      confirm.mockRestore();

      await waitFor(() => {
        expect(result.current.fileStatus.get('/home/test.ts')?.kind).toBe(
          'loaded',
        );
      });

      expect(result.current.saveError).toBeNull();
      expect(result.current.openTabs[0]?.isModified).toBe(false);
    });
  });
});
