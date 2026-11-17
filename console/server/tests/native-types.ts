import { execution, holoscan, compose, composePlan, wire, type NativeExecution, type NativeCapabilities } from '../src/native.js';

// Checked by tsc; this function is never executed.
export function checkNativeTypes(engine: NativeExecution, native: NativeCapabilities) {
  // Arrange
  const catalog: Promise<InstanceType<typeof holoscan.Catalog>> = engine.catalog(holoscan.DiscoverRequest.create());
  const profiles = execution.ComposeProfilesRequest.create();
  const run = execution.RunRequest.create({ compose: composePlan.ComposePlan.create() });
  const composeExpected = compose.ConfigurationSnapshot.create();

  // Act / Assert: generated inputs and results remain typed through the native client.
  const names: Promise<InstanceType<typeof execution.ComposeProfiles>> = engine.composeProfiles(profiles);
  const inspected: Promise<InstanceType<typeof execution.Inspection>> = engine.inspect(run);
  const accepted: Promise<void> = engine.start(execution.StartRequest.create({ run, composeExpected }));
  const update: InstanceType<typeof execution.ExecutionUpdate> = engine.poll();
  const config: InstanceType<typeof wire.ConsoleConfig> = native.parseConfig('');
  void [catalog, names, inspected, accepted, update, config];

  // @ts-expect-error A RunRequest cannot be used as a ComposeProfilesRequest.
  engine.composeProfiles(run);
  // @ts-expect-error Start requires its generated request with a backend snapshot.
  engine.start(run);
  // @ts-expect-error Poll returns an ExecutionUpdate message.
  const bytes: Buffer = engine.poll();
  void bytes;
}

function buildTypes(engine: NativeExecution) {
  const outcome: Promise<InstanceType<typeof holoscan.BuildOutcome>> = engine.build(holoscan.BuildRequest.create());
  const lookup: Promise<InstanceType<typeof holoscan.BuildLookup>> = engine.buildLookup(holoscan.BuildStoreRequest.create());
  return { outcome, lookup };
}
void buildTypes;
