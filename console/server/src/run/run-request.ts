import { launch, type Config, type Run } from '../native.js';
import { composeRequest } from './backends/compose.js';
import { systemdRequest } from './backends/systemd.js';
import type { RunInputs } from './run-plan.js';

/** Encode resolved run inputs using the deployment's native launch form. */
export function buildRunRequest(config: Config, run: RunInputs): Promise<Run> {
  if (config.form === launch.Form.HOLOSCAN)
    return Promise.reject(new Error('Holoscan Prepare and Start are not available yet'));
  return config.form === launch.Form.SYSTEMD ? systemdRequest(config, run) : composeRequest(config, run);
}
