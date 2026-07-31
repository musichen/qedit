import { describe, expect, it } from 'vitest';

import { DEFAULT_SETTINGS, sanitizeSettings } from '../lib/settings';

describe('terminal panel settings', () => {
  it('provides and bounds the persisted preferred height', () => {
    expect(DEFAULT_SETTINGS.terminalPanelHeight).toBe(208);
    expect(
      sanitizeSettings({ terminalPanelHeight: 900 }).terminalPanelHeight,
    ).toBe(800);
    expect(
      sanitizeSettings({ terminalPanelHeight: 20 }).terminalPanelHeight,
    ).toBe(120);
    expect(sanitizeSettings({}).terminalPanelHeight).toBe(208);
  });
});
