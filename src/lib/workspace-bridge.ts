export interface WorkspaceEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  isFile: boolean;
}

export class WorkspaceBridgeError extends Error {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, options);
    this.name = 'WorkspaceBridgeError';
  }
}

export const errorMessage = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

export async function readGitBranch(path: string): Promise<string | null> {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');

  return invoke<string | null>('git_branch', { path });
}

/**
 * Normalize a native path for boundary checks without depending on Node APIs.
 * Dialog selections are absolute paths, but this also handles Windows separators
 * and removes dot segments before comparing the home-directory boundary.
 */
export function normalizePath(path: string): string {
  const replaced = path.replaceAll('\\', '/');
  const isWindowsDrive = /^[A-Za-z]:\//.test(replaced);
  const prefix = replaced.startsWith('/')
    ? '/'
    : isWindowsDrive
      ? replaced.slice(0, 3)
      : '';
  const body = isWindowsDrive ? replaced.slice(3) : replaced;
  const segments: string[] = [];

  for (const segment of body.split('/')) {
    if (!segment || segment === '.') continue;
    if (segment === '..') {
      if (segments.length > 0) segments.pop();

      continue;
    }
    segments.push(segment);
  }

  const normalized = `${prefix}${segments.join('/')}`;

  if (normalized.length > 1 && normalized.endsWith('/')) {
    return normalized.slice(0, -1);
  }

  return normalized || prefix || '.';
}

export function isPathWithinHome(path: string, home: string): boolean {
  const normalizedPath = normalizePath(path);
  const normalizedHome = normalizePath(home);
  const comparePath = /^[A-Za-z]:\//.test(normalizedHome)
    ? normalizedPath.toLowerCase()
    : normalizedPath;
  const compareHome = /^[A-Za-z]:\//.test(normalizedHome)
    ? normalizedHome.toLowerCase()
    : normalizedHome;

  return (
    comparePath === compareHome || comparePath.startsWith(`${compareHome}/`)
  );
}

export function isHiddenOrIgnored(name: string): boolean {
  return (
    name.startsWith('.') ||
    ['node_modules', 'target', 'dist', '.turbo'].includes(name)
  );
}

function sortEntries(a: WorkspaceEntry, b: WorkspaceEntry): number {
  if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;

  return a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
}

