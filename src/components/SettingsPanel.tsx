import { Check, Settings, X } from 'lucide-react';
import { useEffect } from 'react';

import { useSettings } from './SettingsContext';

export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const { settings, setSetting, setMode } = useSettings();

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

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/35 p-4"
      onClick={onClose}
      role="presentation"
    >
      <section
        className="w-full max-w-md overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-2xl"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        <header className="flex items-center justify-between border-b px-4 py-3">
          <div className="flex items-center gap-2">
            <Settings className="h-4 w-4 text-primary" />
            <div>
              <h2 id="settings-title" className="text-sm font-semibold">
                Preferences
              </h2>
              <p className="text-[11px] text-muted-foreground">
                Tune qedit's editor and terminal.
              </p>
            </div>
          </div>
          <button
            type="button"
            className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={onClose}
            aria-label="Close preferences"
          >
            <X className="h-4 w-4" />
          </button>
        </header>

        <div className="max-h-[70vh] space-y-5 overflow-auto p-4">
          <SettingsSection title="Appearance">
            <SettingRow label="Theme" description="Choose how qedit looks.">
              <select
                className="h-8 rounded border bg-background px-2 text-xs outline-none focus:ring-1 focus:ring-ring"
                value={settings.mode}
                onChange={(event) =>
                  setMode(event.target.value as typeof settings.mode)
                }
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
                <option value="system">Auto</option>
              </select>
            </SettingRow>
          </SettingsSection>

          <SettingsSection title="Editor">
            <SettingRow
              label="Font size"
              description="Editor text size in pixels."
            >
              <NumberInput
                value={settings.fontSize}
                min={10}
                max={32}
                onChange={(value) => setSetting('fontSize', value)}
              />
            </SettingRow>
            <SettingRow label="Tab size" description="Spaces inserted by Tab.">
              <NumberInput
                value={settings.tabSize}
                min={1}
                max={8}
                onChange={(value) => setSetting('tabSize', value)}
              />
            </SettingRow>
            <SettingRow
              label="Word wrap"
              description="Wrap long lines in the editor."
            >
              <select
                className="h-8 rounded border bg-background px-2 text-xs outline-none focus:ring-1 focus:ring-ring"
                value={settings.wordWrap}
                onChange={(event) =>
                  setSetting('wordWrap', event.target.value as 'off' | 'on')
                }
              >
                <option value="on">On</option>
                <option value="off">Off</option>
              </select>
            </SettingRow>
            <ToggleRow
              label="Minimap"
              checked={settings.minimap}
              onChange={(value) => setSetting('minimap', value)}
            />
            <ToggleRow
              label="Line numbers"
              checked={settings.lineNumbers}
              onChange={(value) => setSetting('lineNumbers', value)}
            />
          </SettingsSection>

          <SettingsSection title="Terminal">
            <SettingRow
              label="Font size"
              description="Terminal text size in pixels."
            >
              <NumberInput
                value={settings.terminalFontSize}
                min={10}
                max={24}
                onChange={(value) => setSetting('terminalFontSize', value)}
              />
            </SettingRow>
          </SettingsSection>
        </div>

        <footer className="flex items-center justify-between border-t px-4 py-2 text-[11px] text-muted-foreground">
          <span>Saved automatically</span>
          <button
            type="button"
            className="inline-flex items-center gap-1 rounded px-2 py-1 text-foreground hover:bg-muted"
            onClick={onClose}
          >
            <Check className="h-3 w-3" /> Done
          </button>
        </footer>
      </section>
    </div>
  );
}

function SettingsSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section>
      <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
        {title}
      </h3>
      <div className="divide-y rounded border">{children}</div>
    </section>
  );
}

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 px-3 py-2.5">
      <div className="min-w-0">
        <div className="text-xs font-medium">{label}</div>
        <div className="text-[11px] text-muted-foreground">{description}</div>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function ToggleRow({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-center justify-between px-3 py-2.5 text-xs font-medium">
      {label}
      <input
        type="checkbox"
        className="h-4 w-4 accent-primary"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}

function NumberInput({
  value,
  min,
  max,
  onChange,
}: {
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  return (
    <input
      type="number"
      className="h-8 w-16 rounded border bg-background px-2 text-right text-xs outline-none focus:ring-1 focus:ring-ring"
      value={value}
      min={min}
      max={max}
      onChange={(event) => {
        const next = Number(event.target.value);
        if (Number.isFinite(next)) onChange(next);
      }}
    />
  );
}
