import { ChevronRight } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { MENU_DATA, isMenuItemEnabled } from '#/lib/menu-data';
import type { MenuDefinition, MenuItem } from '#/lib/menu-data';

export function MenuBar({ onAction }: { onAction: (action: string) => void }) {
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const barRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (!barRef.current?.contains(event.target as Node)) setOpenMenu(null);
    };

    document.addEventListener('mousedown', close);

    return () => document.removeEventListener('mousedown', close);
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpenMenu(null);
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <header
      ref={barRef}
      className="relative z-40 flex h-menubar select-none items-center border-b border-border-subtle bg-menubar px-1 font-sans text-xs text-text-secondary"
      aria-label="Application menu"
    >
      <div className="flex items-center gap-0.5">
        {MENU_DATA.map((menu) => (
          <MenuButton
            key={menu.label}
            menu={menu}
            open={openMenu === menu.label}
            onOpen={() => setOpenMenu(menu.label)}
            onAction={(action) => {
              onAction(action);
              setOpenMenu(null);
            }}
          />
        ))}
      </div>
      <div className="ml-auto px-2 font-mono text-[10px] uppercase tracking-[0.18em] text-text-dimmed">
        qedit
      </div>
    </header>
  );
}

function MenuButton({
  menu,
  open,
  onOpen,
  onAction,
}: {
  menu: MenuDefinition;
  open: boolean;
  onOpen: () => void;
  onAction: (action: string) => void;
}) {
  return (
    <div className="relative">
      <button
        type="button"
        className={`h-7 rounded-sm px-2 text-[12px] outline-none transition-colors hover:bg-hover hover:text-text-primary focus-visible:ring-1 focus-visible:ring-focus ${
          open ? 'bg-hover text-text-primary' : ''
        }`}
        onClick={() => (open ? onOpen() : onOpen())}
        onMouseEnter={onOpen}
        aria-expanded={open}
        aria-haspopup="menu"
      >
        {menu.label}
      </button>
      {open && (
        <MenuDropdown items={menu.items} depth={0} onAction={onAction} />
      )}
    </div>
  );
}

function MenuDropdown({
  items,
  depth,
  onAction,
}: {
  items: MenuItem[];
  depth: number;
  onAction: (action: string) => void;
}) {
  return (
    <div
      className={`absolute top-full z-50 min-w-64 border border-border-default bg-dropdown p-1 font-sans text-[12px] text-text-primary shadow-2xl ${
        depth === 0 ? 'left-0' : 'left-[calc(100%-4px)] top-[-4px]'
      }`}
      role="menu"
    >
      {items.map((item, index) => {
        if (item.separator) {
          return (
            <div
              key={`separator-${index}`}
              className="my-1 border-t border-border-subtle"
            />
          );
        }

        return (
          <MenuRow
            key={`${item.label}-${index}`}
            item={item}
            onAction={onAction}
          />
        );
      })}
    </div>
  );
}

function MenuRow({
  item,
  onAction,
}: {
  item: MenuItem;
  onAction: (action: string) => void;
}) {
  const [submenuOpen, setSubmenuOpen] = useState(false);

  return (
    <div className="relative" onMouseEnter={() => setSubmenuOpen(true)}>
      <button
        type="button"
        className="flex h-7 w-full items-center gap-3 rounded-sm px-2 text-left outline-none hover:bg-accent hover:text-accent-text focus-visible:bg-accent disabled:pointer-events-none disabled:opacity-40"
        disabled={!isMenuItemEnabled(item)}
        onClick={() => {
          if (item.submenu) return;
          if (item.action) onAction(item.action);
        }}
        role="menuitem"
        aria-haspopup={item.submenu ? 'menu' : undefined}
      >
        <span className="min-w-0 flex-1 truncate">{item.label}</span>
        {item.shortcut && (
          <span className="font-mono text-[10px] text-text-muted">
            {item.shortcut}
          </span>
        )}
        {item.submenu && <ChevronRight className="h-3 w-3 text-text-muted" />}
      </button>
      {item.submenu && submenuOpen && (
        <MenuDropdown items={item.submenu} depth={1} onAction={onAction} />
      )}
    </div>
  );
}
