import { execution, compose, composePlan, wire, type NativeExecution, type NativeCapabilities } from '../src/native.js';

// Checked by tsc; this function is never executed.
export function checkNativeTypes(engine: NativeExecution, native: NativeCapabilities) {
  // Arrange
  const profiles = execution.ComposeProfilesRequest.create();
  const run = execution.RunRequest.create({ compose: composePlan.ComposePlan.create() });
  const expected = compose.ConfigurationSnapshot.create();

  // Act / Assert: generated inputs and results remain typed through the native client.
  const names: Promise<InstanceType<typeof execution.ComposeProfiles>> = engine.composeProfiles(profiles);
  const inspected: Promise<InstanceType<typeof execution.Inspection>> = engine.inspect(run);
  const accepted: Promise<void> = engine.start(execution.StartRequest.create({ run, expected }));
  const update: InstanceType<typeof execution.ExecutionUpdate> = engine.poll();
  const config: InstanceType<typeof wire.ConsoleConfig> = native.parseConfig('');
  void [names, inspected, accepted, update, config];

  // @ts-expect-error A RunRequest cannot be used as a ComposeProfilesRequest.
  engine.composeProfiles(run);
  // @ts-expect-error Start requires its generated request with a backend snapshot.
  engine.start(run);
  // @ts-expect-error Poll returns an ExecutionUpdate message.
  const bytes: Buffer = engine.poll();
  void bytes;
}
