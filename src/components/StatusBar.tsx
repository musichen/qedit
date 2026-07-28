import { useEditor } from './EditorContext';

export function StatusBar() {
  const {
    activeFilePath,
    cursorPosition,
    indentation,
    language,
    saving,
    saveError,
  } = useEditor();

  const fileName = activeFilePath?.split('/').pop() ?? '—';
  const displayPath = activeFilePath ?? 'No file open';

  return (
    <div className="flex h-6 select-none items-center justify-between border-t bg-muted px-3 text-xs text-muted-foreground">
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1">
          <span className="inline-block h-2 w-2 rounded-full bg-green-500" />
          <span title={activeFilePath ?? undefined}>{fileName}</span>
        </span>
        {saving && (
          <span className="animate-pulse text-amber-500">Saving...</span>
        )}
        {!saving && saveError && (
          <span
            role="alert"
            title={saveError}
            className="max-w-[40ch] truncate text-destructive"
          >
            {saveError}
          </span>
        )}
      </div>
      <div className="flex items-center gap-4">
        <span>
          Ln {cursorPosition.line}, Col {cursorPosition.column}
        </span>
        <span>Spaces: {indentation}</span>
        <span>UTF-8</span>
        <span>{language}</span>
      </div>
      <span className="max-w-[30%] truncate text-xs text-muted-foreground/60">
        {displayPath}
      </span>
    </div>
  );
}