async function selectedPath(options: {
  directory: boolean;
  title: string;
}): Promise<string | null> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const result = await open({
      title: options.title,
      directory: options.directory,
      multiple: false,
    });

    // Some macOS native dialog versions return a one-item array even when
    // `multiple: false`; otherwise Open Folder silently becomes a no-op.
    if (typeof result === 'string') return result;

    return result?.[0] ?? null;
  } catch (cause) {
    throw new WorkspaceBridgeError(
      `Could not open ${options.directory ? 'folder' : 'file'} picker: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

async function assertHomePath(path: string): Promise<string> {
  try {
    const { homeDir } = await import('@tauri-apps/api/path');
    const home = await homeDir();

    if (!isPathWithinHome(path, home)) {
      throw new WorkspaceBridgeError(
        'For safety, qedit can only open files and folders in your home directory.',
      );
    }

    return path;
  } catch (cause) {
    if (cause instanceof WorkspaceBridgeError) throw cause;

    throw new WorkspaceBridgeError(
      `Could not verify the selected path: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

export async function openNativeFile(): Promise<string | null> {
  const path = await selectedPath({ directory: false, title: 'Open File' });

  return path ? assertHomePath(path) : null;
}

export async function openNativeFolder(): Promise<string | null> {
  const path = await selectedPath({
    directory: true,
    title: 'Open Folder or Project',
  });

  return path ? assertHomePath(path) : null;
}

export async function saveNativeFile(
  defaultPath?: string,
  title = 'Save File As',
): Promise<string | null> {
  try {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const result = await save({
      title,
      defaultPath,
    });

    return result ? assertHomePath(result) : null;
  } catch (cause) {
    if (cause instanceof WorkspaceBridgeError) throw cause;

    throw new WorkspaceBridgeError(
      `Could not open save dialog: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

function assertWorkspacePath(path: string, workspaceRoot: string): string {
  if (!isPathWithinHome(path, workspaceRoot)) {
    throw new WorkspaceBridgeError(
      'For safety, new files must stay inside the open workspace.',
    );
  }

  if (normalizePath(path) === normalizePath(workspaceRoot)) {
    throw new WorkspaceBridgeError('Choose a file name inside the workspace.');
  }

  return path;
}

/** Pick and create a new empty file inside the currently open workspace. */
export async function createNativeFile(
  workspaceRoot: string,
): Promise<string | null> {
  const selected = await saveNativeFile(
    `${workspaceRoot}/untitled.md`,
    'New File',
  );

  if (!selected) return null;

  const path = assertWorkspacePath(selected, workspaceRoot);

  try {
    const { exists } = await import('@tauri-apps/plugin-fs');

    if (await exists(path)) {
      throw new WorkspaceBridgeError(
        `A file named ${basenameFromPath(path)} already exists.`,
      );
    }

    const { writeTextFile } = await import('@tauri-apps/plugin-fs');
    await writeTextFile(path, '');

    return path;
  } catch (cause) {
    if (cause instanceof WorkspaceBridgeError) throw cause;

    throw new WorkspaceBridgeError(
      `Could not create ${path}: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

export async function renameNativeFile(
  oldPath: string,
  newPath: string,
): Promise<void> {
  await assertHomePath(oldPath);
  await assertHomePath(newPath);

  try {
    const { exists, rename } = await import('@tauri-apps/plugin-fs');

    if (
      normalizePath(oldPath) !== normalizePath(newPath) &&
      (await exists(newPath))
    ) {
      throw new WorkspaceBridgeError(
        `A file named ${basenameFromPath(newPath)} already exists.`,
      );
    }

    await rename(oldPath, newPath);
  } catch (cause) {
    if (cause instanceof WorkspaceBridgeError) throw cause;

    throw new WorkspaceBridgeError(
      `Could not rename ${oldPath}: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

export async function removeNativeFile(filePath: string): Promise<void> {
  await assertHomePath(filePath);

  try {
    const { stat, remove } = await import('@tauri-apps/plugin-fs');
    const info = await stat(filePath);

    if (!info.isFile) {
      throw new WorkspaceBridgeError('Only files can be deleted from qedit.');
    }

    await remove(filePath);
  } catch (cause) {
    if (cause instanceof WorkspaceBridgeError) throw cause;

    throw new WorkspaceBridgeError(
      `Could not delete ${filePath}: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

export async function getNativeHomeDirectory(): Promise<string | null> {
  try {
    const { homeDir } = await import('@tauri-apps/api/path');

    return await homeDir();
  } catch {
    return null;
  }
}

export async function readWorkspaceDirectory(
  directoryPath: string,
): Promise<WorkspaceEntry[]> {
  try {
    const { readDir } = await import('@tauri-apps/plugin-fs');
    const { join } = await import('@tauri-apps/api/path');
    const entries = await readDir(directoryPath);
    const result: WorkspaceEntry[] = [];

    for (const entry of entries) {
      if (!entry.name || isHiddenOrIgnored(entry.name) || entry.isSymlink) {
        continue;
      }

      const isDirectory = entry.isDirectory === true;
      const isFile = entry.isFile === true || !isDirectory;

      result.push({
        name: entry.name,
        path: await join(directoryPath, entry.name),
        isDirectory,
        isFile,
      });
    }

    return result.sort(sortEntries);
  } catch (cause) {
    throw new WorkspaceBridgeError(
      `Could not read ${directoryPath}: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

/** Recursively discover visible files for workspace search and Quick Open. */
export async function readWorkspaceFiles(
  directoryPath: string,
): Promise<WorkspaceEntry[]> {
  const entries = await readWorkspaceDirectory(directoryPath);
  const files: WorkspaceEntry[] = [];

  for (const entry of entries) {
    if (entry.isFile) files.push(entry);
    if (entry.isDirectory) {
      files.push(...(await readWorkspaceFiles(entry.path)));
    }
  }

  return files;
}

export async function readNativeTextFile(filePath: string): Promise<string> {
  try {
    const { readTextFile } = await import('@tauri-apps/plugin-fs');

    return await readTextFile(filePath);
  } catch (cause) {
    throw new WorkspaceBridgeError(
      `Could not read ${filePath}: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

export async function writeNativeTextFile(
  filePath: string,
  content: string,
): Promise<void> {
  try {
    const { writeTextFile } = await import('@tauri-apps/plugin-fs');
    await writeTextFile(filePath, content);
  } catch (cause) {
    throw new WorkspaceBridgeError(
      `Could not write ${filePath}: ${errorMessage(cause)}`,
      { cause },
    );
  }
}

export function basenameFromPath(path: string): string {
  const normalized = path.replaceAll('\\', '/');

  return normalized.split('/').pop() || normalized;
}

/** Containing directory of a path, used as the secondary label next to a filename. */
export function dirnameFromPath(path: string): string {
  const normalized = path.replaceAll('\\', '/').replace(/\/+$/, '');
  const separator = normalized.lastIndexOf('/');

  if (separator < 0) return '';
  if (separator === 0) return '/';

  return normalized.slice(0, separator);
}
