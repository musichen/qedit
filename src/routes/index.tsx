import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@qedit/ui/resizable';
import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useEffect, useRef, useState } from 'react';

import { CommandPalette } from '#/components/CommandPalette';
import { Editor } from '#/components/Editor';
import { EditorProvider, useEditor } from '#/components/EditorContext';
import { FileTree } from '#/components/FileTree';
import { MenuBar } from '#/components/MenuBar';
import { QuickOpen } from '#/components/QuickOpen';
import { SettingsProvider, useSettings } from '#/components/SettingsContext';
import { StatusBar } from '#/components/StatusBar';
import { TabBar } from '#/components/TabBar';
import { TerminalPanel } from '#/components/TerminalPanel';
import { useWorkspace, WorkspaceProvider } from '#/components/WorkspaceContext';
import { logInfo, openLocalLogsFolder } from '#/lib/app-logging';
import { isMenuActionAvailable } from '#/lib/menu-actions';
import { shortcutActionForEvent } from '#/lib/shortcuts';
import { shouldOpenSidebarForWorkspace } from '#/lib/workspace-sidebar';

const MIN_TERMINAL_PANEL_HEIGHT = 120;

interface TerminalPanelHandle {
  collapse: () => void;
  expand: () => void;
  getSize: () => { asPercentage: number; inPixels: number };
  isCollapsed: () => boolean;
  resize: (size: number | string) => void;
}

export const Route = createFileRoute('/')({
  component: Index,
});

function Index() {
  return (
    <SettingsProvider>
      <EditorProvider>
        <WorkspaceProvider>
          <EditorLayout />
        </WorkspaceProvider>
      </EditorProvider>
    </SettingsProvider>
  );
}

