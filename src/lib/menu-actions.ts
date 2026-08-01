/**
 * The single registry of menu actions qedit can actually perform. The menu bar
 * and command palette both grey out anything missing from it, so an enabled
 * command never silently does nothing. Adding a handler means adding it here.
 */

/** Actions handled by the editor shell (`runMenuAction` in the index route). */
export const SHELL_MENU_ACTIONS: ReadonlySet<string> = new Set([
  'app.preferences',
  'help.openLogs',
  'file.new',
  'file.open',
  'file.openFolder',
  'file.save',
  'file.saveAs',
  'file.close',
  'file.closeAll',
  'file.reopen',
  'file.reload',
  'edit.find',
  'view.toggleSidebar',
  'view.toggleStatusBar',
  'view.toggleTerminal',
  'view.commandPalette',
  'view.quickOpen',
  'view.toggleMinimap',
  'view.toggleWordWrap',
  'appearance.dark',
  'appearance.light',
  'appearance.system',
  'terminal.new',
  'terminal.focus',
  'terminal.openEditor',
  'terminal.next',
  'terminal.previous',
  'terminal.close',
]);

/** Actions forwarded to Monaco, keyed by the command the editor triggers. */
export const EDITOR_MENU_COMMANDS: Readonly<Record<string, string>> = {
  'edit.undo': 'undo',
  'edit.redo': 'redo',
  'edit.cut': 'editor.action.clipboardCutAction',
  'edit.copy': 'editor.action.clipboardCopyAction',
  'edit.paste': 'editor.action.clipboardPasteAction',
  'edit.selectAll': 'editor.action.selectAll',
  'edit.replace': 'editor.action.startFindReplaceAction',
  'edit.expandSelection': 'editor.action.smartSelect.expand',
  'edit.shrinkSelection': 'editor.action.smartSelect.shrink',
  'selection.cursorAbove': 'editor.action.insertCursorAbove',
  'selection.cursorBelow': 'editor.action.insertCursorBelow',
  'selection.addNext': 'editor.action.addSelectionToNextFindMatch',
  'selection.addPrevious': 'editor.action.addSelectionToPreviousFindMatch',
  'selection.selectAll': 'editor.action.selectHighlights',
  'selection.copyLineUp': 'editor.action.copyLinesUpAction',
  'selection.copyLineDown': 'editor.action.copyLinesDownAction',
  'selection.moveLineUp': 'editor.action.moveLinesUpAction',
  'selection.moveLineDown': 'editor.action.moveLinesDownAction',
  'selection.duplicate': 'editor.action.duplicateSelection',
  'selection.deleteLine': 'editor.action.deleteLines',
  'go.line': 'editor.action.gotoLine',
  'go.symbol': 'editor.action.quickOutline',
  'go.definition': 'editor.action.revealDefinition',
  'go.declaration': 'editor.action.revealDeclaration',
  'go.typeDefinition': 'editor.action.goToTypeDefinition',
  'go.lastEdit': 'cursorUndo',
  'go.nextProblem': 'editor.action.marker.next',
  'go.previousProblem': 'editor.action.marker.prev',
};

export function isMenuActionAvailable(action: string): boolean {
  return (
    SHELL_MENU_ACTIONS.has(action) ||
    Object.hasOwn(EDITOR_MENU_COMMANDS, action)
  );
}
