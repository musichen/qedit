export type ShortcutAction =
  | 'open-file'
  | 'open-folder'
  | 'save'
  | 'save-as'
  | 'close-tab'
  | 'reopen-tab'
  | 'reload-file'
  | 'next-tab'
  | 'previous-tab'
  | 'focus-terminal'
  | 'quick-open'
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
  if (!(event.metaKey || event.ctrlKey) || isTextEditingTarget(event.target)) {
    return null;
  }

  const key = event.key.toLowerCase();

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
  if (key === '`') return 'focus-terminal';
  if (key === 'p') return 'quick-open';
  if (key === 'f') return 'find';

  return null;
}
