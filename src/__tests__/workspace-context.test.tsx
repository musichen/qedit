import { render, act, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';

import { EditorProvider, useEditor } from '../components/EditorContext';
import {
  WorkspaceProvider,
  useWorkspace,
} from '../components/WorkspaceContext';

import type { WorkspaceEntry } from '#/lib/workspace-bridge';

const openNativeFolder = vi.fn<() => Promise<string | null>>();
const readWorkspaceDirectory =
  vi.fn<(root: string) => Promise<WorkspaceEntry[]>>();
const createNativeFile = vi.fn<(root: string) => Promise<string | null>>();
const readWorkspaceFiles = vi.fn<(root: string) => Promise<WorkspaceEntry[]>>();
const saveNativeFile =
  vi.fn<(defaultPath?: string) => Promise<string | null>>();
const renameNativeFile =
  vi.fn<(oldPath: string, newPath: string) => Promise<void>>();
const removeNativeFile = vi.fn<(path: string) => Promise<void>>();

vi.mock('#/lib/workspace-bridge', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('#/lib/workspace-bridge')>();

  return {
    ...actual,
    openNativeFile: () => Promise.resolve(null),
    openNativeFolder: () => openNativeFolder(),
    readWorkspaceDirectory: (root: string) => readWorkspaceDirectory(root),
    createNativeFile: (root: string) => createNativeFile(root),
    readWorkspaceFiles: (root: string) => readWorkspaceFiles(root),
    saveNativeFile: (defaultPath?: string) => saveNativeFile(defaultPath),
    renameNativeFile: (oldPath: string, newPath: string) =>
      renameNativeFile(oldPath, newPath),
    removeNativeFile: (path: string) => removeNativeFile(path),
    getNativeHomeDirectory: () => Promise.resolve('/home'),
  };
});

vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: () => Promise.resolve('contents'),
  writeTextFile: () => Promise.resolve(),
  exists: () => Promise.resolve(false),
}));

const file = (path: string): WorkspaceEntry => ({
  name: path.split('/').pop() as string,
  path,
  isFile: true,
  isDirectory: false,
});

let workspace: ReturnType<typeof useWorkspace>;
let editor: ReturnType<typeof useEditor>;

function Handle() {
  workspace = useWorkspace();
  editor = useEditor();

  return null;
}

const wrapper = ({ children }: { children: ReactNode }) => (
  <EditorProvider>
    <WorkspaceProvider>
      {children}
      <Handle />
    </WorkspaceProvider>
  </EditorProvider>
);

const renderWorkspace = () => render(wrapper({ children: null }));

beforeEach(() => {
  openNativeFolder.mockReset();
  readWorkspaceDirectory.mockReset();
  createNativeFile.mockReset();
  readWorkspaceFiles.mockReset();
  saveNativeFile.mockReset();
  saveNativeFile.mockResolvedValue(null);
  renameNativeFile.mockReset();
  removeNativeFile.mockReset();
  renameNativeFile.mockResolvedValue(undefined);
  removeNativeFile.mockResolvedValue(undefined);
});

