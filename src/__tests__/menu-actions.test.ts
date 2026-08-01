import { describe, expect, it } from 'vitest';

import {
  EDITOR_MENU_COMMANDS,
  SHELL_MENU_ACTIONS,
  isMenuActionAvailable,
} from '#/lib/menu-actions';
import { MENU_DATA, flattenMenuItems, menuCommands } from '#/lib/menu-data';

const menuActions = new Set(
  MENU_DATA.flatMap((menu) => flattenMenuItems(menu.items))
    .map((item) => item.action)
    .filter((action): action is string => action !== undefined),
);

describe('menu action registry', () => {
  it('only offers commands the shell or the editor handles', () => {
    for (const command of menuCommands()) {
      expect(command.action).toBeDefined();
      expect(isMenuActionAvailable(command.action as string)).toBe(true);
    }
  });

  it('registers no action that the menu model does not expose', () => {
    const registered = [
      ...SHELL_MENU_ACTIONS,
      ...Object.keys(EDITOR_MENU_COMMANDS),
    ];

    expect(registered.filter((action) => !menuActions.has(action))).toEqual([]);
  });

  it('routes each editor action to a namespaced monaco command', () => {
    for (const command of Object.values(EDITOR_MENU_COMMANDS)) {
      expect(command).not.toBe('');
    }

    expect(new Set(Object.values(EDITOR_MENU_COMMANDS)).size).toBe(
      Object.keys(EDITOR_MENU_COMMANDS).length,
    );
  });

  it('never marks an unimplemented menu row as available', () => {
    expect(isMenuActionAvailable('run.debug')).toBe(false);
    expect(isMenuActionAvailable('window.minimize')).toBe(false);
    expect(menuCommands().map((item) => item.action)).not.toContain(
      'help.releaseNotes',
    );
  });

  it('keeps terminal focus discoverable and exposes the editor-tab command', () => {
    const terminalItems = MENU_DATA.find(
      (menu) => menu.label === 'Terminal',
    )?.items.filter((item) => !item.separator);
    const focus = terminalItems?.find(
      (item) => item.action === 'terminal.focus',
    );
    const openEditor = terminalItems?.find(
      (item) => item.action === 'terminal.openEditor',
    );

    expect(focus).toMatchObject({ label: 'Focus Terminal', shortcut: '⌃⇧`' });
    expect(openEditor).toMatchObject({
      label: 'Open Terminal in Editor Tab',
    });
    expect(menuCommands()).toEqual(expect.arrayContaining([focus, openEditor]));
  });
});
