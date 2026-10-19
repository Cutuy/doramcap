import type { Resources } from "./Resources";
import { doramcap } from "./generated/activity.js";
export const Batch = doramcap.activity.v1.Batch;
type Integer = number | { toString(): string } | null | undefined;
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
export interface DemoStatus {
  frequency_hz: number;
  conversations: number;
  resources: Resources;
  sleep: { mu_ms: number; sigma_ms: number };
  workers: string[];
  paused: boolean;
  published: number;
  received: number;
  mailbox_drops: number;
  capture_drops: number;
  field_losses: number;
  recording: string;
}
export class TraceStore {
  session = "";
  lastSequence = 0n;
  gaps = 0;
  producerLoss = 0;
  fieldLoss = 0;
  received = 0;
  latest = 0;
  lastArrival = 0;
  spans = new Map<string, TraceSpan>();
  events: TraceEvent[] = [];
  ingest(bytes: Uint8Array): boolean {
    const batch = Batch.decode(bytes);
    if (batch.version !== 1) throw new Error("Unsupported activity schema");
    const session = integer(batch.session);
    if (session !== this.session) {
      this.session = session;
      this.lastSequence = 0n;
      this.spans.clear();
      this.events = [];
      this.gaps = 0;
      this.received = 0;
      this.latest = 0;
    }
    const sequence = BigInt(integer(batch.sequence));
    if (sequence <= this.lastSequence) return false;
    if (this.lastSequence > 0n && sequence > this.lastSequence + 1n)
      this.gaps += Number(sequence - this.lastSequence - 1n);
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
      const id = `${session}:${integer(record.spanId)}`,
        root = `${session}:${integer(record.traceRoot)}`,
        parent = `${session}:${integer(record.parentId)}`;
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
            fields[key] = decoder.decode(
              batch.strings.subarray(f.offset, f.offset + f.length),
            );
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
      } else {
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
        Object.assign(span.fields, fields);
        if (record.kind === 4) span.end = time;
        if (record.kind === 3)
          span.links.push(`${session}:${integer(record.linkedId)}`);
      }
      this.received++;
    }
    this.lastArrival = performance.now();
    for (const [id, span] of this.spans) {
      if ((span.end ?? span.start) < this.latest - 65) this.spans.delete(id);
    }
    while (this.spans.size > 2000)
      this.spans.delete(this.spans.keys().next().value!);
    this.events = this.events
      .filter((e) => e.time >= this.latest - 65)
      .slice(-600);
    return true;
  }
  now(): number {
    return (
      this.latest +
      (this.lastArrival
        ? Math.min(10000, performance.now() - this.lastArrival) / 1000
        : 0)
    );
  }
}
export const phaseColor = (name: string) =>
  ({
    sleep: "#74a9e8",
    work: "#b59bfa",
    publish: "#f8c76a",
    receive: "#64d9b2",
    greeting: "#52677d",
  })[name] ?? "#8da4b9";
export function depth(span: TraceSpan, spans: Map<string, TraceSpan>): number {
  let n = 0,
    parent = spans.get(span.parent);
  const seen = new Set([span.id]);
  while (
    parent &&
    parent.worker === span.worker &&
    n < 3 &&
    !seen.has(parent.id)
  ) {
    seen.add(parent.id);
    n++;
    parent = spans.get(parent.parent);
  }
  return n;
}
