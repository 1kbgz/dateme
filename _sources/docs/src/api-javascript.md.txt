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

```js
const schedule = new Schedule({
  freq: { type: "weekly", days: ["mon"], time: "17:30" },
  timezone: "America/New_York",
  overlays: [{ calendar: "nyse_holiday", rule: "exclude" }],
  makeup: "after",
});
```

The constructor accepts either a JSON string or an object; an object is
`JSON.stringify`ed for you. It throws on malformed input or an unknown
timezone/enum value.

## Methods

| Method                              | Returns                  | Order      |
| ----------------------------------- | ------------------------ | ---------- |
| `next(after = new Date())`          | `Date` or `null`         | —          |
| `previous(before = new Date())`     | `Date` or `null`         | —          |
| `until(before, after = new Date())` | `Date[]`                 | ascending  |
| `since(after, before = new Date())` | `Date[]`                 | descending |
| `upcoming(n, after = new Date())`   | `Date[]`                 | ascending  |
| `validate()`                        | `void` (throws on error) | —          |
| `toJSON()`                          | `unknown` (plain object) | —          |

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
