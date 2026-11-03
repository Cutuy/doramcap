import { useEffect, useState } from 'react';

export type Theme = 'light' | 'dark';
// index.html reads the same key before first paint, so a pinned theme never flashes.
const key = 'doramcap-theme';
const dark = () => window.matchMedia('(prefers-color-scheme: dark)');

/** The theme in effect: a pinned choice from the toggle, else the system setting. */
export function resolvedTheme(): Theme {
  const pinned = document.documentElement.dataset.theme;
  return pinned === 'light' || pinned === 'dark' ? pinned : dark().matches ? 'dark' : 'light';
}
/** Pin a theme for this browser and notify listeners. Storage failures keep the choice for this visit. */
export function pinTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
  try { localStorage.setItem(key, theme); } catch { /* private mode or blocked storage */ }
  window.dispatchEvent(new Event('themechange'));
}
/** Re-render on toggle or system theme changes. */
export function useTheme(): Theme {
  const [theme, setTheme] = useState(resolvedTheme);
  useEffect(() => {
    const update = () => setTheme(resolvedTheme());
    const media = dark();
    media.addEventListener('change', update);
    window.addEventListener('themechange', update);
    return () => { media.removeEventListener('change', update); window.removeEventListener('themechange', update); };
  }, []);
  return theme;
}
