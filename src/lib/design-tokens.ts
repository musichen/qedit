export type TokenTheme = 'light' | 'dark';

export type TokenReader = (name: string, fallback: string) => string;

/**
 * Resolve tokens as they would compute under `theme`, regardless of the theme
 * currently applied to the document. Canvas-based surfaces (Monaco, xterm)
 * cannot inherit CSS variables, so they must snapshot values; reading through
 * a detached probe keeps that snapshot correct without depending on when the
 * document's own `data-theme` attribute is updated.
 */
export function withThemeTokens<T>(
  theme: TokenTheme,
  build: (token: TokenReader) => T,
): T {
  if (typeof document === 'undefined') {
    return build((_name, fallback) => fallback);
  }

  const probe = document.createElement('div');
  probe.dataset.theme = theme;
  probe.style.display = 'none';
  document.body.appendChild(probe);

  try {
    const styles = getComputedStyle(probe);

    return build(
      (name, fallback) => styles.getPropertyValue(name).trim() || fallback,
    );
  } finally {
    probe.remove();
  }
}
