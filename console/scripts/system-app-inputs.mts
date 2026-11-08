// Copy app-owned inputs that the release system apps mount at launch.
import { cpSync } from 'node:fs';
import { join } from 'node:path';

const DESCRIPTOR = 'libs/telemetry/protocol/proto/telemetry_descriptor.bin';
const WATCHDOG_PROFILES = 'apps/fault-watchdog/deploy/standalone/config';

/** Copy descriptor and watchdog profiles from repository `root` into `systemApps`; throws if a source is missing. */
export function stageSystemAppInputs(root: string, systemApps: string) {
  cpSync(join(root, DESCRIPTOR), join(systemApps, 'data-recorder/telemetry_descriptor.bin'));
  for (const [source, target] of [['watchdog.pbtxt', 'fault-watchdog-live.pbtxt'], ['watchdog-mcap.pbtxt', 'fault-watchdog-mcap.pbtxt'], ['watchdog-otel.pbtxt', 'fault-watchdog-otel.pbtxt']])
    cpSync(join(root, WATCHDOG_PROFILES, source), join(systemApps, 'fault-watchdog', target));
}
