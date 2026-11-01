import { strict as assert } from 'node:assert';

export async function sessionCookie(base: string, origin = 'http://console.test', token = 'secret') {
  const response = await fetch(`${base}/api/session`, {
    method: 'POST', headers: { origin, authorization: `Bearer ${token}` },
  });
  assert.equal(response.status, 204);
  const cookie = response.headers.get('set-cookie');
  assert.ok(cookie);
  assert.ok(cookie.includes('HttpOnly; SameSite=Strict; Path=/'));
  assert.equal(response.headers.get('www-authenticate'), null);
  return cookie.split(';')[0];
}
