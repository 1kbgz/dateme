# JavaScript API

Reference for the JavaScript and TypeScript package.

The JavaScript package wraps the same Rust engine through WebAssembly. Public
methods take and return `Date` objects. Schedule specs use the same JSON shape
as every other binding.

## Exports

| Name                    | Description                               |
| ----------------------- | ----------------------------------------- |
| `init`                  | WebAssembly initialization function.      |
| `Schedule`              | Query engine class.                       |
| `wasm`                  | Raw generated WebAssembly module exports. |
| `Weekday`               | Runtime weekday enum object.              |
| `Nth`                   | Runtime nth-weekday enum object.          |
| `Makeup`                | Runtime makeup enum object.               |
| `MakeupFailure`         | Runtime makeup-failure enum object.       |
| `OverlayRule`           | Runtime overlay-rule enum object.         |
| `CalendarId`            | Runtime built-in calendar enum object.    |
| `ScheduleSpec`          | TypeScript schedule model type.           |
| `Frequency`             | TypeScript frequency union type.          |
| `CalendarSpec`          | TypeScript calendar spec union type.      |
| `Overlay`, `AnyOverlay` | TypeScript overlay types.                 |
| `OccurrenceTrace`       | `{ instant: Date, reason: string }`.      |

## Initialization

```js
import init, { Schedule } from "dateme";

await init();
```

`init()` must resolve before constructing `Schedule`.

## `Schedule`

### Constructor

```ts
new Schedule(spec: ScheduleSpec | string, calendarProvider?: CalendarProvider)
```

Parameters:

| Parameter          | Type                          | Description                                  |
| ------------------ | ----------------------------- | -------------------------------------------- |
| `spec`             | `ScheduleSpec` or JSON string | Schedule model.                              |
| `calendarProvider` | function or object, optional  | Provider for `{ custom: "name" }` calendars. |

The constructor validates the schedule. Malformed JSON, invalid enum values,
invalid timezone names, and structural validation failures throw.

```js
const schedule = new Schedule({
  freq: { type: "daily", time: "09:00" },
  timezone: "UTC",
});
```

### Custom Calendar Providers

Custom calendars are referenced in a schedule with `{ custom: "name" }`.
Providers receive `(name, date)` where `date` is a `YYYY-MM-DD` string.

A function provider:

```js
const schedule = new Schedule(
  {
    freq: { type: "daily", time: "09:00" },
    timezone: "UTC",
    overlays: [{ calendar: { custom: "shutdown" }, rule: "exclude" }],
  },
  (name, date) => name === "shutdown" && date === "2026-08-14",
);
```

An object provider:

```js
const provider = {
  contains(name, date) {
    return name === "shutdown" && date === "2026-08-14";
  },
};

const schedule = new Schedule(spec, provider);
```

Missing custom calendar values are treated as absent from the set.

## Date Handling

| Rule             | Behavior                                                         |
| ---------------- | ---------------------------------------------------------------- |
| Input dates      | JavaScript `Date` objects.                                       |
| Returned values  | JavaScript `Date` objects.                                       |
| Optional anchors | Default to `new Date()`.                                         |
| Query bounds     | Strict: occurrences exactly at `after` or `before` are excluded. |
| WASM bridge      | The wrapper converts `Date` values to epoch milliseconds.        |

## Methods

### `validate`

```ts
schedule.validate(): void
```

Re-runs structural validation. Throws on failure.

```js
schedule.validate();
```

### `toObject`

```ts
schedule.toObject(): ScheduleSpec
```

Returns the schedule as a plain object.

```js
const spec = schedule.toObject();
```

### `toJSON`

```ts
schedule.toJSON(): ScheduleSpec
```

Returns the schedule as a plain object for `JSON.stringify`.

```js
JSON.stringify(schedule);
```

### `next`

```ts
schedule.next(after = new Date()): Date | null
```

Returns the first occurrence strictly after `after`.

```js
const after = new Date("2026-01-13T00:00:00Z");
schedule.next(after);
```

### `previous`

```ts
schedule.previous(before = new Date()): Date | null
```

Returns the last occurrence strictly before `before`.

```js
schedule.previous(new Date("2026-01-13T00:00:00Z"));
```

### `until`

```ts
schedule.until(before: Date, after = new Date()): Date[]
```

