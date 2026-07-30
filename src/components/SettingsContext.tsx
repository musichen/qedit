import { resolveMode } from '@qedit/shared/mode-utils';
import type { Mode } from '@qedit/shared/mode-utils';
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import type { ReactNode } from 'react';

import {
  DEFAULT_SETTINGS,
  loadSettings,
  readSettingsBeforePaint,
  saveSettings,
  sanitizeSettings,
} from '#/lib/settings';
import type { QEditSettings } from '#/lib/settings';

type ResolvedTheme = 'light' | 'dark';

interface SettingsContextValue {
  settings: QEditSettings;
  resolvedTheme: ResolvedTheme;
  setSetting: <Key extends keyof QEditSettings>(
    key: Key,
    value: QEditSettings[Key],
  ) => void;
  setMode: (mode: Mode) => void;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

function applyTheme(theme: ResolvedTheme): void {
  if (typeof document === 'undefined') return;

  document.documentElement.classList.toggle('dark', theme === 'dark');
  document.documentElement.dataset.theme = theme;
}

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<QEditSettings>(() =>
    readSettingsBeforePaint(),
  );
  const [systemTheme, setSystemTheme] = useState<ResolvedTheme>(() =>
    resolveMode('system'),
  );
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  const resolvedTheme: ResolvedTheme =
    settings.mode === 'system' ? systemTheme : resolveMode(settings.mode);

  useEffect(() => {
    let cancelled = false;

    void loadSettings().then((next) => {
      if (!cancelled) {
        settingsRef.current = next;
        setSettings(next);
      }
    });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    applyTheme(resolvedTheme);
  }, [resolvedTheme]);

  useEffect(() => {
    if (settings.mode !== 'system' || typeof window === 'undefined') return;

    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const update = () => setSystemTheme(resolveMode('system'));
    if (media.addEventListener) media.addEventListener('change', update);
    else media.addListener(update);

    return () => {
      if (media.removeEventListener)
        media.removeEventListener('change', update);
      else media.removeListener(update);
    };
  }, [settings.mode]);

  const setSetting = useCallback(
    <Key extends keyof QEditSettings>(
      key: Key,
      value: QEditSettings[Key],
    ): void => {
      const next = sanitizeSettings({ ...settingsRef.current, [key]: value });
      settingsRef.current = next;
      setSettings(next);
      void saveSettings(next);
    },
    [],
  );

  const setMode = useCallback(
    (mode: Mode) => setSetting('mode', mode),
    [setSetting],
  );

  const value = useMemo(
    () => ({ settings, resolvedTheme, setSetting, setMode }),
    [resolvedTheme, setMode, setSetting, settings],
  );

  return (
    <SettingsContext.Provider value={value}>
      {children}
    </SettingsContext.Provider>
  );
}

export function useSettings(): SettingsContextValue {
  const context = useContext(SettingsContext);

  if (!context) {
    throw new Error('useSettings must be used inside SettingsProvider');
  }

  return context;
}

export function useSettingsOrDefault(): SettingsContextValue {
  const context = useContext(SettingsContext);

  return (
    context ?? {
      settings: DEFAULT_SETTINGS,
      resolvedTheme: 'dark',
      setSetting: () => undefined,
      setMode: () => undefined,
    }
  );
}

export { DEFAULT_SETTINGS };
