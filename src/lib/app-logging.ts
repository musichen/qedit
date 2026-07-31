const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Local-only diagnostics. Nothing is sent over the network. */
export async function logInfo(message: string): Promise<void> {
  if (!isTauri()) return;

  try {
    const { info } = await import('@tauri-apps/plugin-log');
    await info(`[local-only] ${message}`);
  } catch (cause) {
    console.warn('Could not write local qedit log:', cause);
  }
}

export async function logError(
  message: string,
  cause?: unknown,
): Promise<void> {
  if (!isTauri()) return;

  try {
    const { error } = await import('@tauri-apps/plugin-log');
    await error(`[local-only] ${message}${cause ? `: ${String(cause)}` : ''}`);
  } catch (logCause) {
    console.warn('Could not write local qedit log:', logCause);
  }
}

export async function openLocalLogsFolder(): Promise<string | null> {
  if (!isTauri()) return null;

  const [{ appLogDir }, { open }] = await Promise.all([
    import('@tauri-apps/api/path'),
    import('@tauri-apps/plugin-shell'),
  ]);
  const path = await appLogDir();
  await open(path);
  await logInfo(`opened local logs folder: ${path}`);

  return path;
}
