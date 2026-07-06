import init, { Schedule as WasmSchedule } from "../../dist/pkg/dateme";
import type { ScheduleSpec } from "./model";

export { default as init } from "../../dist/pkg/dateme";
export * as wasm from "../../dist/pkg/dateme";
export * from "./model";

/**
 * A recurrence schedule. Construct from a typed spec object or its JSON string,
 * then query occurrence instants. Reference instants default to `new Date()`.
 *
 * The WASM module must be initialized first: `await init()` (or `initSync`).
 */
export class Schedule {
  private inner: WasmSchedule;

  constructor(spec: ScheduleSpec | string) {
    this.inner = new WasmSchedule(
      typeof spec === "string" ? spec : JSON.stringify(spec),
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
}

export default Schedule;
