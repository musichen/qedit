export type TerminalStatus = 'starting' | 'running' | 'exited' | 'error';

export interface TerminalTabState {
  id: string;
  name: string;
  status: TerminalStatus;
  isDirty: boolean;
  error: string | null;
}

export interface TerminalTabsState {
  tabs: TerminalTabState[];
  activeId: string | null;
}

export type TerminalTabsAction =
  | { type: 'add'; tab: TerminalTabState }
  | { type: 'activate'; id: string }
  | {
      type: 'status';
      id: string;
      status: TerminalStatus;
      error?: string | null;
    }
  | { type: 'activity'; id: string }
  | { type: 'rename'; id: string; name: string }
  | { type: 'reorder'; id: string; toIndex: number }
  | { type: 'close'; id: string };

export function createTerminalTabsState(): TerminalTabsState {
  return {
    tabs: [
      {
        id: 'terminal-1',
        name: 'Terminal 1',
        status: 'starting',
        isDirty: false,
        error: null,
      },
    ],
    activeId: 'terminal-1',
  };
}

export function terminalTabsReducer(
  state: TerminalTabsState,
  action: TerminalTabsAction,
): TerminalTabsState {
  switch (action.type) {
    case 'add':
      return {
        tabs: [...state.tabs, action.tab],
        activeId: action.tab.id,
      };
    case 'activate':
      if (!state.tabs.some((tab) => tab.id === action.id)) return state;

      return {
        ...state,
        activeId: action.id,
        tabs: state.tabs.map((tab) =>
          tab.id === action.id ? { ...tab, isDirty: false } : tab,
        ),
      };
    case 'status':
      return {
        ...state,
        tabs: state.tabs.map((tab) =>
          tab.id === action.id
            ? { ...tab, status: action.status, error: action.error ?? null }
            : tab,
        ),
      };
    case 'activity':
      return {
        ...state,
        tabs: state.tabs.map((tab) =>
          tab.id === action.id && state.activeId !== action.id
            ? { ...tab, isDirty: true }
            : tab,
        ),
      };
    case 'rename':
      return {
        ...state,
        tabs: state.tabs.map((tab) =>
          tab.id === action.id ? { ...tab, name: action.name } : tab,
        ),
      };
    case 'reorder': {
      const fromIndex = state.tabs.findIndex((tab) => tab.id === action.id);

      if (fromIndex < 0 || state.tabs.length < 2) return state;

      const toIndex = Math.max(
        0,
        Math.min(action.toIndex, state.tabs.length - 1),
      );
      if (fromIndex === toIndex) return state;

      const tabs = [...state.tabs];
      const [tab] = tabs.splice(fromIndex, 1);
      if (!tab) return state;

      tabs.splice(toIndex, 0, tab);

      return { ...state, tabs };
    }
    case 'close': {
      const closedIndex = state.tabs.findIndex((tab) => tab.id === action.id);

      if (closedIndex < 0) return state;

      const tabs = state.tabs.filter((tab) => tab.id !== action.id);
      if (state.activeId !== action.id) return { ...state, tabs };

      const nextActive = tabs[closedIndex] ?? tabs[closedIndex - 1] ?? null;

      return {
        tabs,
        activeId: nextActive?.id ?? null,
      };
    }
  }
}

export function terminalTabIndexForId(
  tabs: TerminalTabState[],
  id: string,
): number {
  return tabs.findIndex((tab) => tab.id === id);
}
