import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useEffect, useState } from 'react';

import { Editor } from '#/components/Editor';
import { EditorProvider, useEditor } from '#/components/EditorContext';
import { FileTree } from '#/components/FileTree';
import { QuickOpen } from '#/components/QuickOpen';
import { StatusBar } from '#/components/StatusBar';
import { TabBar } from '#/components/TabBar';
import { TerminalPanel } from '#/components/TerminalPanel';
import { useWorkspace, WorkspaceProvider } from '#/components/WorkspaceContext';
import { shortcutActionForEvent } from '#/lib/shortcuts';

export const Route = createFileRoute('/')({
  component: Index,
});

function Index() {
  return (
    <EditorProvider>
      <WorkspaceProvider>
        <EditorLayout />
      </WorkspaceProvider>
    </EditorProvider>
  );
}

function EditorLayout() {
  const { activeFilePath, closeTab, saveActiveFile, saveActiveFileAs } =
    useEditor();
  const { openFileDialog, openFolderDialog } = useWorkspace();
  const [quickOpenVisible, setQuickOpenVisible] = useState(false);

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const action = shortcutActionForEvent(event);

      if (!action) return;

      event.preventDefault();

      switch (action) {
        case 'open-file':
          void openFileDialog();
          break;
        case 'open-folder':
          void openFolderDialog();
          break;
        case 'save':
          void saveActiveFile();
          break;
        case 'save-as':
          void saveActiveFileAs();
          break;
        case 'close-tab':
          if (activeFilePath) closeTab(activeFilePath);
          break;
        case 'quick-open':
          setQuickOpenVisible(true);
          break;
        case 'find':
          window.dispatchEvent(new Event('qedit:find'));
          break;
      }
    },
    [
      activeFilePath,
      closeTab,
      openFileDialog,
      openFolderDialog,
      saveActiveFile,
      saveActiveFileAs,
    ],
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="grid h-screen w-screen overflow-hidden bg-background text-foreground">
      <div className="grid h-screen grid-cols-[240px_1fr] grid-rows-[auto_1fr_auto_auto]">
        <div className="col-start-1 row-span-3 row-start-1 min-h-0 overflow-hidden">
          <FileTree />
        </div>

        <div className="col-start-2 row-start-1">
          <TabBar />
        </div>

        <div className="col-start-2 row-start-2 min-h-0 overflow-hidden">
          <Editor />
        </div>

        <div className="col-start-2 row-start-3">
          <TerminalPanel />
        </div>

        <div className="col-span-2 row-start-4">
          <StatusBar />
        </div>
      </div>
      {quickOpenVisible && (
        <QuickOpen onClose={() => setQuickOpenVisible(false)} />
      )}
    </div>
  );
}
