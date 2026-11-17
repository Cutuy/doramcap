import { McapWriter } from '@mcap/core';
import { telemetry } from '../src/native.js';

export const cpuPacket = (body: object) => telemetry.Packet.encode(telemetry.Packet.fromObject(body)).finish();
export const dictionary = (path: string, sessionId = 'session', revision = 1) => cpuPacket({ dictionary: {
  version: 2, sessionId, revision, entries: [
    { id: '9007199254740993', parentId: 1, path },
    { id: 12, parentId: '9007199254740993', path: path + '/container' },
  ],
} });
export const snapshot = (sequence: number, end: bigint, runtimes: bigint[], fields: object = {}) => cpuPacket({ snapshot: {
  version: 2, sessionId: 'session', dictionaryRevision: 1, snapshot: {
    schemaVersion: 1, sequence, monotonicEndNs: String(end),
    records: [
      ...runtimes.map((value, cpu) => ({ cgroupId: '9007199254740993', parentId: 1, cpu, flags: 5, runtimeNs: String(value) })),
      { cgroupId: 12, parentId: '9007199254740993', cpu: 0, flags: 5, runtimeNs: String(end * 50n) },
      { cgroupId: 1, parentId: 0, cpu: 0, flags: 21, runtimeNs: String(end * 100n) },
    ],
  }, ...fields,
} });

/** Independent official writer, with all optional index/summary records disabled. */
export async function cpuRecording(packets: Uint8Array[]) {
  const chunks: Uint8Array[] = [];
  let position = 0n;
  const writer = new McapWriter({
    useChunks: false, useStatistics: false, useSummaryOffsets: false, useMessageIndex: false, useChunkIndex: false,
    useAttachmentIndex: false, useMetadataIndex: false, repeatSchemas: false, repeatChannels: false,
    writable: { position: () => position, write: async bytes => { chunks.push(bytes.slice()); position += BigInt(bytes.length); } },
  });
  await writer.start({ library: 'cpu-comparison-test', profile: '' });
  const schemaId = await writer.registerSchema({ name: 'doramcap.telemetry.v1.Packet', encoding: 'protobuf', data: new Uint8Array() });
  const channelId = await writer.registerChannel({ schemaId, topic: 'doramcap/cpuacct', messageEncoding: 'protobuf', metadata: new Map() });
  for (const [sequence, data] of packets.entries()) {
    const time = BigInt(sequence);
    await writer.addMessage({ channelId, sequence, logTime: time, publishTime: time, data });
  }
  await writer.end();
  return Buffer.concat(chunks);
}
