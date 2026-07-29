import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  createNativeFile,
  removeNativeFile,
  renameNativeFile,
} from '../lib/workspace-bridge';

const save = vi.fn<(options: unknown) => Promise<string | null>>();
const exists = vi.fn<(path: string) => Promise<boolean>>();
const writeTextFile = vi.fn<(path: string, content: string) => Promise<void>>();
const rename = vi.fn<(oldPath: string, newPath: string) => Promise<void>>();
const stat = vi.fn<(path: string) => Promise<{ isFile: boolean }>>();
const remove = vi.fn<(path: string) => Promise<void>>();

vi.mock('@tauri-apps/plugin-dialog', () => ({ save }));
vi.mock('@tauri-apps/plugin-fs', () => ({
  exists,
  writeTextFile,
  rename,
  stat,
  remove,
}));
vi.mock('@tauri-apps/api/path', () => ({
  homeDir: () => Promise.resolve('/home'),
}));

beforeEach(() => {
  save.mockReset();
  exists.mockReset();
  writeTextFile.mockReset();
  rename.mockReset();
  stat.mockReset();
  remove.mockReset();
  exists.mockResolvedValue(false);
  writeTextFile.mockResolvedValue(undefined);
  rename.mockResolvedValue(undefined);
  stat.mockResolvedValue({ isFile: true });
  remove.mockResolvedValue(undefined);
});

describe('native workspace file operations', () => {
  it('creates an empty file in the selected workspace and supports cancellation', async () => {
    save.mockResolvedValueOnce('/home/project/notes.md');

    await expect(createNativeFile('/home/project')).resolves.toBe(
      '/home/project/notes.md',
    );
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'New File',
        defaultPath: '/home/project/untitled.md',
      }),
    );
    expect(writeTextFile).toHaveBeenCalledWith('/home/project/notes.md', '');

    save.mockResolvedValueOnce(null);
    await expect(createNativeFile('/home/project')).resolves.toBeNull();
    expect(writeTextFile).toHaveBeenCalledTimes(1);
  });

  it('rejects existing and outside-workspace destinations', async () => {
    save.mockResolvedValueOnce('/home/project/notes.md');
    exists.mockResolvedValueOnce(true);
    await expect(createNativeFile('/home/project')).rejects.toThrow(
      'already exists',
    );

    save.mockResolvedValueOnce('/home/other/notes.md');
    await expect(createNativeFile('/home/project')).rejects.toThrow(
      'inside the open workspace',
    );
  });

  it('keeps rename and delete operations inside the native home boundary', async () => {
    await renameNativeFile('/home/project/a.md', '/home/project/b.md');
    expect(rename).toHaveBeenCalledWith(
      '/home/project/a.md',
      '/home/project/b.md',
    );

    await removeNativeFile('/home/project/b.md');
    expect(stat).toHaveBeenCalledWith('/home/project/b.md');
    expect(remove).toHaveBeenCalledWith('/home/project/b.md');

    await expect(renameNativeFile('/tmp/a.md', '/home/b.md')).rejects.toThrow(
      'only open files and folders',
    );
    await expect(removeNativeFile('/tmp/b.md')).rejects.toThrow(
      'only open files and folders',
    );
  });
});
