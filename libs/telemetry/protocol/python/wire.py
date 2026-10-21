"""Protobuf validation reader; schema shared with Rust and TypeScript.

Install libs/telemetry/protocol/python/requirements.txt. MCAP readers can supply the embedded schema.
"""
import struct
from pathlib import Path
from google.protobuf import descriptor_pb2, descriptor_pool, message_factory


def packet_type(descriptor):
    files = descriptor_pb2.FileDescriptorSet.FromString(descriptor)
    pool = descriptor_pool.DescriptorPool()
    for file in files.file:
        pool.Add(file)
    descriptor = pool.FindMessageTypeByName('doramcap.telemetry.v1.Packet')
    if hasattr(message_factory, 'GetMessageClass'):
        return message_factory.GetMessageClass(descriptor)
    return message_factory.MessageFactory(pool).GetPrototype(descriptor)


Packet = packet_type((Path(__file__).resolve().parents[1] / 'proto/telemetry_descriptor.bin').read_bytes())


def unpack(data, message_type=Packet):
    packet = message_type.FromString(data)
    kind = packet.WhichOneof('body')
    assert kind in ('snapshot', 'dictionary'), 'missing packet body'
    assert getattr(packet, kind).version == 2, 'unsupported wire version'
    return packet


def snapshot(packet):
    assert packet.WhichOneof('body') == 'snapshot'
    frame = packet.snapshot
    values = frame.snapshot
    assert values.schema_version == 1
    fields = ('schema_version','sequence','capture_unix_ns','monotonic_start_ns','monotonic_end_ns','dropped_batches','capacity_overflow')
    result = {name: getattr(values, name) for name in fields}
    fields = ('cgroup_id','parent_id','cpu','flags','runtime_ns','user_ns','system_ns')
    result['records'] = [{name: getattr(row, name) for name in fields} for row in values.records]
    result.update(source=frame.source, session_id=frame.session_id, dictionary_revision=frame.dictionary_revision, dictionary_full=frame.dictionary_full, dictionary=list(frame.dictionary))
    return result


def framed(data):
    offset = 0
    result = []
    while offset < len(data):
        length, = struct.unpack_from('<I', data, offset)
        offset += 4
        assert offset + length <= len(data), 'truncated packet'
        result.append(snapshot(unpack(data[offset:offset+length])))
        offset += length
    assert offset == len(data)
    return result
