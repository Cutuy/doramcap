import {doramcap} from './generated/snapshot.js';
const Snapshot = doramcap.storage.v1.Snapshot;
// Only identity fields remain strings. Numeric dashboard counters must be safe integers.
const ids = new Set(['id','writer','file','write','operation','region','slabUse','pool','cursor','use']);
function normalize(value: unknown, key = ''): unknown {
    if (value === null || value === undefined) return value;
    if (value instanceof Uint8Array) return Array.from(value);
    if (typeof value === 'object' && 'low' in value && 'high' in value) {
        const text = value.toString();
        if (ids.has(key)) return text;
        const n = Number(text);
        if (!Number.isSafeInteger(n)) throw Error(`Unsafe numeric counter: ${key}`);
        return n;
    }
    if (Array.isArray(value)) return value.map(v => normalize(v));
    if (typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([k,v])=>[k,normalize(v,k)]));
    return value;
}
export function decodeSnapshot(bytes: Uint8Array): any {
    const packet = Snapshot.decode(bytes);
    if (packet.version !== 1) throw Error('Unsupported storage snapshot version');
    return normalize(Snapshot.toObject(packet,{defaults:true,arrays:true,objects:true}));
}
