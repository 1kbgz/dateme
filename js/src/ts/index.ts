import init, { Schedule as WasmSchedule } from "../../dist/pkg/dateme";
import type { ScheduleSpec } from "./model";

export { default as init } from "../../dist/pkg/dateme";
export * as wasm from "../../dist/pkg/dateme";
export * from "./model";

export type CalendarProvider =
  | ((name: string, date: string) => boolean)
  | { contains(name: string, date: string): boolean };

export interface OccurrenceTrace {
  instant: Date;
  reason: string;
}

interface RawOccurrenceTrace {
  instant: string;
  reason: string;
}

function traceFromJSON(json: string): OccurrenceTrace {
  const raw = JSON.parse(json) as RawOccurrenceTrace;
  return { instant: new Date(raw.instant), reason: raw.reason };
}

function tracesFromJSON(json: string): OccurrenceTrace[] {
  return (JSON.parse(json) as RawOccurrenceTrace[]).map((raw) => ({
    instant: new Date(raw.instant),
    reason: raw.reason,
  }));
}

/**
 * A recurrence schedule. Construct from a typed spec object or its JSON string,
 * then query occurrence instants. Reference instants default to `new Date()`.
 *
 * The WASM module must be initialized first: `await init()` (or `initSync`).
 */
export class Schedule {
  private inner: WasmSchedule;

  constructor(
    spec: ScheduleSpec | string,
    calendarProvider?: CalendarProvider,
  ) {
    this.inner = new WasmSchedule(
      typeof spec === "string" ? spec : JSON.stringify(spec),
      calendarProvider,
    );
  }

  /** Structural validation; throws on an invalid schedule. */
  validate(): void {
    this.inner.validate();
  }

  /** The schedule as a plain spec object (round-trips the JSON form). */
  toObject(): ScheduleSpec {
    return JSON.parse(this.inner.toJSON());
  }

  /** The schedule as a plain object; enables `JSON.stringify(schedule)`. */
  toJSON(): ScheduleSpec {
    return this.toObject();
  }

  /** First occurrence strictly after `after`; `null` if none. */
  next(after: Date = new Date()): Date | null {
    const ms = this.inner.next(after.getTime());
    return ms == null ? null : new Date(ms);
  }

  /** Last occurrence strictly before `before`; `null` if none. */
  previous(before: Date = new Date()): Date | null {
    const ms = this.inner.previous(before.getTime());
    return ms == null ? null : new Date(ms);
  }

  /** Occurrences in `(after, before)`, ascending. `until(end)[0]` == `next()`. */
  until(before: Date, after: Date = new Date()): Date[] {
    return Array.from(
      this.inner.until(before.getTime(), after.getTime()),
      (ms) => new Date(ms),
    );
  }

  /** Occurrences in `(after, before)`, descending. `since(start)[0]` == `previous()`. */
  since(after: Date, before: Date = new Date()): Date[] {
    return Array.from(
      this.inner.since(after.getTime(), before.getTime()),
      (ms) => new Date(ms),
    );
  }

  /** The next `n` occurrences strictly after `after`, ascending. */
  upcoming(n: number, after: Date = new Date()): Date[] {
    return Array.from(
      this.inner.upcoming(n, after.getTime()),
      (ms) => new Date(ms),
    );
  }

  /** First occurrence trace strictly after `after`; `null` if none. */
  nextTrace(after: Date = new Date()): OccurrenceTrace | null {
    const json = this.inner.nextTraceJSON(after.getTime());
    return json == null ? null : traceFromJSON(json);
  }

  /** Last occurrence trace strictly before `before`; `null` if none. */
  previousTrace(before: Date = new Date()): OccurrenceTrace | null {
    const json = this.inner.previousTraceJSON(before.getTime());
    return json == null ? null : traceFromJSON(json);
  }

  /** Occurrence traces in `(after, before)`, ascending. */
  untilTrace(before: Date, after: Date = new Date()): OccurrenceTrace[] {
    return tracesFromJSON(
      this.inner.untilTraceJSON(before.getTime(), after.getTime()),
    );
  }

  /** Occurrence traces in `(after, before)`, descending. */
  sinceTrace(after: Date, before: Date = new Date()): OccurrenceTrace[] {
    return tracesFromJSON(
      this.inner.sinceTraceJSON(after.getTime(), before.getTime()),
    );
  }

  /** The next `n` occurrence traces strictly after `after`, ascending. */
  upcomingTrace(n: number, after: Date = new Date()): OccurrenceTrace[] {
    return tracesFromJSON(this.inner.upcomingTraceJSON(n, after.getTime()));
  }

  /** Whether `instant` is an occurrence of this schedule. */
  isOccurrence(instant: Date): boolean {
    return this.inner.isOccurrence(instant.getTime());
  }

  /** Count occurrences strictly in `(after, before)`. */
  countBetween(after: Date, before: Date): number {
    return this.inner.countBetween(after.getTime(), before.getTime());
  }

  /** Human-readable schedule summary. */
  describe(): string {
    return this.inner.describe();
  }
}

export default Schedule;
