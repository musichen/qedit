import { Settings } from 'lucide-react';
import { useEffect, useState } from 'react';

import { useEditor } from './EditorContext';
import { SettingsPanel } from './SettingsPanel';

import { readGitBranch } from '#/lib/workspace-bridge';

export function StatusBar() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [gitBranch, setGitBranch] = useState<string | null>(null);
  const {
    activeFilePath,
    openTabs,
    fileStatus,
    cursorPosition,
    indentation,
    language,
    saving,
    saveError,
    saveErrorOwnsActiveFile,
  } = useEditor();

  useEffect(() => {
    let cancelled = false;

    if (!activeFilePath) {
      setGitBranch(null);
      return;
    }

    void readGitBranch(activeFilePath)
      .then((branch) => {
        if (!cancelled) setGitBranch(branch);
      })
      .catch(() => {
        if (!cancelled) setGitBranch(null);
      });

    return () => {
      cancelled = true;
    };
  }, [activeFilePath]);

  useEffect(() => {
    const openSettings = () => setSettingsOpen(true);
    window.addEventListener('qedit:open-settings', openSettings);

    return () =>
      window.removeEventListener('qedit:open-settings', openSettings);
  }, []);

  const fileName = activeFilePath?.split('/').pop() ?? 'No file open';
  const displayPath = activeFilePath ?? 'Open a file to begin';
  const activeTab = openTabs.find((tab) => tab.path === activeFilePath);
  const activeStatus = activeFilePath
    ? fileStatus.get(activeFilePath)
    : undefined;
  const state = !activeFilePath
    ? 'Ready'
    : saving
      ? 'Saving...'
      : saveError && saveErrorOwnsActiveFile
        ? 'Save failed'
        : activeStatus?.kind === 'loading'
          ? 'Loading...'
          : activeStatus?.kind === 'error'
            ? 'Error'
            : activeTab?.isModified
              ? 'Unsaved'
              : 'Saved';
  const stateClass =
    state === 'Error' || state === 'Save failed'
      ? 'text-danger'
      : state === 'Unsaved' || state === 'Saving...' || state === 'Loading...'
        ? 'text-warning'
        : 'text-success';

  return (
    <div className="flex h-statusbar select-none items-center justify-between gap-3 border-t border-border-default bg-statusbar px-3 text-xs text-text-secondary">
      <div className="flex min-w-0 items-center gap-3">
        <span className="flex min-w-0 items-center gap-1.5">
          <span
            className={`inline-block h-2 w-2 shrink-0 rounded-full ${
              stateClass === 'text-danger'
                ? 'bg-danger'
                : stateClass === 'text-warning'
                  ? 'bg-warning'
                  : 'bg-success'
            }`}
            aria-hidden="true"
          />
          <span className="truncate" title={activeFilePath ?? undefined}>
            {fileName}
          </span>
        </span>
        <span className={stateClass} role="status" aria-live="polite">
          {state}
        </span>
        {saveError && (
          <span
            role="alert"
            title={saveError}
            className="max-w-[40ch] truncate text-danger"
          >
            {saveError}
          </span>
        )}
      </div>
      <div className="flex shrink-0 items-center gap-4">
        <span>
          Ln {cursorPosition.line}, Col {cursorPosition.column}
        </span>
        <span>Spaces: {indentation}</span>
        <span>UTF-8</span>
        <span>{language}</span>
      </div>
      <div className="flex min-w-0 shrink-0 items-center gap-2">
        <span className="shrink-0 text-text-dimmed">
          git:{gitBranch ?? '—'}
        </span>
        <span
          className="max-w-[30%] truncate text-xs text-text-dimmed"
          title={activeFilePath ?? undefined}
        >
          {displayPath}
        </span>
        <button
          type="button"
          className="rounded p-1 text-text-secondary hover:bg-editor hover:text-text-primary"
          onClick={() => setSettingsOpen(true)}
          aria-label="Open preferences"
          title="Preferences"
        >
          <Settings className="h-3.5 w-3.5" />
        </button>
      </div>
      {settingsOpen && <SettingsPanel onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}
