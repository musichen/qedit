/** Read a qedit token after the stylesheet has been loaded. */
export function designToken(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback;

  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();

  return value || fallback;
}
