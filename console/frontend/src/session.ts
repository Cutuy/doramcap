/** Exchange a launch-link token for an HTTP-only browser session before mounting the app. */
export async function establishSession(location: Pick<Location, 'hash' | 'pathname' | 'search'>,
  history: Pick<History, 'replaceState'>, fetcher: typeof fetch = fetch): Promise<void> {
  const token = new URLSearchParams(location.hash.slice(1)).get('access');
  if (!token) return;
  history.replaceState(null, '', location.pathname + location.search);
  const response = await fetcher('/api/session', {
    method: 'POST', credentials: 'same-origin', headers: { authorization: `Bearer ${token}` },
  });
  if (!response.ok) throw new Error('Access link expired. Open the link printed by the running Console.');
}
