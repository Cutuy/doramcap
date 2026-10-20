import { doramcap } from "./generated/activity.js";
export const Batch = doramcap.activity.v1.Batch;
type Integer = number | {
    toString(): string;
} | null | undefined;
export const integer = (value: Integer): string => value?.toString() ?? "0";
export interface TraceSpan {
    id: string;
    root: string;
    parent: string;
    name: string;
    worker: string;
    tid: number;
    start: number;
    end?: number;
    lastSeen?: number;
    cqeAt?: number;
    partial: boolean;
    fields: Record<string, string>;
    links: string[];
}
export interface TraceEvent {
    id: string;
    span: string;
    root: string;
    time: number;
    worker: string;
    name: string;
    fields: Record<string, string>;
}
export class TraceStore {
    session = "";
    lastSequence = 0n;
    gaps = 0;
    producerLoss = 0;
    fieldLoss = 0;
    evicted = 0;
    received = 0;
    latest = 0;
    lastArrival = 0;
    spans = new Map<string, TraceSpan>();
    events: TraceEvent[] = [];
    ingest(bytes: Uint8Array): boolean {
        const batch = Batch.decode(bytes);
        if (batch.version !== 1)
            throw new Error("Unsupported activity schema");
        const session = integer(batch.session);
        if (session !== this.session) {
            this.session = session;
            this.lastSequence = 0n;
            this.spans.clear();
            this.events = [];
            this.gaps = 0;
            this.evicted = 0;
            this.received = 0;
            this.latest = 0;
        }
        const sequence = BigInt(integer(batch.sequence));
        if (sequence <= this.lastSequence)
            return false;
        if (this.lastSequence > 0n && sequence > this.lastSequence + 1n)
            this.gaps += Number(sequence - this.lastSequence - 1n);
        if (this.lastSequence === 0n && sequence > 1n)
            this.gaps += Number(sequence - 1n);
        if (sequence > this.lastSequence + 1n) {
            for (const span of this.spans.values())
                if (span.end === undefined)
                    span.partial = true;
        }
        this.lastSequence = sequence;
        this.producerLoss =
            Number(integer(batch.droppedRecords)) +
                Number(integer(batch.rejectedSpans)) +
                Number(integer(batch.unregisteredRecords));
        this.fieldLoss = Number(integer(batch.droppedFields));
        const names = batch.dictionary;
        const decoder = new TextDecoder("utf-8", { fatal: true });
        const name = (id: number) => {
            if (id < 0 || id >= names.length)
                throw new Error("Invalid dictionary index");
            return names[id];
        };
        for (const [index, r] of batch.records.entries()) {
            const record = new doramcap.activity.v1.Record(r);
            const time = Number(integer(record.monotonicNs)) / 1e9;
            this.latest = Math.max(this.latest, time);
            const id = `${session}:${integer(record.spanId)}`, root = `${session}:${integer(record.traceRoot)}`, parent = `${session}:${integer(record.parentId)}`;
            const worker = name(record.threadName);
            const fields: Record<string, string> = {};
            for (const value of record.fields) {
                const f = new doramcap.activity.v1.Field(value);
                const key = name(f.name);
                const bits = BigInt(integer(f.bits));
                switch (f.kind) {
                    case 0:
                        fields[key] = bits.toString();
                        break;
                    case 1:
                        fields[key] = BigInt.asIntN(64, bits).toString();
                        break;
                    case 2: {
                        const view = new DataView(new ArrayBuffer(8));
                        view.setBigUint64(0, bits, true);
                        fields[key] = String(view.getFloat64(0, true));
                        break;
                    }
                    case 3:
                        fields[key] = bits === 0n ? "false" : "true";
                        break;
                    case 4:
                        if (f.offset + f.length > batch.strings.length)
                            throw new Error("Invalid string range");
                        fields[key] = decoder.decode(batch.strings.subarray(f.offset, f.offset + f.length));
                        break;
                    default:
                        throw new Error("Invalid field kind");
                }
            }
            if (record.kind === 1) {
                this.events.push({
                    id: `${session}:${sequence}:${index}`,
                    span: id,
                    root,
                    time,
                    worker,
                    name: name(record.name),
                    fields,
                });
            }
            else {
                let span = this.spans.get(id);
                if (!span) {
                    span = {
                        id,
                        root,
                        parent,
                        name: name(record.name),
                        worker,
                        tid: record.tid,
                        start: time,
                        partial: record.kind !== 0,
                        fields: {},
                        links: [],
                    };
                    this.spans.set(id, span);
                }
                if (record.kind === 0) {
                    span.start = time;
                    span.partial = false;
                    span.name = name(record.name);
                    span.parent = parent;
                }
                span.lastSeen = Math.max(span.lastSeen ?? span.start, time);
                Object.assign(span.fields, fields);
                if (record.kind === 4)
                    span.end = time;
                if (record.kind === 3)
                    span.links.push(`${session}:${integer(record.linkedId)}`);
            }
            this.received++;
        }
        // CQE is independent evidence of completion even when the END batch was lost.
        for (const event of this.events) {
            const span = this.spans.get(event.span);
            if (!span) continue;
            span.lastSeen = Math.max(span.lastSeen ?? span.start, event.time);
            if (span.name === 'storage.io' && event.fields.message === 'cqe_observed')
                span.cqeAt = event.time;
        }
        for (const span of this.spans.values()) {
            if (span.end === undefined && this.spans.get(span.parent)?.end !== undefined)
                span.partial = true;
        }
        this.lastArrival = performance.now();
        for (const [id, span] of this.spans) {
            if ((span.end ?? span.start) < this.latest - 65) {
                this.spans.delete(id);
                this.evicted++;
            }
        }
        while (this.spans.size > 2000) {
            this.spans.delete(this.spans.keys().next().value!);
            this.evicted++;
        }
        this.events = this.events
            .filter((e) => e.time >= this.latest - 65)
            .slice(-600);
        return true;
    }
    now(): number {
        return (this.latest +
            (this.lastArrival
                ? Math.min(10000, performance.now() - this.lastArrival) / 1000
                : 0));
    }
}
export const colors: Record<string, string> = { combined: "#b8ef8d", large: "#8fbeed", slab: "#d7a1d9", mapped: "#72d6c4", managed: "#99a7bb", combining: "#f0c37a" };
export function rootOf(span: TraceSpan, spans: Map<string, TraceSpan>): TraceSpan {
    const seen = new Set<string>();
    while (spans.has(span.parent) && !seen.has(span.id)) {
        seen.add(span.id);
        span = spans.get(span.parent)!;
    }
    return span;
}
/** A missing END is not proof that an operation is still running. */
export function spanExtent(span: TraceSpan, spans: Map<string, TraceSpan>, now: number): {time: number; status: string; uncertain: boolean} {
    if (span.end !== undefined) return {time: span.end, status: 'Closed', uncertain: false};
    if (span.cqeAt !== undefined) return {time: span.cqeAt, status: 'CQE observed · END missing', uncertain: true};
    const parentEnd = spans.get(span.parent)?.end;
    if (parentEnd !== undefined) return {time: Math.max(span.start, parentEnd), status: 'Parent closed · END missing (upper bound)', uncertain: true};
    if (span.partial) return {time: span.lastSeen ?? span.start, status: 'Unknown · history incomplete', uncertain: true};
    return {time: now, status: 'Open', uncertain: false};
}
export function rows(store: TraceStore, left: number, right: number, filter: string) {
    return [...store.spans.values()].filter(s => rootOf(s, store.spans) === s &&
        s.start <= right && spanExtent(s, store.spans, right).time >= left &&
        [s.name, ...Object.values(s.fields)].join(" ").toLowerCase().includes(filter.toLowerCase()))
        .sort((a, b) => b.start - a.start);
}

