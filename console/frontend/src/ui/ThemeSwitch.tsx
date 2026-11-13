import { Moon, Sun } from 'lucide-react';
import { chooseTheme, useTheme } from './theme';
export { chooseTheme, useTheme, type Theme } from './theme';

/** Toggle between light and dark; hosts style `.theme-switch`, `.theme-track` and `.theme-thumb`. */
export function ThemeSwitch() {
  const dark = useTheme() === 'dark';
  const Icon = dark ? Moon : Sun;
  return <button type="button" role="switch" aria-checked={dark} aria-label="Dark mode" className="theme-switch" onClick={() => chooseTheme(dark ? 'light' : 'dark')}><span className="theme-track"><span className="theme-thumb"><Icon size={11} strokeWidth={2.2} aria-hidden="true" /></span></span></button>;
}
