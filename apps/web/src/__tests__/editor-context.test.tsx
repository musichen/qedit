import { renderHook, act } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, it, expect } from 'vitest';

import { EditorProvider, useEditor } from '../components/EditorContext';

const wrapper = ({ children }: { children: ReactNode }) => (
  <EditorProvider>{children}</EditorProvider>
);

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
    it('does not update content for a file that has not been loaded', () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      act(() => {
        result.current.updateFileContent('/home/test.ts', 'new content');
      });

      // File is still "loading" since Tauri read isn't mocked,
      // so updateFileContent must be a no-op
      expect(result.current.fileContents.get('/home/test.ts')).toBeUndefined();
    });
  });

  describe('saveActiveFile', () => {
    it('does nothing when no file is active', async () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      await act(async () => {
        await result.current.saveActiveFile();
      });

      expect(result.current.saveError).toBeNull();
    });

    it('reports error when trying to save a still-loading file', async () => {
      const { result } = renderHook(() => useEditor(), { wrapper });

      act(() => {
        result.current.openFile('/home/test.ts', 'test.ts');
      });

      // File is still "loading" — save must report an error
      await act(async () => {
        await result.current.saveActiveFile();
      });

      expect(result.current.saveError).toContain('still loading');
    });
  });
});
