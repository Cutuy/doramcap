import { constants } from 'node:fs';
import { open } from 'node:fs/promises';

/** Create a private file and sync it; fails if the path exists or is a symlink. */
export async function writeOnce(path: string, bytes: Uint8Array) {
  const file = await open(
    path,
    constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY | constants.O_NOFOLLOW,
    0o600,
  );
  try {
    await file.writeFile(bytes);
    await file.sync();
  } finally {
    await file.close();
  }
}
