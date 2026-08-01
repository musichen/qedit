export type ShortcutAction =
  | 'new-file'
  | 'open-file'
  | 'open-folder'
  | 'save'
  | 'save-as'
  | 'close-tab'
  | 'reopen-tab'
  | 'reload-file'
  | 'next-tab'
  | 'previous-tab'
  | 'close-terminal'
  | 'next-terminal'
  | 'previous-terminal'
  | 'terminal-1'
  | 'terminal-2'
  | 'terminal-3'
  | 'terminal-4'
  | 'terminal-5'
  | 'terminal-6'
  | 'terminal-7'
  | 'terminal-8'
  | 'terminal-9'
  | 'focus-terminal'
  | 'focus-editor'
  | 'quick-open'
  | 'command-palette'
  | 'toggle-sidebar'
  | 'find';

/**
 * Monaco keeps focus on a hidden `<textarea class="inputarea">`, so a naive
 * text-field guard would disable every workspace shortcut while the user is
 * editing. Editing inside Monaco is the primary flow and its shortcuts are
 * owned by qedit, so the editor is explicitly not treated as a text field.
 */
function isMonacoTarget(target: Element): boolean {
  return target.closest('.monaco-editor') !== null;
}

/**
 * Xterm owns a hidden `<textarea class="xterm-helper-textarea">`, so terminal
 * focus otherwise swallows every workspace shortcut with no keyboard way back
 * out. Only the focus binding is exempted; everything else still belongs to the
 * shell so ordinary typing is untouched.
 */
function isTerminalTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;

  return (
    target.closest('.xterm') !== null ||
    target.closest('.xterm-helper-textarea') !== null
  );
}

function isTerminalNavigationTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;

  return (
    isTerminalTarget(target) || target.closest('[data-terminal-tab]') !== null
  );
}

function isTextEditingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (isMonacoTarget(target)) return false;

  const isEditable = (element: Element): boolean =>
    element.matches('input, textarea') ||
    element.getAttribute('contenteditable') === 'true' ||
    (element as HTMLElement).contentEditable === 'true' ||
    (element as HTMLElement).isContentEditable === true;

  const editingAncestor = target.closest(
    'input, textarea, [contenteditable="true"]',
  );

  return (
    isEditable(target) ||
    (editingAncestor !== null && isEditable(editingAncestor))
  );
}

export function shortcutActionForEvent(
  event: Pick<
    KeyboardEvent,
    'key' | 'metaKey' | 'ctrlKey' | 'shiftKey' | 'target'
  >,
): ShortcutAction | null {
  if (!(event.metaKey || event.ctrlKey)) return null;

  const key = event.key.toLowerCase();

  if (isTerminalNavigationTarget(event.target)) {
    if (key === 'pagedown' || (key === 'tab' && !event.shiftKey)) {
      return 'next-terminal';
    }
    if (key === 'pageup' || (key === 'tab' && event.shiftKey)) {
      return 'previous-terminal';
    }
    if (/^[1-9]$/.test(key)) {
      return `terminal-${key}` as ShortcutAction;
    }
    if (key === 'w' && !isTerminalTarget(event.target)) {
      return 'close-terminal';
    }
  }

  if (key === '`') {
    return isTerminalTarget(event.target) ? 'focus-editor' : 'focus-terminal';
  }

  if (isTextEditingTarget(event.target)) return null;

  if (key === 'n') return 'new-file';
  if (key === 'o') return event.shiftKey ? 'open-folder' : 'open-file';
  if (key === 's') return event.shiftKey ? 'save-as' : 'save';
  if (key === 'w') return 'close-tab';
  if (key === 't' && event.shiftKey) return 'reopen-tab';
  if (key === 'r' && event.shiftKey) return 'reload-file';
  if (key === 'pagedown' || (key === 'tab' && event.shiftKey === false)) {
    return 'next-tab';
  }
  if (key === 'pageup' || (key === 'tab' && event.shiftKey)) {
    return 'previous-tab';
  }
  if (key === 'p') return event.shiftKey ? 'command-palette' : 'quick-open';
  if (key === 'b') return 'toggle-sidebar';
  if (key === 'f') return 'find';

  return null;
}
