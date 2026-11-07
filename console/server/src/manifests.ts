import { readFileSync } from 'node:fs';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { capabilities, compose, launch, type Config } from './native.js';

/** Resolve `file` beside a manifest; throws when it is absolute or escapes `directory`. */
function beside(directory: string, file: string) {
  const target = resolve(directory, file), inside = relative(directory, target);
  if (!file || isAbsolute(file) || inside === '..' || inside.startsWith(`..${sep}`))
    throw new Error(`Manifest path must stay beside it: ${file}`);
  return target;
}

/**
 * Replace each app's `manifest` with the launch facts it names, resolved beside the manifest.
 * Errors: a form other than COMPOSE or SYSTEMD, SYSTEMD without absolute binaries, a relative manifest path, launch fields declared beside a
 * manifest, an unreadable or invalid manifest, or a manifest path escaping its directory.
 */
export function expandManifests(config: Config) {
  const form = config.form ?? launch.Form.FORM_UNSPECIFIED;
  if (![launch.Form.FORM_UNSPECIFIED, launch.Form.COMPOSE, launch.Form.SYSTEMD].includes(form))
    throw new Error('form: only COMPOSE and SYSTEMD are supported');
  if (form === launch.Form.SYSTEMD && !isAbsolute(config.binaries ?? ''))
    throw new Error('binaries must be absolute for SYSTEMD');
  for (const app of config.systemApps) {
    if (!app.manifest) continue;
    if (!isAbsolute(app.manifest)) throw new Error('system app manifest must be absolute');
    if (app.id || app.label || app.input || app.graph || app.configMount || app.program ||
        app.recordings?.length || app.outputMounts?.length || app.launchProfiles?.length)
      throw new Error(`Launch fields belong in the manifest: ${app.manifest}`);
    const manifest = capabilities.parseManifest(readFileSync(app.manifest, 'utf8'));
    const directory = dirname(app.manifest);
    const files = manifest.compose?.files ?? [];
    files.forEach((file) => beside(directory, file));
    app.id = manifest.id;
    app.label = manifest.label;
    app.input = compose.ComposeProject.create({ directory, files });
    app.graph = manifest.graph ? beside(directory, manifest.graph) : '';
    app.outputMounts = manifest.compose?.outputMounts ?? [];
    app.configMount = manifest.compose?.configMount ?? null;
    app.recordings = manifest.recordings;
    app.launchProfiles = manifest.variants.map((variant) => ({
      id: variant.id,
      label: variant.label,
      config: beside(directory, variant.config ?? ''),
      graph: variant.graph ? beside(directory, variant.graph) : '',
      recordings: variant.recordings,
    }));
    app.program = manifest.program
      ? {
          ...manifest.program,
          files: (manifest.program.files ?? []).map((file) => ({ source: beside(directory, file.source ?? ''), target: file.target })),
        }
      : null;
    app.manifest = '';
  }
}
