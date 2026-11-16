import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { test } from 'node:test';
import { McapStreamReader, McapIndexedReader } from '@mcap/core';
import type { TypedMcapRecords } from '@mcap/core';
import protobuf from 'protobufjs';
import 'protobufjs/ext/descriptor/index.js';

const directory = process.argv[2];
assert.ok(directory, 'usage: npm test -- FIXTURE_DIRECTORY');
const BASE_TIME = 1_800_000_000_000_000_001n;
const INPUT_SIZES = [17, 4093];
const EXPECTED_FILES = 18;
const files = await readdir(directory);
const recordings = files.filter(file => file.endsWith('.mcap')).sort();
assert.equal(recordings.length, EXPECTED_FILES, 'all paths and both rotations must be present');

for (const filename of recordings) {
  for (const inputSize of INPUT_SIZES) {
    test(`${filename}: streaming fragments of ${inputSize} bytes preserve protobuf fields`, async () => {
      // Arrange: expected payloads were saved before the real writer accepted them.
      const prefix = filename.replace(/-\d+\.mcap$/, '');
      const expected = await Promise.all(files.filter(file => file.startsWith(`${prefix}-`) && file.endsWith('.bin'))
        .sort((a, b) => Number(a.match(/-(\d+)\.bin$/)![1]) - Number(b.match(/-(\d+)\.bin$/)![1]))
        .map(file => readFile(join(directory, file))));
      assert.ok(expected.length > 0, 'missing source payloads');
      const data = await readFile(join(directory, filename));
      const reader = new McapStreamReader({ includeChunks: true, validateCrcs: true });
      const schemas = new Map<number, { schema: TypedMcapRecords['Schema']; type: protobuf.Type }>();
      const channels = new Map<number, TypedMcapRecords['Channel']>();
      const messages: TypedMcapRecords['Message'][] = [];
      let footerSeen = false;
      const chunks: TypedMcapRecords['Chunk'][] = [];
      const chunkIndexes: TypedMcapRecords['ChunkIndex'][] = [];
      const messageIndexes: TypedMcapRecords['MessageIndex'][] = [];

      // Act: fragment headers, payloads and private padding across input boundaries.
      for (let offset = 0; offset < data.length; offset += inputSize) {
        reader.append(data.subarray(offset, offset + inputSize));
        for (let record; (record = reader.nextRecord());) {
          if (record.type === 'Schema') {
            assert.equal(record.encoding, 'protobuf');
            const root = protobuf.Root.fromDescriptor(record.data);
            schemas.set(record.id, { schema: record, type: root.lookupType(record.name) });
          } else if (record.type === 'Channel') {
            channels.set(record.id, record);
          } else if (record.type === 'Message') {
            messages.push(record);
          } else if (record.type === 'Footer') {
            assert.ok(record.summaryStart > 0n);
            assert.ok(record.summaryOffsetStart > record.summaryStart);
            footerSeen = true;
          } else if (record.type === 'Chunk') {
            chunks.push(record);
          } else if (record.type === 'MessageIndex') {
            messageIndexes.push(record);
          } else if (record.type === 'ChunkIndex') {
            chunkIndexes.push(record);
          }
        }
      }

      // Assert: each rotated file independently supplies its decoding descriptors.
      assert.equal(reader.done(), true);
      assert.equal(reader.bytesRemaining(), 0);
      assert.equal(footerSeen, true);
      assert.equal(messages.length, expected.length);
      const chunked = !filename.includes('external-');
      assert.equal(chunks.length, chunkIndexes.length);
      assert.equal(chunks.length > 0, chunked);
      if (chunked) {
        for (const index of chunkIndexes) {
          const start = Number(index.chunkStartOffset);
          assert.equal(start % 4096, 0);
          assert.equal(data[start], 6);
          const offsets = [...index.messageIndexOffsets.values()];
          if (offsets.length > 0) {
            assert.equal(offsets.reduce((a, b) => a < b ? a : b), index.chunkStartOffset + index.chunkLength);
          } else {
            assert.equal(index.messageIndexLength, 0n);
          }
        }
        assert.equal(messageIndexes.length, chunkIndexes.reduce((n, i) => n + i.messageIndexOffsets.size, 0));
        if (filename.startsWith('cpu-reserved')) {
          assert.ok(chunkIndexes.some(i => i.messageIndexOffsets.size === 0), 'zero-slack chunk');
          assert.ok(chunkIndexes.some(i => i.messageIndexOffsets.size > 0), 'indexed chunk');
        }
        const indexed = McapIndexedReader.Initialize({ readable: {
          size: async () => BigInt(data.length),
          read: async (offset, length) => data.subarray(Number(offset), Number(offset + length)),
        } });
        if (chunkIndexes.every(i => i.messageIndexOffsets.size > 0)) {
          const indexedReader = await indexed;
          const found = [];
          for await (const msg of indexedReader.readMessages()) found.push(msg);
          assert.equal(found.length, expected.length);
        } else {
          await assert.rejects(indexed, /without message indexes/);
        }
      }
      // Sequential topic/time filtering includes messages in chunks without indexes.
      assert.equal(messages.filter(m => m.channelId === 1 && m.logTime >= BASE_TIME).length,
        expected.filter((_, sequence) => sequence % 2 === 0).length);
      for (const [sequence, message] of messages.entries()) {
        assert.equal(message.sequence, sequence);
        assert.equal(message.logTime, BASE_TIME + BigInt(sequence));
        assert.equal(message.publishTime, BASE_TIME + BigInt(sequence) + 1n);
        const recordedPayload = Buffer.from(message.data);
        assert.deepEqual(recordedPayload, expected[sequence]);
        const channel = channels.get(message.channelId);
        assert.ok(channel);
        assert.equal(channel.topic, sequence % 2 === 0 ? 'primary' : 'secondary');
        assert.equal(channel.messageEncoding, 'protobuf');
        const schema = schemas.get(channel.schemaId);
        assert.ok(schema, 'each protobuf channel needs its embedded descriptor');
        assert.equal(schema.schema.name, filename.startsWith('cpu-')
          ? 'doramcap.telemetry.v1.Packet' : 'doramcap.fault.v1.AggregationPublication');
        assert.deepEqual(schema.type.decode(recordedPayload), schema.type.decode(expected[sequence]));
      }
    });
  }
}
