import { workflow } from './native.js';
import type { doramcap } from './generated/console.js';
type Event = doramcap.workflow.v1.IEvent;
export const MAX_PROGRESS_ROWS = 512;
const STARTED = workflow.Event.State.STARTED;
const summary = (a: Event, b: Event) =>
  a.parent == null &&
  b.parent == null &&
  a.session === b.session &&
  a.executor === b.executor &&
  a.step === b.step;

/** Reduce raw execution events into bounded, redacted Console rows. */
export class Progress {
  rows: Event[] = [];
  sequence = 0n;
  gaps = 0n;
  truncated = 0;
  overflow = 0;
  unknown = false;
  apply(raw: Event) {
    const event = workflow.Event.create({
      ...raw,
      payloadJson: null,
      error: raw.error ? 'step failed' : null,
    });
    const sequence = BigInt(event.sequence.toString());
    if (sequence > this.sequence) {
      this.gaps += sequence - this.sequence - 1n;
      this.sequence = sequence;
    }
    if (event.version !== 1 || !event.state || !Object.values(workflow.Event.State).includes(event.state)) {
      this.unknown = true;
      return;
    }
    const index = this.rows.findIndex(
      (row) =>
        row.session === event.session &&
        row.executor === event.executor &&
        String(row.invocation) === String(event.invocation),
    );
    if (index >= 0) {
      this.rows[index] = event;
      return;
    }
    if (this.rows.length === MAX_PROGRESS_ROWS) {
      let victim = this.rows.findIndex(
        (row, i) =>
          row.state !== STARTED &&
          (row.parent != null ||
            summary(row, event) ||
            this.rows.slice(i + 1).some((later) => summary(row, later))),
      );
      if (victim < 0) victim = this.rows.findIndex((row) => row.state !== STARTED);
      if (victim < 0) {
        this.overflow++;
        return;
      }
      this.rows.splice(victim, 1);
      this.truncated++;
    }
    this.rows.push(event);
  }
  get incomplete() {
    return this.unknown || this.gaps > 0n || this.overflow > 0;
  }
}
