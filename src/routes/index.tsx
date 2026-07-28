import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useEffect, useRef, useState } from 'react';

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
  const {
    activeFilePath,
    closeTab,
    openTabs,
    hasDirtyTabs,
    dirtyTabCount,
    reopenLastClosedTab,
    reloadActiveFile,
    setActiveFile,
    saveActiveFile,
    saveActiveFileAs,
  } = useEditor();
  const { openFileDialog, openFolderDialog } = useWorkspace();
  const [quickOpenVisible, setQuickOpenVisible] = useState(false);
  const dirtyStateRef = useRef({ hasDirtyTabs, dirtyTabCount });
  dirtyStateRef.current = { hasDirtyTabs, dirtyTabCount };

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
        case 'reopen-tab':
          reopenLastClosedTab();
          break;
        case 'reload-file':
          reloadActiveFile();
          break;
        case 'next-tab': {
          if (openTabs.length < 2 || !activeFilePath) break;

          const activeIndex = openTabs.findIndex(
            (tab) => tab.path === activeFilePath,
          );
          const nextTab = openTabs[(activeIndex + 1) % openTabs.length];

          if (nextTab) setActiveFile(nextTab.path);
          break;
        }
        case 'previous-tab': {
          if (openTabs.length < 2 || !activeFilePath) break;

          const activeIndex = openTabs.findIndex(
            (tab) => tab.path === activeFilePath,
          );
          const previousIndex =
            (activeIndex - 1 + openTabs.length) % openTabs.length;
          const previousTab = openTabs[previousIndex];

          if (previousTab) setActiveFile(previousTab.path);
          break;
        }
        case 'focus-terminal':
          window.dispatchEvent(new Event('qedit:focus-terminal'));
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
      openTabs,
      reopenLastClosedTab,
      reloadActiveFile,
      setActiveFile,
      openFileDialog,
      openFolderDialog,
      saveActiveFile,
      saveActiveFileAs,
    ],
  );

  useEffect(() => {
    // Capture before Monaco or xterm can consume a platform shortcut. The
    // target guard in shortcutActionForEvent still leaves ordinary text fields
    // and terminal input to their native behavior.
    window.addEventListener('keydown', handleKeyDown, true);

    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [handleKeyDown]);

  useEffect(() => {
    const message = () => {
      const { dirtyTabCount: count } = dirtyStateRef.current;

      return `${count} file${count === 1 ? '' : 's'} have unsaved changes. Exit qedit and discard them?`;
    };
    const handleBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!dirtyStateRef.current.hasDirtyTabs) return;

      event.preventDefault();
      event.returnValue = message();
    };

    window.addEventListener('beforeunload', handleBeforeUnload);

    let disposed = false;
    let unlisten: (() => void) | undefined;
    void import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) =>
        getCurrentWindow().onCloseRequested((event) => {
          if (
            dirtyStateRef.current.hasDirtyTabs &&
            !window.confirm(message())
          ) {
            event.preventDefault();
          }
        }),
      )
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch(() => {
        // Browser development and Vitest do not provide Tauri's window IPC.
      });

    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener('beforeunload', handleBeforeUnload);
    };
  }, []);

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
