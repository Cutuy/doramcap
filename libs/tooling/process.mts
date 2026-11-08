import { execaSync } from 'execa';

/** Run argv directly with inherited output; nonzero exits, signals and spawn failures throw ExecaError. */
export const run = execaSync({ stdio: 'inherit' });

/** Capture complete UTF-8 output, preserving trailing newlines; failures throw ExecaError. */
export const capture = execaSync({ stripFinalNewline: false, maxBuffer: Infinity });
