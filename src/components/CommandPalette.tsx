import { Command, Search } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';

import { menuCommands } from '#/lib/menu-data';
import type { MenuItem } from '#/lib/menu-data';

export function CommandPalette({
  onClose,
  onCommand,
}: {
  onClose: () => void;
  onCommand: (action: string) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const commands = useMemo(() => menuCommands(), []);
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();

    if (!needle) return commands;

    return commands.filter((item) =>
      `${item.label} ${item.action ?? ''}`.toLowerCase().includes(needle),
    );
  }, [commands, query]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const choose = (item: MenuItem | undefined) => {
    if (!item?.action) return;
    onCommand(item.action);
    onClose();
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-overlay pt-[13vh]"
      onMouseDown={onClose}
      role="presentation"
    >
      <div
        className="w-full max-w-[560px] overflow-hidden border border-border-default bg-command-palette font-sans text-text-primary shadow-2xl"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
        aria-label="Command Palette"
      >
        <div className="flex h-11 items-center gap-2 border-b border-border-default px-3">
          <Search className="h-4 w-4 text-text-muted" aria-hidden="true" />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'ArrowDown') {
                event.preventDefault();
                setSelectedIndex((index) =>
                  Math.min(index + 1, filtered.length - 1),
                );
              } else if (event.key === 'ArrowUp') {
                event.preventDefault();
                setSelectedIndex((index) => Math.max(index - 1, 0));
              } else if (event.key === 'Enter') {
                event.preventDefault();
                choose(filtered[selectedIndex]);
              }
            }}
            className="min-w-0 flex-1 bg-transparent text-sm text-text-primary outline-none placeholder:text-text-muted"
            placeholder="Type a command to search…"
            aria-label="Search commands"
          />
          <kbd className="flex items-center gap-1 border border-border-subtle px-1.5 py-0.5 font-mono text-[10px] text-text-muted">
            <Command className="h-2.5 w-2.5" />K
          </kbd>
        </div>
        <div className="max-h-[min(60vh,420px)] overflow-auto p-1">
          {filtered.length === 0 ? (
            <div className="px-3 py-8 text-center text-xs text-text-muted">
              No matching commands
            </div>
          ) : (
            filtered.map((item, index) => (
              <button
                key={`${item.action}-${item.label}`}
                type="button"
                className={`flex h-8 w-full items-center gap-3 px-2 text-left text-xs ${
                  index === selectedIndex
                    ? 'bg-accent text-accent-text'
                    : 'hover:bg-hover'
                }`}
                onMouseEnter={() => setSelectedIndex(index)}
                onClick={() => choose(item)}
              >
                <span className="min-w-0 flex-1 truncate">{item.label}</span>
                {item.shortcut && (
                  <span className="font-mono text-[10px] opacity-70">
                    {item.shortcut}
                  </span>
                )}
              </button>
            ))
          )}
        </div>
        <div className="flex h-7 items-center gap-3 border-t border-border-default px-3 text-[10px] text-text-muted">
          <span>↑↓ navigate</span>
          <span>↵ run</span>
          <span>esc close</span>
        </div>
      </div>
    </div>
  );
}
