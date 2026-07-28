import { describe, it, expect } from 'vitest';

describe('qedit app shell', () => {
  it('renders layout components are importable', async () => {
    const { Editor } = await import('../components/Editor');
    const { EditorProvider } = await import('../components/EditorContext');
    const { FileTree } = await import('../components/FileTree');
    const { StatusBar } = await import('../components/StatusBar');
    const { TabBar } = await import('../components/TabBar');

    expect(Editor).toBeDefined();
    expect(EditorProvider).toBeDefined();
    expect(FileTree).toBeDefined();
    expect(StatusBar).toBeDefined();
    expect(TabBar).toBeDefined();
  });

  it('database schema exports are loadable', async () => {
    const { sessions, recentFiles } = await import('@qedit/db');

    expect(sessions).toBeDefined();
    expect(recentFiles).toBeDefined();
  });
});
