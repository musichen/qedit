export interface TerminalOutputEvent {
  sessionId: number;
  data: string;
}

export interface TerminalExitEvent {
  sessionId: number;
  code: number | null;
}

export type Unlisten = () => void;

export async function spawnTerminal(
  cwd: string,
  cols: number,
  rows: number,
): Promise<number> {
  const { invoke } = await import('@tauri-apps/api/core');

  return invoke<number>('terminal_spawn', { cwd, cols, rows });
}

export async function writeTerminal(
  sessionId: number,
  data: string,
): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');

  await invoke('terminal_write', { sessionId, data });
}

export async function resizeTerminal(
  sessionId: number,
  cols: number,
  rows: number,
): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');

  await invoke('terminal_resize', { sessionId, cols, rows });
}

export async function closeTerminal(sessionId: number): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');

  await invoke('terminal_close', { sessionId });
}

export async function listenTerminalOutput(
  callback: (event: TerminalOutputEvent) => void,
): Promise<Unlisten> {
  const { listen } = await import('@tauri-apps/api/event');

  return listen<TerminalOutputEvent>('terminal://output', (event) =>
    callback(event.payload),
  );
}

export async function listenTerminalExit(
  callback: (event: TerminalExitEvent) => void,
): Promise<Unlisten> {
  const { listen } = await import('@tauri-apps/api/event');

  return listen<TerminalExitEvent>('terminal://exit', (event) =>
    callback(event.payload),
  );
}