Returns occurrences in `(after, before)`, ascending.

```js
schedule.until(new Date("2026-02-01T00:00:00Z"), after);
```

### `since`

```ts
schedule.since(after: Date, before = new Date()): Date[]
```

Returns occurrences in `(after, before)`, descending.

```js
schedule.since(new Date("2026-01-01T00:00:00Z"));
```

### `upcoming`

```ts
schedule.upcoming(n: number, after = new Date()): Date[]
```

Returns the next `n` occurrences strictly after `after`, ascending.

```js
schedule.upcoming(5, after);
```

### Trace Methods

Trace methods return `OccurrenceTrace` values.

| Method                                   | Return type               | Order      |
| ---------------------------------------- | ------------------------- | ---------- |
| `nextTrace(after = new Date())`          | `OccurrenceTrace \| null` | —          |
| `previousTrace(before = new Date())`     | `OccurrenceTrace \| null` | —          |
| `untilTrace(before, after = new Date())` | `OccurrenceTrace[]`       | ascending  |
| `sinceTrace(after, before = new Date())` | `OccurrenceTrace[]`       | descending |
| `upcomingTrace(n, after = new Date())`   | `OccurrenceTrace[]`       | ascending  |

Reason strings include:

| Reason form                    | Meaning                                         |
| ------------------------------ | ----------------------------------------------- |
| `base`                         | Base occurrence was kept.                       |
| `makeup_from(YYYY-MM-DD)`      | Occurrence was moved from the local date.       |
| `base,shifted_dst`             | Base occurrence was shifted through DST gap.    |
| `makeup_from(...),shifted_dst` | Made-up occurrence was shifted through DST gap. |

```js
const trace = schedule.nextTrace(after);
// { instant: Date, reason: "base" }
```

### `isOccurrence`

```ts
schedule.isOccurrence(instant: Date): boolean
```

Returns whether `instant` is an occurrence of the schedule.

```js
schedule.isOccurrence(new Date("2026-01-20T22:30:00Z"));
```

### `countBetween`

```ts
schedule.countBetween(after: Date, before: Date): number
```

Returns the number of occurrences strictly in `(after, before)`.

```js
schedule.countBetween(after, new Date("2026-02-01T00:00:00Z"));
```

### `describe`

```ts
schedule.describe(): string
```

Returns a human-readable summary of the base recurrence, timezone, overlay
count, and makeup presence.

```js
schedule.describe();
// "Every Monday at 17:30 America/New_York, with 1 overlay(s), with makeup"
```

### Iteration

```ts
schedule[Symbol.iterator](): IterableIterator<Date>
schedule.iterBetween(after: Date, before: Date): IterableIterator<Date>
schedule.iterUpcoming(n: number, after = new Date()): IterableIterator<Date>
```

`for...of` requires the schedule to have an `end` bound. It starts from `start`,
or from the current time when `start` is absent.

```js
for (const instant of boundedSchedule) {
  console.log(instant.toISOString());
}

Array.from(schedule.iterBetween(after, new Date("2026-02-01T00:00:00Z")));
Array.from(schedule.iterUpcoming(3, after));
```

## TypeScript Model

The package exports TypeScript types that mirror the
[Schedule model](schedule-model.md).

```ts
import init, {
  Schedule,
  Weekday,
  CalendarId,
  OverlayRule,
  Makeup,
} from "dateme";
import type { ScheduleSpec } from "dateme";

const spec: ScheduleSpec = {
  freq: { type: "weekly", days: [Weekday.Mon], time: "17:30" },
  timezone: "America/New_York",
  overlays: [{ calendar: CalendarId.NyseHoliday, rule: OverlayRule.Exclude }],
  makeup: Makeup.After,
};

await init();
const schedule = new Schedule(spec);
```

Type families:

| Family         | Types or values                                           |
| -------------- | --------------------------------------------------------- |
| Frequencies    | `Frequency` union with all schedule frequency variants.   |
| Calendar specs | `CalendarSpec`, `CalendarId`.                             |
| Overlays       | `Overlay`, `AnyOverlay`, `OverlayRule`.                   |
| Makeup         | `Makeup`, `MakeupFailure`, `MakeupStep`, `WeekdayMakeup`. |
| Date helpers   | `MonthDay`, `NthWeekday`, `Nth`, `Weekday`.               |