describe('WorkspaceContext', () => {
  it('keeps the open tabs when the chosen folder cannot be read', async () => {
    openNativeFolder.mockResolvedValue('/home/broken');
    readWorkspaceDirectory.mockRejectedValue(new Error('folder is gone'));

    renderWorkspace();

    act(() => {
      editor.openFile('/home/keep.ts', 'keep.ts');
    });
    await waitFor(() => {
      expect(editor.fileStatus.get('/home/keep.ts')?.kind).toBe('loaded');
    });

    await act(async () => {
      await workspace.openFolderDialog();
    });

    // A folder that cannot be read must not cost the user their buffers.
    expect(editor.openTabs.map((tab) => tab.path)).toEqual(['/home/keep.ts']);
    expect(workspace.workspaceRoot).toBeNull();
    expect(workspace.error).toContain('folder is gone');
  });

  it('requests the sidebar to open when a folder becomes the workspace', async () => {
    const openSidebar = vi.fn();
    window.addEventListener('qedit:open-sidebar', openSidebar);
    openNativeFolder.mockResolvedValue('/home/project');
    readWorkspaceDirectory.mockResolvedValue([file('/home/project/main.md')]);

    try {
      renderWorkspace();
      await act(async () => {
        await workspace.openFolderDialog();
      });

      expect(openSidebar).toHaveBeenCalledTimes(1);
      expect(workspace.workspaceRoot).toBe('/home/project');
    } finally {
      window.removeEventListener('qedit:open-sidebar', openSidebar);
    }
  });

  it('ignores a folder expansion that belongs to a previous workspace', async () => {
    openNativeFolder.mockResolvedValue('/home/second');
    readWorkspaceDirectory.mockResolvedValue([file('/home/second/main.ts')]);

    renderWorkspace();

    await act(async () => {
      await workspace.openFolderDialog();
    });
    expect(workspace.workspaceRoot).toBe('/home/second');

    // A tree expansion from the workspace the user just left resolves late.
    act(() => {
      workspace.registerEntries([file('/home/first/stale.ts')], '/home/first');
    });
    expect(workspace.knownFiles.map((entry) => entry.path)).toEqual([
      '/home/second/main.ts',
    ]);

    act(() => {
      workspace.registerEntries(
        [file('/home/second/nested/util.ts')],
        '/home/second',
      );
    });
    expect(workspace.knownFiles.map((entry) => entry.path)).toEqual([
      '/home/second/main.ts',
      '/home/second/nested/util.ts',
    ]);
  });

  it('creates a file and refreshes the workspace before opening it', async () => {
    openNativeFolder.mockResolvedValue('/home/project');
    readWorkspaceDirectory
      .mockResolvedValueOnce([file('/home/project/main.ts')])
      .mockResolvedValueOnce([
        file('/home/project/main.ts'),
        file('/home/project/new.md'),
      ]);
    createNativeFile.mockResolvedValue('/home/project/new.md');

    renderWorkspace();
    await act(async () => {
      await workspace.openFolderDialog();
    });
    await act(async () => {
      await workspace.createFile();
    });

    expect(readWorkspaceDirectory).toHaveBeenCalledTimes(2);
    expect(workspace.rootEntries.map((entry) => entry.path)).toEqual([
      '/home/project/main.ts',
      '/home/project/new.md',
    ]);
    expect(editor.activeFilePath).toBe('/home/project/new.md');
  });

  it('discovers nested files for workspace search', async () => {
    openNativeFolder.mockResolvedValue('/home/project');
    readWorkspaceDirectory.mockResolvedValue([file('/home/project/main.ts')]);
    readWorkspaceFiles.mockResolvedValue([
      file('/home/project/main.ts'),
      file('/home/project/docs/readme.md'),
    ]);

    renderWorkspace();
    await act(async () => {
      await workspace.openFolderDialog();
    });
    await waitFor(() => expect(workspace.workspaceRoot).toBe('/home/project'));
    await act(async () => {
      await workspace.discoverWorkspaceFiles();
    });

    await waitFor(() =>
      expect(workspace.knownFiles.map((entry) => entry.path)).toContain(
        '/home/project/docs/readme.md',
      ),
    );
  });

  it('refreshes the tree when Save As creates a file', async () => {
    openNativeFolder.mockResolvedValue('/home/project');
    readWorkspaceDirectory
      .mockResolvedValueOnce([file('/home/project/main.ts')])
      .mockResolvedValueOnce([
        file('/home/project/main.ts'),
        file('/home/project/copy.ts'),
      ]);
    saveNativeFile.mockResolvedValue('/home/project/copy.ts');

    renderWorkspace();
    await act(async () => {
      await workspace.openFolderDialog();
    });
    await waitFor(() => expect(workspace.workspaceRoot).toBe('/home/project'));

    act(() => editor.openFile('/home/project/main.ts', 'main.ts'));
    await waitFor(() =>
      expect(editor.fileStatus.get('/home/project/main.ts')?.kind).toBe(
        'loaded',
      ),
    );
    await act(async () => {
      await editor.saveActiveFileAs();
    });

    await waitFor(() =>
      expect(workspace.rootEntries.map((entry) => entry.path)).toContain(
        '/home/project/copy.ts',
      ),
    );
  });

  it('refreshes the tree after rename and delete', async () => {
    openNativeFolder.mockResolvedValue('/home/project');
    readWorkspaceDirectory
      .mockResolvedValueOnce([file('/home/project/a.md')])
      .mockResolvedValueOnce([file('/home/project/b.md')])
      .mockResolvedValueOnce([]);

    renderWorkspace();
    await act(async () => {
      await workspace.openFolderDialog();
    });
    await waitFor(() => expect(workspace.workspaceRoot).toBe('/home/project'));

    await act(async () => {
      await workspace.renameFile('/home/project/a.md', 'b.md');
    });
    expect(editor.openTabs).toHaveLength(0);
    expect(workspace.rootEntries.map((entry) => entry.path)).toEqual([
      '/home/project/b.md',
    ]);

    await act(async () => {
      await workspace.deleteFile('/home/project/b.md');
    });
    expect(workspace.rootEntries).toEqual([]);
  });
});
