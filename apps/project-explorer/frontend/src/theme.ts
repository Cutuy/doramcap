import { useSyncExternalStore } from 'react';

export type Theme = 'light' | 'dark';

const STORAGE_KEY = 'doramcap-explorer-theme';
const systemDark = window.matchMedia('(prefers-color-scheme: dark)');
const listeners = new Set<() => void>();
let chosen = readChoice();

function readChoice(): Theme | null {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    return stored === 'light' || stored === 'dark' ? stored : null;
  } catch {
    return null;
  }
}

function currentTheme(): Theme {
  return chosen ?? (systemDark.matches ? 'dark' : 'light');
}

function applyTheme() {
  document.documentElement.style.colorScheme = currentTheme();
  listeners.forEach(notify => notify());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}

/** Return the active theme: the stored choice, else the system preference. */
export function useTheme(): Theme {
  return useSyncExternalStore(subscribe, currentTheme);
}

/** Apply `theme` and remember it in this browser; it then overrides the system preference. */
export function chooseTheme(theme: Theme) {
  chosen = theme;
  try {
    window.localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    // Storage is blocked: the choice lasts until the page reloads.
  }
  applyTheme();
}

systemDark.addEventListener('change', applyTheme);
applyTheme();
