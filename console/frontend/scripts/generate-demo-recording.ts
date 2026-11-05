// Regenerate the built-in run's recording: npm run generate:demo-recording
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { McapWriter } from '@mcap/core';
import protobuf from 'protobufjs';
import 'protobufjs/ext/descriptor/index.js';
import { BUILTIN_STARTED_AT_MS } from '../src/demo/client';
import { RUN_SAMPLES, SAMPLE_MS, cpuPacket, dictionaryPacket } from '../src/demo/telemetry';

const SCHEMA_NAME = 'doramcap.telemetry.v1.Packet';
const SAMPLES_TOPIC = 'doramcap/cpuacct', DICTIONARY_TOPIC = 'doramcap/cpuacct/dictionary';
const NS_PER_MS = 1_000_000n;
const output = fileURLToPath(new URL('../src/demo/builtin-cpu.mcap', import.meta.url));
const proto = fileURLToPath(new URL('../../../libs/telemetry/protocol/proto/telemetry.proto', import.meta.url));

const root = await new protobuf.Root().load(proto);
// protobufjs extends Root with descriptor conversion when the extension above is imported.
const descriptor = (root as unknown as { toDescriptor(syntax: string): { constructor: { encode(message: unknown): { finish(): Uint8Array } } } }).toDescriptor('proto3');
const chunks: Uint8Array[] = [];
let position = 0n;
const writer = new McapWriter({ writable: {
  position: () => position,
  write: async (bytes: Uint8Array) => { chunks.push(bytes.slice()); position += BigInt(bytes.length); },
} });
await writer.start({ library: 'doramcap-demo', profile: '' });
const schemaId = await writer.registerSchema({ name: SCHEMA_NAME, encoding: 'protobuf', data: descriptor.constructor.encode(descriptor).finish() });
const channelId = await writer.registerChannel({ schemaId, topic: SAMPLES_TOPIC, messageEncoding: 'protobuf', metadata: new Map() });
const dictionaryId = await writer.registerChannel({ schemaId, topic: DICTIONARY_TOPIC, messageEncoding: 'protobuf', metadata: new Map() });
const startTime = BigInt(BUILTIN_STARTED_AT_MS) * NS_PER_MS;
await writer.addMessage({ channelId: dictionaryId, sequence: 0, logTime: startTime, publishTime: startTime, data: dictionaryPacket(BUILTIN_STARTED_AT_MS) });
for (let sample = 1; sample <= RUN_SAMPLES; sample++) {
  const logTime = (BigInt(BUILTIN_STARTED_AT_MS) + BigInt(sample * SAMPLE_MS)) * NS_PER_MS;
  await writer.addMessage({ channelId, sequence: sample, logTime, publishTime: logTime, data: cpuPacket(sample, BUILTIN_STARTED_AT_MS) });
}
await writer.end();
const file = Buffer.concat(chunks);
writeFileSync(output, file);
console.log(`${output}: ${file.length} bytes`);