export interface BufferLane {
    name: string;
    entries: { span: TraceSpan; track: number }[];
    tracks: number;
}
/** Pack visible instances within fixed buffer lanes, including the minimum bar width. */
export function bufferLanes(store: TraceStore, left: number, right: number, filter: string): BufferLane[] {
    const lanes = new Map<string, BufferLane>();
    for (const name of ['combining', 'combined', 'large', 'slab'])
        lanes.set(name, { name, entries: [], tracks: 1 });
    const ends = new Map<string, number[]>();
    const spacing = (right - left) * 4 / 840;
    for (const span of rows(store, left, right, filter).sort((a, b) => a.start - b.start)) {
        const name = span.name === 'storage.combining' ? 'combining' : span.fields.path ?? 'partial';
        if (!lanes.has(name)) lanes.set(name, { name, entries: [], tracks: 1 });
        const lane = lanes.get(name)!;
        const occupied = ends.get(name) ?? [];
        const start = Math.max(left, span.start);
        let track = occupied.findIndex(end => end <= start);
        if (track < 0) track = occupied.length;
        occupied[track] = Math.max(start + spacing, spanExtent(span, store.spans, right).time + spacing);
        ends.set(name, occupied);
        lane.entries.push({ span, track });
        lane.tracks = Math.max(lane.tracks, occupied.length);
    }
    return [...lanes.values()];
}
