import { execution, compose, wire, type NativeExecution, type NativeCapabilities } from '../src/native.js';

// Checked by tsc; this function is never executed.
export function checkNativeTypes(engine: NativeExecution, native: NativeCapabilities) {
  // Arrange
  const profiles = execution.ProfilesRequest.create();
  const run = execution.RunRequest.create();
  const expected = compose.ConfigurationSnapshot.create();

  // Act / Assert: generated inputs and results remain typed through the native client.
  const names: Promise<InstanceType<typeof execution.Profiles>> = engine.profiles(profiles);
  const inspected: Promise<InstanceType<typeof execution.Inspection>> = engine.inspect(run);
  const accepted: Promise<void> = engine.start(run, expected);
  const update: InstanceType<typeof execution.ExecutionUpdate> = engine.poll();
  const config: InstanceType<typeof wire.ConsoleConfig> = native.parseConfig('');
  void [names, inspected, accepted, update, config];

  // @ts-expect-error Serialized bytes are private to the adapter.
  engine.inspect(Buffer.alloc(0));
  // @ts-expect-error A ProfilesRequest cannot be used as a run.
  engine.start(profiles, expected);
  // @ts-expect-error A RunRequest cannot be used as a ProfilesRequest.
  engine.profiles(run);
  // @ts-expect-error Start requires the inspected configuration snapshot.
  engine.start(run);
  // @ts-expect-error Serialized bytes are private to the adapter.
  engine.start(Buffer.alloc(0), expected);
  // @ts-expect-error Poll returns an ExecutionUpdate message.
  const bytes: Buffer = engine.poll();
  void bytes;
}
