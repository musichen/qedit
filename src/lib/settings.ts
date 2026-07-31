import type { Mode } from '@qedit/shared/mode-utils';

export interface QEditSettings {
  mode: Mode;
  fontSize: number;
  tabSize: number;
  wordWrap: 'off' | 'on';
  minimap: boolean;
  lineNumbers: boolean;
  terminalFontSize: number;
  terminalPanelHeight: number;
}

export const DEFAULT_SETTINGS: QEditSettings = {
  mode: 'dark',
  fontSize: 14,
  tabSize: 2,
  wordWrap: 'on',
  minimap: true,
  lineNumbers: true,
  terminalFontSize: 12,
  terminalPanelHeight: 208,
};

const STORE_PATH = 'qedit-settings.json';
const STORE_KEY = 'preferences';
const LOCAL_STORAGE_KEY = 'qedit.settings';

function isMode(value: unknown): value is Mode {
  return value === 'dark' || value === 'light' || value === 'system';
}

function asNumber(value: unknown, fallback: number, min: number, max: number) {
  return typeof value === 'number' && Number.isFinite(value)
    ? Math.min(max, Math.max(min, value))
    : fallback;
}

export function sanitizeSettings(value: unknown): QEditSettings {
  const raw = value && typeof value === 'object' ? value : {};

  return {
    mode: isMode((raw as { mode?: unknown }).mode)
      ? (raw as { mode: Mode }).mode
      : DEFAULT_SETTINGS.mode,
    fontSize: asNumber(
      (raw as { fontSize?: unknown }).fontSize,
      DEFAULT_SETTINGS.fontSize,
      10,
      32,
    ),
    tabSize: Math.round(
      asNumber(
        (raw as { tabSize?: unknown }).tabSize,
        DEFAULT_SETTINGS.tabSize,
        1,
        8,
      ),
    ),
    wordWrap: (raw as { wordWrap?: unknown }).wordWrap === 'off' ? 'off' : 'on',
    minimap:
      typeof (raw as { minimap?: unknown }).minimap === 'boolean'
        ? (raw as { minimap: boolean }).minimap
        : DEFAULT_SETTINGS.minimap,
    lineNumbers:
      typeof (raw as { lineNumbers?: unknown }).lineNumbers === 'boolean'
        ? (raw as { lineNumbers: boolean }).lineNumbers
        : DEFAULT_SETTINGS.lineNumbers,
    terminalFontSize: asNumber(
      (raw as { terminalFontSize?: unknown }).terminalFontSize,
      DEFAULT_SETTINGS.terminalFontSize,
      10,
      24,
    ),
    terminalPanelHeight: asNumber(
      (raw as { terminalPanelHeight?: unknown }).terminalPanelHeight,
      DEFAULT_SETTINGS.terminalPanelHeight,
      120,
      800,
    ),
  };
}

function readLocalSettings(): QEditSettings {
  if (typeof window === 'undefined') return DEFAULT_SETTINGS;

  try {
    const value = window.localStorage.getItem(LOCAL_STORAGE_KEY);
    return value ? sanitizeSettings(JSON.parse(value)) : DEFAULT_SETTINGS;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function readSettingsBeforePaint(): QEditSettings {
  return readLocalSettings();
}

function writeLocalSettings(settings: QEditSettings): void {
  if (typeof window === 'undefined') return;

  try {
    window.localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // The Tauri store remains authoritative if localStorage is unavailable.
  }
}

export async function loadSettings(): Promise<QEditSettings> {
  const localSettings = readLocalSettings();

  try {
    const { Store } = await import('@tauri-apps/plugin-store');
    const store = await Store.load(STORE_PATH, { autoSave: false });
    const stored = await store.get<unknown>(STORE_KEY);
    const storedSettings = stored && typeof stored === 'object' ? stored : {};
    const settings = sanitizeSettings({ ...localSettings, ...storedSettings });
    writeLocalSettings(settings);
    return settings;
  } catch {
    // Browser development and tests do not expose Tauri IPC.
    return localSettings;
  }
}

export async function saveSettings(settings: QEditSettings): Promise<void> {
  const safeSettings = sanitizeSettings(settings);
  writeLocalSettings(safeSettings);

  try {
    const { Store } = await import('@tauri-apps/plugin-store');
    const store = await Store.load(STORE_PATH, { autoSave: false });
    await store.set(STORE_KEY, safeSettings);
    await store.save();
  } catch {
    // Browser development and tests fall back to localStorage.
  }
}
