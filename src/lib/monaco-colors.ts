/**
 * Monaco token rules accept six-digit RGB hex colors without the leading '#'.
 * CSS custom properties can also resolve to named colors, which Monaco rejects
 * when they are used as token foregrounds.
 */
export function toMonacoTokenColor(
  value: string,
  fallback = '#000000',
): string {
  const normalized = value.trim().replace(/^#/, '');
  if (/^[\da-f]{6}$/i.test(normalized)) return normalized;

  const fallbackColor = fallback.trim().replace(/^#/, '');
  return /^[\da-f]{6}$/i.test(fallbackColor) ? fallbackColor : '000000';
}
