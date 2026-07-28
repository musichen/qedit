import { X } from 'lucide-react';

import { useEditor } from './EditorContext';
import type { OpenTab } from './EditorContext';

export function TabBar() {
  const { openTabs, activeFilePath, setActiveFile, closeTab } = useEditor();

  if (openTabs.length === 0) {
    return (
      <div className="flex h-9 items-center border-b bg-muted/50 px-4">
        <span className="text-xs text-muted-foreground">No open files</span>
      </div>
    );
  }

  return (
    <div className="flex h-9 items-end border-b bg-muted/50">
      <div className="flex h-full items-stretch overflow-x-auto">
        {openTabs.map((tab) => (
          <Tab
            key={tab.path}
            tab={tab}
            isActive={tab.path === activeFilePath}
            onSelect={() => setActiveFile(tab.path)}
            onClose={() => closeTab(tab.path)}
          />
        ))}
      </div>
    </div>
  );
}

function Tab({
  tab,
  isActive,
  onSelect,
  onClose,
}: {
  tab: OpenTab;
  isActive: boolean;
  onSelect: () => void;
  onClose: () => void;
}) {
  return (
    <div
      className={`group flex h-full cursor-pointer items-center gap-1.5 border-r px-3 text-sm transition-colors ${
        isActive
          ? 'border-t-2 border-t-primary bg-background text-foreground'
          : 'bg-muted/50 text-muted-foreground hover:bg-muted'
      }`}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.key === 'Enter') onSelect();
      }}
      role="tab"
      aria-selected={isActive}
      tabIndex={0}
    >
      <span className="max-w-[140px] truncate">
        {tab.isModified && (
          <span className="mr-1 inline-block h-2 w-2 rounded-full bg-accent-foreground" />
        )}
        {tab.name}
      </span>
      <button
        type="button"
        className="ml-1 rounded p-0.5 opacity-0 transition-opacity hover:bg-muted-foreground/20 group-hover:opacity-100"
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
        aria-label={`Close ${tab.name}`}
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}
