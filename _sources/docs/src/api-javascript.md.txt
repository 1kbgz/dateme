# JavaScript API

The JavaScript package is a WebAssembly build of the same engine. It exposes an
`init` function and a `Schedule` class. Build a schedule from the JSON
[Schedule model](schedule-model.md) (as a string or a plain object); every method
takes and returns `Date` objects.

## Initialization

The WebAssembly module must be initialized once before use:

```js
import init, { Schedule } from "dateme";

await init();
```

`init` loads and instantiates the `.wasm` binary. Call it once at startup and
await it before constructing any `Schedule`.

## Constructing a schedule

The constructor accepts either a native spec object or a JSON string; an object
is serialized for you. It validates the schedule and throws on malformed input,
an unknown timezone/enum value, or a structurally invalid schedule — so a
`Schedule` you hold is always well-formed. `validate()` re-runs the check on
demand.

```js
const schedule = new Schedule({
  freq: { type: "weekly", days: ["mon"], time: "17:30" },
  timezone: "America/New_York",
  overlays: [{ calendar: "nyse_holiday", rule: "exclude" }],
  makeup: "after",
});
```

## Typed model

The package exports TypeScript types mirroring the [Schedule model](schedule-model.md)
— `ScheduleSpec`, `Frequency`, `MonthDay`, `NthWeekday`, `Overlay` — so the spec
object is checked at compile time. It also exports runtime enum objects
(`Weekday`, `Nth`, `Makeup`, `OverlayRule`, `CalendarId`) for plain JavaScript:

```ts
import init, { Schedule, Weekday, CalendarId, OverlayRule, Makeup } from "dateme";
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

## Methods

| Method                              | Returns                  | Order      |
| ----------------------------------- | ------------------------ | ---------- |
| `next(after = new Date())`          | `Date` or `null`         | —          |
| `previous(before = new Date())`     | `Date` or `null`         | —          |
| `until(before, after = new Date())` | `Date[]`                 | ascending  |
| `since(after, before = new Date())` | `Date[]`                 | descending |
| `upcoming(n, after = new Date())`   | `Date[]`                 | ascending  |
| `validate()`                        | `void` (throws on error) | —          |
| `toObject()`                        | `ScheduleSpec`           | —          |
| `toJSON()`                          | `ScheduleSpec`           | —          |

- Every optional reference instant defaults to `new Date()`.
- `until(end)[0]` equals `next()`; `since(start)[0]` equals `previous()`.
- Results are strictly between the two bounds and deduplicated by instant.

## Example

```js
await init();

const schedule = new Schedule({
  freq: { type: "weekly", days: ["mon"], time: "17:30" },
  timezone: "America/New_York",
  overlays: [{ calendar: "nyse_holiday", rule: "exclude" }],
  makeup: "after",
});

schedule.validate();

const after = new Date("2026-01-13T00:00:00Z");
schedule.next(after);          // 2026-01-20T22:30:00.000Z (MLK Day made up to Tuesday)
schedule.upcoming(3, after);   // three Dates, ascending
schedule.previous(after);      // 2026-01-12T22:30:00.000Z
```

## Notes

- Internally the WebAssembly layer exchanges epoch milliseconds; the `Schedule`
  wrapper converts to and from `Date` for you.
- `null` (JavaScript) corresponds to `None` (Python) for `next` / `previous` when
  no occurrence exists in range.
- The same built-in calendars ship in the WebAssembly build, so market-holiday
  overlays work in the browser without any additional data.
