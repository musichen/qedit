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

vi.mock('#/lib/workspace-bridge', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('#/lib/workspace-bridge')>();

  return {
    ...actual,
    openNativeFile: () => Promise.resolve(null),
    openNativeFolder: () => openNativeFolder(),
    readWorkspaceDirectory: (root: string) => readWorkspaceDirectory(root),
    getNativeHomeDirectory: () => Promise.resolve('/home'),
  };
});

vi.mock('@tauri-apps/plugin-fs', () => ({
  readTextFile: () => Promise.resolve('contents'),
  writeTextFile: () => Promise.resolve(),
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
});
