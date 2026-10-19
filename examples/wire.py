"""Small stdlib-only reader for the project's MessagePack array validation fixtures."""
import struct


def unpack(data):
    offset = 0

    def take(n):
        nonlocal offset
        value = data[offset:offset+n]
        if len(value) != n:
            raise ValueError('truncated MessagePack')
        offset += n
        return value

    def number(fmt):
        return struct.unpack('>' + fmt, take(struct.calcsize(fmt)))[0]

    def item(depth=0):
        if depth > 32:
            raise ValueError('excessive nesting')
        code = take(1)[0]
        if code < 128:
            return code
        if code >= 224:
            return code - 256
        if code == 192:
            return None
        if code in (194, 195):
            return code == 195
        if 204 <= code <= 211:
            return number(('B','H','I','Q','b','h','i','q')[code-204])
        if code in (202,203):
            return number('f' if code == 202 else 'd')
        if 160 <= code <= 191 or code in (217,218,219):
            n = code-160 if code < 192 else number({217:'B',218:'H',219:'I'}[code])
            return take(n).decode()
        if 144 <= code <= 159 or code in (220,221):
            n = code-144 if code < 160 else number('H' if code == 220 else 'I')
            if n > len(data):
                raise ValueError('invalid array length')
            return [item(depth+1) for _ in range(n)]
        raise ValueError(f'unsupported MessagePack tag {code}')

    result = item()
    if offset != len(data):
        raise ValueError('trailing MessagePack data')
    return result


def snapshot(frame):
    assert len(frame) == 11 and frame[0] == 1
    values = frame[1]
    assert len(values) == 10 and values[0] == 1
    result = dict(zip(('schema_version','sequence','capture_unix_ns','monotonic_start_ns','monotonic_end_ns','dropped_batches','capacity_overflow','records','task_records','task_losses'), values))
    result['records'] = [dict(zip(('cgroup_id','parent_id','cpu','flags','runtime_ns','user_ns','system_ns'), row)) for row in values[7]]
    result.update(source=frame[2], session_id=frame[5], dictionary_revision=frame[6], dictionary_full=frame[7], dictionary=frame[9])
    return result


def framed(data):
    offset = 0
    result = []
    while offset < len(data):
        length, = struct.unpack_from('<I', data, offset)
        offset += 4
        result.append(snapshot(unpack(data[offset:offset+length])))
        offset += length
    assert offset == len(data)
    return result
