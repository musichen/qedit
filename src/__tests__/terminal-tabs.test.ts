import { describe, expect, it } from 'vitest';

import {
  createTerminalTabsState,
  terminalTabsReducer,
} from '../lib/terminal-tabs';

const tab = (id: string, name = id) => ({
  id,
  name,
  status: 'running' as const,
  isDirty: false,
  error: null,
});

describe('terminalTabsReducer', () => {
  it('adds sessions and keeps the new session active', () => {
    const state = terminalTabsReducer(createTerminalTabsState(), {
      type: 'add',
      tab: tab('terminal-2', 'Terminal 2'),
    });

    expect(state.tabs.map((item) => item.id)).toEqual([
      'terminal-1',
      'terminal-2',
    ]);
    expect(state.activeId).toBe('terminal-2');
  });

  it('reorders tabs without changing the active session', () => {
    const state = {
      tabs: [tab('a'), tab('b'), tab('c')],
      activeId: 'b',
    };

    const reordered = terminalTabsReducer(state, {
      type: 'reorder',
      id: 'b',
      toIndex: 0,
    });

    expect(reordered.tabs.map((item) => item.id)).toEqual(['b', 'a', 'c']);
    expect(reordered.activeId).toBe('b');
  });

  it('marks background output dirty and clears it on activation', () => {
    const state = {
      tabs: [tab('a'), tab('b')],
      activeId: 'a',
    };
    const dirty = terminalTabsReducer(state, { type: 'activity', id: 'b' });

    expect(dirty.tabs[1]?.isDirty).toBe(true);
    expect(
      terminalTabsReducer(dirty, { type: 'activate', id: 'b' }).tabs[1]
        ?.isDirty,
    ).toBe(false);
  });

  it('renames a tab and preserves closed status', () => {
    const state = terminalTabsReducer(createTerminalTabsState(), {
      type: 'rename',
      id: 'terminal-1',
      name: 'Build shell',
    });
    const closed = terminalTabsReducer(state, {
      type: 'status',
      id: 'terminal-1',
      status: 'exited',
    });

    expect(closed.tabs[0]).toMatchObject({
      name: 'Build shell',
      status: 'exited',
    });
  });

  it('selects the adjacent session when the active tab closes', () => {
    const state = {
      tabs: [tab('a'), tab('b'), tab('c')],
      activeId: 'b',
    };

    expect(
      terminalTabsReducer(state, { type: 'close', id: 'b' }),
    ).toMatchObject({
      activeId: 'c',
      tabs: [tab('a'), tab('c')],
    });
  });

  it('keeps the active session when closing a background tab', () => {
    const state = {
      tabs: [tab('a'), tab('b')],
      activeId: 'a',
    };

    expect(
      terminalTabsReducer(state, { type: 'close', id: 'b' }).activeId,
    ).toBe('a');
  });
});
