import { isMenuActionAvailable } from './menu-actions';

export interface MenuItem {
  label: string;
  action?: string;
  shortcut?: string;
  disabled?: boolean;
  submenu?: MenuItem[];
  separator?: boolean;
}

export interface MenuDefinition {
  label: string;
  items: MenuItem[];
}

const separator = (): MenuItem => ({ label: '', separator: true });

/* The menu model is intentionally data-first so the menu bar and command
 * palette always expose the same commands. Native actions are dispatched by
 * the editor shell; unavailable actions remain visible and disabled. */
export const MENU_DATA: MenuDefinition[] = [
  {
    label: 'qedit',
    items: [
      { label: 'About qedit', action: 'app.about' },
      separator(),
      { label: 'Preferences…', action: 'app.preferences', shortcut: '⌘,' },
      separator(),
      { label: 'Hide qedit', action: 'app.hide', shortcut: '⌘H' },
      { label: 'Hide Others', action: 'app.hideOthers', shortcut: '⌥⌘H' },
      { label: 'Show All', action: 'app.showAll' },
      separator(),
      { label: 'Quit qedit', action: 'app.quit', shortcut: '⌘Q' },
    ],
  },
  {
    label: 'File',
    items: [
      { label: 'New File', action: 'file.new', shortcut: '⌘N' },
      { label: 'Open File…', action: 'file.open', shortcut: '⌘O' },
      { label: 'Open Folder…', action: 'file.openFolder', shortcut: '⇧⌘O' },
      {
        label: 'Open Recent',
        submenu: [
          { label: 'No Recent Files', disabled: true },
          separator(),
          { label: 'Clear Recently Opened', action: 'file.clearRecent' },
        ],
      },
      separator(),
      { label: 'Save', action: 'file.save', shortcut: '⌘S' },
      { label: 'Save As…', action: 'file.saveAs', shortcut: '⇧⌘S' },
      { label: 'Save All', action: 'file.saveAll', shortcut: '⌥⌘S' },
      separator(),
      { label: 'Close', action: 'file.close', shortcut: '⌘W' },
      { label: 'Close All', action: 'file.closeAll', shortcut: '⇧⌘W' },
      { label: 'Reopen Closed Editor', action: 'file.reopen', shortcut: '⇧⌘T' },
      { label: 'Revert File', action: 'file.reload' },
    ],
  },
  {
    label: 'Edit',
    items: [
      { label: 'Undo', action: 'edit.undo', shortcut: '⌘Z' },
      { label: 'Redo', action: 'edit.redo', shortcut: '⇧⌘Z' },
      separator(),
      { label: 'Cut', action: 'edit.cut', shortcut: '⌘X' },
      { label: 'Copy', action: 'edit.copy', shortcut: '⌘C' },
      { label: 'Paste', action: 'edit.paste', shortcut: '⌘V' },
      { label: 'Paste and Match Style', action: 'edit.pasteMatchStyle' },
      separator(),
      { label: 'Select All', action: 'edit.selectAll', shortcut: '⌘A' },
      {
        label: 'Expand Selection',
        action: 'edit.expandSelection',
        shortcut: '⌥⇧→',
      },
      {
        label: 'Shrink Selection',
        action: 'edit.shrinkSelection',
        shortcut: '⌥⇧←',
      },
      separator(),
      { label: 'Find', action: 'edit.find', shortcut: '⌘F' },
      { label: 'Find and Replace', action: 'edit.replace', shortcut: '⌥⌘F' },
      { label: 'Find in Files', action: 'edit.findInFiles', shortcut: '⇧⌘F' },
    ],
  },
  {
    label: 'Selection',
    items: [
      {
        label: 'Add Cursor Above',
        action: 'selection.cursorAbove',
        shortcut: '⌥⌘↑',
      },
      {
        label: 'Add Cursor Below',
        action: 'selection.cursorBelow',
        shortcut: '⌥⌘↓',
      },
      {
        label: 'Add Next Occurrence',
        action: 'selection.addNext',
        shortcut: '⌘D',
      },
      { label: 'Add Previous Occurrence', action: 'selection.addPrevious' },
      {
        label: 'Select All Occurrences',
        action: 'selection.selectAll',
        shortcut: '⇧⌘L',
      },
      separator(),
      {
        label: 'Copy Line Up',
        action: 'selection.copyLineUp',
        shortcut: '⇧⌥↑',
      },
      {
        label: 'Copy Line Down',
        action: 'selection.copyLineDown',
        shortcut: '⇧⌥↓',
      },
      { label: 'Move Line Up', action: 'selection.moveLineUp', shortcut: '⌥↑' },
      {
        label: 'Move Line Down',
        action: 'selection.moveLineDown',
        shortcut: '⌥↓',
      },
      { label: 'Duplicate Selection', action: 'selection.duplicate' },
      { label: 'Delete Line', action: 'selection.deleteLine', shortcut: '⇧⌘K' },
    ],
  },
  {
    label: 'View',
    items: [
      { label: 'Toggle Sidebar', action: 'view.toggleSidebar', shortcut: '⌘B' },
      { label: 'Toggle Status Bar', action: 'view.toggleStatusBar' },
      { label: 'Toggle Terminal', action: 'view.toggleTerminal' },
      separator(),
      {
        label: 'Command Palette…',
        action: 'view.commandPalette',
        shortcut: '⇧⌘P',
      },
      { label: 'Quick Open…', action: 'view.quickOpen', shortcut: '⌘P' },
      { label: 'Toggle Minimap', action: 'view.toggleMinimap' },
      { label: 'Toggle Word Wrap', action: 'view.toggleWordWrap' },
      {
        label: 'Appearance',
        submenu: [
          { label: 'Dark', action: 'appearance.dark' },
          { label: 'Light', action: 'appearance.light' },
          { label: 'System', action: 'appearance.system' },
        ],
      },
      {
        label: 'Editor Layout',
        submenu: [
          { label: 'Single', action: 'view.layoutSingle' },
          { label: 'Split Right', action: 'view.layoutSplitRight' },
          { label: 'Split Down', action: 'view.layoutSplitDown' },
        ],
      },
      separator(),
      { label: 'Zoom In', action: 'view.zoomIn', shortcut: '⌘+' },
      { label: 'Zoom Out', action: 'view.zoomOut', shortcut: '⌘-' },
      { label: 'Reset Zoom', action: 'view.resetZoom', shortcut: '⌘0' },
    ],
  },
  {
    label: 'Go',
    items: [
      { label: 'Back', action: 'go.back', shortcut: '⌘[' },
      { label: 'Forward', action: 'go.forward', shortcut: '⌘]' },
      { label: 'Last Edit Location', action: 'go.lastEdit' },
      separator(),
      { label: 'Go to Line/Column…', action: 'go.line', shortcut: '⌘G' },
      { label: 'Go to Symbol…', action: 'go.symbol', shortcut: '⇧⌘O' },
      { label: 'Go to Definition', action: 'go.definition', shortcut: 'F12' },
      { label: 'Go to Declaration', action: 'go.declaration' },
      { label: 'Go to Type Definition', action: 'go.typeDefinition' },
      separator(),
      { label: 'Next Problem', action: 'go.nextProblem', shortcut: 'F8' },
      {
        label: 'Previous Problem',
        action: 'go.previousProblem',
        shortcut: '⇧F8',
      },
      { label: 'Next Change', action: 'go.nextChange' },
      { label: 'Previous Change', action: 'go.previousChange' },
    ],
  },
  {
    label: 'Run',
    items: [
      { label: 'Run Without Debugging', action: 'run.start', shortcut: '⌃F5' },
      { label: 'Start Debugging', action: 'run.debug', shortcut: 'F5' },
      { label: 'Stop Debugging', action: 'run.stop', shortcut: '⇧F5' },
      separator(),
      { label: 'Run Task…', action: 'run.task' },
      { label: 'Run Build Task…', action: 'run.build' },
      { label: 'Run Test Task…', action: 'run.test' },
      separator(),
      { label: 'Add Configuration…', action: 'run.addConfiguration' },
      { label: 'Open Configurations', action: 'run.openConfigurations' },
    ],
  },
  {
    label: 'Terminal',
    items: [
      { label: 'New Terminal', action: 'terminal.new' },
      { label: 'Split Terminal', action: 'terminal.split' },
      { label: 'Run Selected Text', action: 'terminal.runSelection' },
      separator(),
      {
        label: 'Focus Terminal',
        action: 'terminal.focus',
        shortcut: '⌃⇧`',
      },
      {
        label: 'Open Terminal in Editor Tab',
        action: 'terminal.openEditor',
      },
      { label: 'Focus Previous Terminal Group', action: 'terminal.previous' },
      { label: 'Focus Next Terminal Group', action: 'terminal.next' },
      { label: 'Rename Terminal…', action: 'terminal.rename' },
      { label: 'Kill Terminal', action: 'terminal.close', shortcut: '⌘W' },
      separator(),
      { label: 'Clear Terminal', action: 'terminal.clear' },
      { label: 'Scroll to Top', action: 'terminal.scrollTop' },
      { label: 'Scroll to Bottom', action: 'terminal.scrollBottom' },
    ],
  },
  {
    label: 'Window',
    items: [
      { label: 'Minimize', action: 'window.minimize', shortcut: '⌘M' },
      { label: 'Zoom', action: 'window.zoom' },
      separator(),
      { label: 'Bring All to Front', action: 'window.bringAllToFront' },
      { label: 'Next Window', action: 'window.next', shortcut: '⌘`' },
      { label: 'Previous Window', action: 'window.previous', shortcut: '⇧⌘`' },
    ],
  },
  {
    label: 'Help',
    items: [
      { label: 'qedit Help', action: 'help.open' },
      {
        label: 'Keyboard Shortcuts',
        action: 'help.shortcuts',
        shortcut: '⌘K ⌘S',
      },
      { label: 'Release Notes', action: 'help.releaseNotes' },
      separator(),
      { label: 'Report an Issue', action: 'help.reportIssue' },
      { label: 'Open Local Logs Folder', action: 'help.openLogs' },
      { label: 'View License', action: 'help.license' },
      { label: 'Check for Updates…', action: 'help.updates' },
    ],
  },
];

export function flattenMenuItems(items: MenuItem[]): MenuItem[] {
  return items.flatMap((item) => {
    if (item.separator) return [];

    return [item, ...(item.submenu ? flattenMenuItems(item.submenu) : [])];
  });
}

/**
 * A menu row is enabled only when the shell or the editor actually handles its
 * action, so the menu shows the full command surface while the parts qedit has
 * not implemented yet stay visibly unavailable instead of failing silently.
 */
export function isMenuItemEnabled(item: MenuItem): boolean {
  if (item.disabled) return false;
  if (item.submenu) return true;

  return item.action !== undefined && isMenuActionAvailable(item.action);
}

export function menuCommands(): MenuItem[] {
  const commands = MENU_DATA.flatMap((menu) => flattenMenuItems(menu.items));
  const seen = new Set<string>();

  return commands.filter((item) => {
    if (!item.action || seen.has(item.action)) return false;
    if (!isMenuItemEnabled(item)) return false;
    seen.add(item.action);

    return true;
  });
}
