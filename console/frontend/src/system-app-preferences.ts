import { LIMITS } from './generated/transport';
import type { doramcap } from './generated/console';

export type SystemAppChoice = Pick<doramcap.console.v1.SystemAppSelection, 'ids' | 'profiles'>;

/** Browser preferences belong to one page, embedded Console and backend project. */
export function systemAppPreferenceKey(pathname: string, basePath: string, projectDirectory: string): string | undefined {
  return projectDirectory ? `doramcap-console-system-apps:v1:${JSON.stringify([pathname, basePath, projectDirectory])}` : undefined;
}

/** Malformed or unavailable storage uses configured defaults; the backend validates available apps. */
export function readSystemAppChoice(key: string | undefined): SystemAppChoice | undefined {
  if (!key) return;
  try {
    const value: unknown = JSON.parse(window.localStorage.getItem(key) ?? 'null');
    if (!value || typeof value !== 'object') return;
    const { ids, profiles } = value as Partial<SystemAppChoice>;
    const validString = (item: unknown): item is string => typeof item === 'string' && item.length > 0 && item.length <= LIMITS.stringLength;
    if (!Array.isArray(ids) || ids.length > LIMITS.selectionLength || !ids.every(validString) || new Set(ids).size !== ids.length) return;
    if (!profiles || typeof profiles !== 'object' || Array.isArray(profiles)) return;
    if (!Object.entries(profiles).every(([id, profile]) => ids.includes(id) && validString(profile))) return;
    return { ids, profiles };
  } catch {
    return;
  }
}

/** Save an accepted choice atomically, or forget a rejected one; failures leave in-tab choices intact. */
export function saveSystemAppChoice(key: string | undefined, choice?: SystemAppChoice): void {
  if (!key) return;
  try {
    if (choice) window.localStorage.setItem(key, JSON.stringify(choice));
    else window.localStorage.removeItem(key);
  } catch {
    // Storage may be blocked or full; the current tab still keeps its accepted choice.
  }
}
