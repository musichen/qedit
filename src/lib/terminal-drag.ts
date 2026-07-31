export const TERMINAL_DRAG_TYPE = 'application/x-qedit-terminal';
export const TERMINAL_DRAG_TEXT_PREFIX = 'qedit-terminal:';

export function setTerminalDragData(
  dataTransfer: DataTransfer,
  id: string,
): void {
  const value = `${TERMINAL_DRAG_TEXT_PREFIX}${id}`;
  dataTransfer.setData(TERMINAL_DRAG_TYPE, id);
  dataTransfer.setData('text/qedit-terminal', id);
  dataTransfer.setData('text/plain', value);
}

export function isTerminalDragOver(dataTransfer: DataTransfer): boolean {
  const types = Array.from(dataTransfer.types);

  return (
    types.includes(TERMINAL_DRAG_TYPE) ||
    types.includes('text/qedit-terminal') ||
    types.includes('text/plain')
  );
}

export function getTerminalDragId(dataTransfer: DataTransfer): string | null {
  const typedId = dataTransfer.getData(TERMINAL_DRAG_TYPE);
  if (typedId) return typedId;

  const legacyId = dataTransfer.getData('text/qedit-terminal');
  if (legacyId) return legacyId;

  const text = dataTransfer.getData('text/plain');
  if (text.startsWith(TERMINAL_DRAG_TEXT_PREFIX)) {
    return text.slice(TERMINAL_DRAG_TEXT_PREFIX.length) || null;
  }

  return null;
}
