import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { test } from 'node:test';
import { McapStreamReader } from '@mcap/core';
import type { TypedMcapRecords } from '@mcap/core';
import protobuf from 'protobufjs';
import 'protobufjs/ext/descriptor/index.js';

const directory = process.argv[2];
assert.ok(directory, 'usage: npm test -- FIXTURE_DIRECTORY');
const BASE_TIME = 1_800_000_000_000_000_001n;
const LARGE_COUNTER = '9007199254740993';
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
            assert.equal(record.summaryStart, 0n);
            footerSeen = true;
          } else {
            assert.notEqual(record.type, 'Chunk', 'fixtures must exercise the unchunked writer');
            assert.notEqual(record.type, 'MessageIndex');
            assert.notEqual(record.type, 'ChunkIndex');
          }
        }
      }

      // Assert: each rotated file independently supplies its decoding descriptors.
      assert.equal(reader.done(), true);
      assert.equal(reader.bytesRemaining(), 0);
      assert.equal(footerSeen, true);
      assert.equal(messages.length, expected.length);
      for (const [sequence, message] of messages.entries()) {
        assert.equal(message.sequence, sequence);
        assert.equal(message.logTime, BASE_TIME + BigInt(sequence));
        assert.equal(message.publishTime, BASE_TIME + BigInt(sequence) + 1n);
        assert.deepEqual(Buffer.from(message.data), expected[sequence]);
        const channel = channels.get(message.channelId);
        assert.ok(channel);
        assert.equal(channel.topic, sequence % 2 === 0 ? 'primary' : 'secondary');
        assert.equal(channel.messageEncoding, 'protobuf');
        const schema = schemas.get(channel.schemaId);
        assert.ok(schema, 'each protobuf channel needs its embedded descriptor');
        const decoded = schema.type.toObject(schema.type.decode(message.data), { longs: String, bytes: Array });
        if (filename.startsWith('cpu-')) {
          assert.equal(schema.schema.name, 'doramcap.telemetry.v1.Packet');
          assert.equal(decoded.snapshot.version, 2);
          assert.equal(decoded.snapshot.snapshot.sequence, LARGE_COUNTER);
          assert.equal(decoded.snapshot.snapshot.captureUnixNs, BASE_TIME.toString());
          assert.equal(decoded.snapshot.snapshot.records[0].cgroupId, LARGE_COUNTER);
          assert.equal(decoded.snapshot.snapshot.records[0].cpu, 3);
          assert.equal(decoded.snapshot.snapshot.records[0].runtimeNs, LARGE_COUNTER);
          assert.equal(decoded.snapshot.sessionId, 'reader-roundtrip');
          assert.equal(decoded.snapshot.dictionaryRevision, LARGE_COUNTER);
          assert.equal(decoded.snapshot.dictionaryFull, true);
          assert.equal(decoded.snapshot.dictionary[0].path, '/roundtrip/測試');
        } else {
          assert.equal(schema.schema.name, 'doramcap.fault.v1.AggregationPublication');
          assert.equal(decoded.version, 1);
          assert.equal(decoded.publishedUnixNs, BASE_TIME.toString());
          assert.equal(decoded.emittedAtNs, LARGE_COUNTER);
          assert.deepEqual(decoded.snapshot.epoch, [...new Uint8Array(15), 1]);
          assert.equal(decoded.snapshot.revision, LARGE_COUNTER);
          assert.equal(decoded.snapshot.stats.droppedUpdates, LARGE_COUNTER);
          assert.equal(decoded.snapshot.stats.rejectedInputs, '2');
          assert.equal(decoded.snapshot.stats.tooOld, '3');
        }
      }
    });
  }
}