function EditorLayout() {
  const {
    activeFilePath,
    closeTab,
    closeAllTabs,
    openTabs,
    hasDirtyTabs,
    dirtyTabCount,
    reopenLastClosedTab,
    reloadActiveFile,
    setActiveFile,
    saveActiveFile,
    saveActiveFileAs,
  } = useEditor();
  const { openFileDialog, openFolderDialog, createFile, workspaceRoot } =
    useWorkspace();
  const { settings, setMode, setSetting } = useSettings();
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [statusBarVisible, setStatusBarVisible] = useState(true);
  const [terminalVisible, setTerminalVisible] = useState(false);
  const [terminalEditorVisible, setTerminalEditorVisible] = useState(false);
  const [editorBounds, setEditorBounds] = useState<{
    top: number;
    left: number;
    width: number;
    height: number;
  } | null>(null);
  const [quickOpenVisible, setQuickOpenVisible] = useState(false);
  const [commandPaletteVisible, setCommandPaletteVisible] = useState(false);
  const dirtyStateRef = useRef({ hasDirtyTabs, dirtyTabCount });
  const terminalPanelRef = useRef<TerminalPanelHandle | null>(null);
  const editorAreaRef = useRef<HTMLDivElement | null>(null);
  dirtyStateRef.current = { hasDirtyTabs, dirtyTabCount };

  useEffect(() => {
    const openSidebar = () => setSidebarOpen(true);
    window.addEventListener('qedit:open-sidebar', openSidebar);
    return () => window.removeEventListener('qedit:open-sidebar', openSidebar);
  }, []);

  const previousWorkspaceRootRef = useRef<string | null>(workspaceRoot);
  useEffect(() => {
    if (
      shouldOpenSidebarForWorkspace(
        previousWorkspaceRootRef.current,
        workspaceRoot,
      )
    ) {
      setSidebarOpen(true);
    }

    previousWorkspaceRootRef.current = workspaceRoot;
  }, [workspaceRoot]);

  useEffect(() => {
    const panel = terminalPanelRef.current;
    if (!panel) return;

    if (terminalVisible) {
      panel.expand();
      panel.resize(`${settings.terminalPanelHeight}px`);
    } else {
      panel.collapse();
    }
  }, [settings.terminalPanelHeight, terminalVisible]);

  const openTerminal = useCallback(() => {
    setTerminalVisible(true);
    setTerminalEditorVisible(false);
    requestAnimationFrame(() =>
      window.dispatchEvent(new Event('qedit:focus-terminal')),
    );
  }, []);

  const pinTerminalInEditor = useCallback((terminalId?: string) => {
    setTerminalVisible(false);
    setTerminalEditorVisible(true);
    if (terminalId) {
      window.dispatchEvent(
        new CustomEvent('qedit:terminal-select', { detail: terminalId }),
      );
    }
    requestAnimationFrame(() =>
      window.dispatchEvent(new Event('qedit:focus-terminal')),
    );
  }, []);

  useEffect(() => {
    const updateBounds = () => {
      const rect = editorAreaRef.current?.getBoundingClientRect();
      if (!rect) return;

      setEditorBounds({
        top: rect.top,
        left: rect.left,
        width: rect.width,
        height: rect.height,
      });
    };

    updateBounds();
    window.addEventListener('resize', updateBounds);
    const observer = editorAreaRef.current
      ? new ResizeObserver(updateBounds)
      : null;
    if (observer && editorAreaRef.current)
      observer.observe(editorAreaRef.current);

    return () => {
      window.removeEventListener('resize', updateBounds);
      observer?.disconnect();
    };
  }, [terminalEditorVisible]);

  const handleTerminalLayoutChanged = useCallback(
    (
      layout: { [panelId: string]: number },
      meta: { isUserInteraction: boolean },
    ) => {
      if (!meta.isUserInteraction) return;

      const terminalSize = layout.terminal;
      if (terminalSize === undefined) return;

      if (terminalSize <= 0) {
        setTerminalVisible(false);
        return;
      }

      const height = terminalPanelRef.current?.getSize().inPixels;
      if (height === undefined || height < MIN_TERMINAL_PANEL_HEIGHT) return;

      setSetting('terminalPanelHeight', Math.round(height));
    },
    [setSetting],
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const action = shortcutActionForEvent(event);

      if (!action) return;

      event.preventDefault();
      // Handled here, so keep it away from Monaco and xterm keybindings that
      // share the same chord (Cmd+Shift+O also opens Monaco's symbol picker).
      event.stopPropagation();

      if (/^terminal-[1-9]$/.test(action)) {
        setTerminalVisible(true);
        setTerminalEditorVisible(false);
        window.dispatchEvent(
          new CustomEvent('qedit:terminal-tab', {
            detail: Number(action.slice('terminal-'.length)) - 1,
          }),
        );
        return;
      }

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
        case 'close-terminal':
          window.dispatchEvent(new Event('qedit:terminal-close'));
          break;
        case 'next-terminal':
          setTerminalVisible(true);
          setTerminalEditorVisible(false);
          window.dispatchEvent(new Event('qedit:terminal-next'));
          break;
        case 'previous-terminal':
          setTerminalVisible(true);
          setTerminalEditorVisible(false);
          window.dispatchEvent(new Event('qedit:terminal-previous'));
          break;
        case 'focus-terminal':
          // Revealing the panel makes its active terminal focus itself, so the
          // chord works the same whether or not the panel was already open.
          setTerminalVisible(true);
          setTerminalEditorVisible(false);
          window.dispatchEvent(new Event('qedit:focus-terminal'));
          break;
        case 'focus-editor':
          window.dispatchEvent(new Event('qedit:focus-editor'));
          break;
        case 'quick-open':
          setQuickOpenVisible(true);
          break;
        case 'command-palette':
          setCommandPaletteVisible(true);
          break;
        case 'toggle-sidebar':
          setSidebarOpen((visible) => !visible);
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

  const runMenuAction = useCallback(
    (action: string) => {
      switch (action) {
        case 'app.preferences':
          window.dispatchEvent(new Event('qedit:open-settings'));
          break;
        case 'help.openLogs':
          void openLocalLogsFolder()
            .then((path) => {
              if (path)
                void logInfo(`logs folder action completed path=${path}`);
            })
            .catch((cause: unknown) => {
              console.warn('Could not open local logs folder:', cause);
            });
          break;
        case 'file.new':
          void createFile();
          break;
        case 'file.open':
          void openFileDialog();
          break;
        case 'file.openFolder':
          void openFolderDialog();
          break;
        case 'file.save':
          void saveActiveFile();
          break;
        case 'file.saveAs':
          void saveActiveFileAs();
          break;
        case 'file.close':
          if (activeFilePath) closeTab(activeFilePath);
          break;
        case 'file.closeAll':
          closeAllTabs();
          break;
        case 'file.reopen':
          reopenLastClosedTab();
          break;
        case 'file.reload':
          reloadActiveFile();
          break;
        case 'view.toggleSidebar':
          setSidebarOpen((visible) => !visible);
          break;
        case 'view.toggleStatusBar':
          setStatusBarVisible((visible) => !visible);
          break;
        case 'view.toggleTerminal':
          setTerminalVisible((visible) => !visible);
          setTerminalEditorVisible(false);
          break;
        case 'view.commandPalette':
          setCommandPaletteVisible(true);
          break;
        case 'view.quickOpen':
          setQuickOpenVisible(true);
          break;
        case 'view.toggleMinimap':
          setSetting('minimap', !settings.minimap);
          break;
        case 'view.toggleWordWrap':
          setSetting('wordWrap', settings.wordWrap === 'off' ? 'on' : 'off');
          break;
        case 'appearance.dark':
          setMode('dark');
          break;
        case 'appearance.light':
          setMode('light');
          break;
        case 'appearance.system':
          setMode('system');
          break;
        case 'edit.find':
          window.dispatchEvent(new Event('qedit:find'));
          break;
        case 'terminal.new':
          setTerminalVisible(true);
          setTerminalEditorVisible(false);
          window.dispatchEvent(new Event('qedit:terminal-new'));
          break;
        case 'terminal.focus':
          setTerminalVisible(true);
          setTerminalEditorVisible(false);
          window.dispatchEvent(new Event('qedit:focus-terminal'));
          break;
        case 'terminal.openEditor':
          pinTerminalInEditor();
          break;
        case 'terminal.next':
          window.dispatchEvent(new Event('qedit:terminal-next'));
          break;
        case 'terminal.previous':
          window.dispatchEvent(new Event('qedit:terminal-previous'));
          break;
        case 'terminal.close':
          window.dispatchEvent(new Event('qedit:terminal-close'));
          break;
        default:
          // Only actions the registry knows about reach the menu enabled, so
          // anything else here is a wiring bug rather than a user-facing no-op.
          if (isMenuActionAvailable(action)) {
            window.dispatchEvent(
              new CustomEvent('qedit:editor-command', { detail: action }),
            );
          } else {
            console.warn(`Unhandled menu action: ${action}`);
          }
          break;
      }
    },
    [
      activeFilePath,
      closeAllTabs,
      closeTab,
      createFile,
      openFileDialog,
      openFolderDialog,
      pinTerminalInEditor,
      reloadActiveFile,
      reopenLastClosedTab,
      saveActiveFile,
      saveActiveFileAs,
      setMode,
      setSetting,
      settings.minimap,
      settings.wordWrap,
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
    const showSearch = () => setQuickOpenVisible(true);
    window.addEventListener('qedit:quick-open', showSearch);

    return () => window.removeEventListener('qedit:quick-open', showSearch);
  }, []);

  useEffect(() => {
    const message = () => {
      const { dirtyTabCount: count } = dirtyStateRef.current;

      return count === 1
        ? '1 file has unsaved changes. Exit qedit and discard it?'
        : `${count} files have unsaved changes. Exit qedit and discard them?`;
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
      .catch((err: unknown) => {
        // Browser development and Vitest do not provide Tauri's window IPC.
        if (
          err instanceof Error &&
          err.message &&
          /is not a Tauri window|not a function|Cannot find module/i.test(
            err.message,
          )
        ) {
          // Expected: running outside Tauri (browser dev, Vitest).
        } else {
          console.warn('Failed to set up Tauri IPC listener:', err);
        }
      });

    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener('beforeunload', handleBeforeUnload);
    };
  }, []);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-editor font-sans text-text-primary">
      <MenuBar onAction={runMenuAction} />
      <div
        className="grid min-h-0 flex-1"
        style={{
          gridTemplateColumns: sidebarOpen
            ? 'var(--spacing-sidebar) minmax(0, 1fr)'
            : '0 minmax(0, 1fr)',
        }}
      >
        <aside className="min-h-0 overflow-hidden" aria-hidden={!sidebarOpen}>
          {sidebarOpen && <FileTree />}
        </aside>
        <main className="grid min-h-0 min-w-0 grid-rows-[var(--spacing-tab)_minmax(0,1fr)] overflow-hidden">
          <TabBar
            onOpenTerminal={openTerminal}
            onOpenTerminalEditor={pinTerminalInEditor}
            terminalVisible={terminalVisible}
            terminalEditorVisible={terminalEditorVisible}
            onDropTerminal={pinTerminalInEditor}
            onCloseTerminalEditor={() => setTerminalEditorVisible(false)}
          />
          <ResizablePanelGroup
            orientation="vertical"
            className="min-h-0 flex-1"
            onLayoutChanged={handleTerminalLayoutChanged}
          >
            <ResizablePanel
              id="editor"
              minSize="100px"
              groupResizeBehavior="preserve-relative-size"
            >
              <div
                ref={editorAreaRef}
                className="h-full min-h-0 overflow-hidden"
              >
                <Editor />
              </div>
            </ResizablePanel>
            <ResizableHandle
              withHandle
              className="h-1 cursor-row-resize border-t border-border-default bg-editor"
              aria-label="Resize terminal panel"
            />
            <ResizablePanel
              id="terminal"
              panelRef={terminalPanelRef}
              defaultSize={
                terminalVisible ? `${settings.terminalPanelHeight}px` : '0px'
              }
              minSize={`${MIN_TERMINAL_PANEL_HEIGHT}px`}
              collapsedSize="0px"
              collapsible
              groupResizeBehavior="preserve-pixel-size"
            >
              <TerminalPanel
                visible={terminalVisible || terminalEditorVisible}
                editorMode={terminalEditorVisible}
                editorBounds={editorBounds}
              />
            </ResizablePanel>
          </ResizablePanelGroup>
        </main>
      </div>
      {statusBarVisible && <StatusBar />}
      {quickOpenVisible && (
        <QuickOpen onClose={() => setQuickOpenVisible(false)} />
      )}
      {commandPaletteVisible && (
        <CommandPalette
          onClose={() => setCommandPaletteVisible(false)}
          onCommand={runMenuAction}
        />
      )}
    </div>
  );
}
