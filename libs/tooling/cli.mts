import { Command, CommanderError } from 'commander';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export { Command };

/** Identify direct execution while keeping an imported command module side-effect free. */
export const isMain = (url: string): boolean => Boolean(process.argv[1] && url === pathToFileURL(resolve(process.argv[1])).href);

/** Parse a Commander CLI; report failures without exiting before resource cleanup finishes. */
export async function cli(command: Command, failureCode = 1): Promise<void> {
  try { await command.exitOverride().parseAsync(); }
  catch (error) {
    if (!(error instanceof CommanderError)) console.error((error as Error).message);
    process.exitCode = error instanceof CommanderError && error.exitCode === 0 ? 0 : failureCode;
  }
}
