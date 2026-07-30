import { Settings } from 'lucide-react';
import { useState } from 'react';

import { useEditor } from './EditorContext';
import { SettingsPanel } from './SettingsPanel';

export function StatusBar() {
  const [settingsOpen, setSettingsOpen] = useState(false);
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
      ? 'text-destructive'
      : state === 'Unsaved' || state === 'Saving...' || state === 'Loading...'
        ? 'text-amber-500'
        : 'text-emerald-500';

  return (
    <div className="flex h-7 select-none items-center justify-between gap-3 border-t bg-muted px-3 text-xs text-muted-foreground">
      <div className="flex min-w-0 items-center gap-3">
        <span className="flex min-w-0 items-center gap-1.5">
          <span
            className={`inline-block h-2 w-2 shrink-0 rounded-full ${
              stateClass === 'text-destructive'
                ? 'bg-destructive'
                : stateClass === 'text-amber-500'
                  ? 'bg-amber-500'
                  : 'bg-emerald-500'
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
            className="max-w-[40ch] truncate text-destructive"
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
        <span
          className="max-w-[30%] truncate text-xs text-muted-foreground/60"
          title={activeFilePath ?? undefined}
        >
          {displayPath}
        </span>
        <button
          type="button"
          className="rounded p-1 text-muted-foreground hover:bg-background hover:text-foreground"
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
