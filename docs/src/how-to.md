# How-to guides

Practical recipes for common scheduling goals. Each assumes you already know the
basics ([Getting started](tutorial.md)) and can build a `Schedule` from JSON.
For the meaning of every field, see the [Schedule model](schedule-model.md).

The examples use Python. The same JSON and method names work in
[JavaScript](api-javascript.md); only the datetime type differs (`Date` instead
of `datetime`).

## Skip market holidays, moving the cycle to the next open day

Add an `exclude` overlay against the market's holiday calendar and set `makeup`
to `after` so a dropped cycle moves forward to the next surviving day:

```json
{
  "freq": { "type": "weekly", "days": ["mon"], "time": "17:30" },
  "timezone": "America/New_York",
  "overlays": [ { "calendar": "nyse_holiday", "rule": "exclude" } ],
  "makeup": "after"
}
```

Use `"makeup": "before"` to move to the previous open day instead, or
`"makeup": "none"` to simply skip the cycle.

## Run only on trading days

To fire *only* when the market is open, use an `only` overlay against the trading
calendar. This drops weekends and holidays in one rule:

```json
{
  "freq": { "type": "daily", "time": "16:00" },
  "timezone": "America/New_York",
  "overlays": [ { "calendar": "nyse_trading_day", "rule": "only" } ],
  "makeup": "none"
}
```

`only` keeps a date when the calendar *contains* it; `exclude` drops a date when
the calendar contains it. See [Calendars and overlays](#overlays).

## Fire on the last business day of the month

Combine the `last` day-of-month with an `only` business-day overlay and
`makeup: before`, so a month that ends on a weekend or holiday rolls back to the
last business day:

```json
{
  "freq": { "type": "monthly_by_day", "days": [ { "type": "last" } ], "time": "16:00" },
  "timezone": "America/New_York",
  "overlays": [ { "calendar": "us_business_day", "rule": "only" } ],
  "makeup": "before"
}
```

For example May 31 2026 is a Sunday, so the May cycle fires on Friday May 29.

## Fire on the first and third Tuesday of the month

List several slots; the engine emits each, in ascending order:

```json
{
  "freq": {
    "type": "monthly_by_weekday",
    "weekdays": [
      { "nth": "first", "weekday": "tue" },
      { "nth": "third", "weekday": "tue" }
    ],
    "time": "09:00"
  },
  "timezone": "America/New_York"
}
```

The same applies to weekly `days` (`["mon", "wed", "fri"]`) and monthly
`monthly_by_day` (`[{"type":"day","value":1}, {"type":"day","value":15}]`).

## Start a series in the future

Set `start` to the earliest allowed instant. No occurrence is returned before it —
useful for a competition created now that should not open until later:

```python
from datetime import datetime, timezone
from dateme import Schedule

schedule = Schedule.from_json("""
{
  "freq": { "type": "daily", "time": "12:00" },
  "timezone": "UTC",
  "start": "2026-06-01T00:00:00Z"
}
""")

schedule.next(datetime(2026, 1, 1, tzinfo=timezone.utc))
# -> 2026-06-01 12:00:00+00:00
```

## End a series on a date

Set `end` to the exclusive upper bound. `next` returns `None` once the following
occurrence would fall at or after it:

```python
schedule = Schedule.from_json("""
{
  "freq": { "type": "daily", "time": "12:00" },
  "timezone": "UTC",
  "end": "2026-01-03T00:00:00Z"
}
""")

schedule.next(datetime(2026, 1, 2, 13, tzinfo=timezone.utc)) is None
# -> True   (the next 12:00 would be 2026-01-03, which is >= end)
```

## Project the next N cycles for a UI

To render an "upcoming instances" table, ask for a fixed count with `upcoming`.
These are computed, not stored — future cycles need not exist yet:

```python
rows = schedule.upcoming(10)          # next 10 after now
```

If you have a concrete window instead of a count, use `until`:

```python
from datetime import datetime, timezone
year_end = datetime(2026, 12, 31, tzinfo=timezone.utc)
rows = schedule.until(year_end)       # every occurrence from now to year end
```

## Show recent cycles, most-recent first

`since` is the backward series and returns **descending** order, so the first
element is the most recent occurrence:

```python
from datetime import datetime, timezone
year_start = datetime(2026, 1, 1, tzinfo=timezone.utc)
recent = schedule.since(year_start)   # now back to Jan 1, newest first
recent[0]                             # == schedule.previous()
```

## Handle daylight-saving time safely

Just give a local `time` and an IANA `timezone`; the engine tracks DST for you. A
daily 09:00 schedule fires at 09:00 *local* every day, so the UTC instant shifts
by an hour across the spring and autumn transitions — you do not manage that:

```json
{ "freq": { "type": "daily", "time": "09:00" }, "timezone": "America/New_York" }
```

Two edge cases are resolved automatically (see
[Timezones and DST](#timezones-and-dst)): a local time that falls
in a spring-forward gap moves to the first valid instant after the gap; a local
time that occurs twice at an autumn fall-back uses the earlier instant.

## Build a schedule from typed objects instead of JSON

Use the `dateme.model` builders for a typed, autocomplete-friendly spec that is
validated as you build it. Pass the result straight to `Schedule`:

```python
from dateme import Schedule, model as m
from dateme import Weekly, Overlay, Makeup, CalendarId, OverlayRule, Weekday

spec = m.Schedule(
    freq=Weekly([Weekday.MON], "17:30"),
    timezone="America/New_York",
    overlays=[Overlay(CalendarId.NYSE_HOLIDAY, OverlayRule.EXCLUDE)],
    makeup=Makeup.AFTER,
)
schedule = Schedule(spec)             # or Schedule(spec.to_dict())
```

In JavaScript/TypeScript the spec object is typed by `ScheduleSpec`, with runtime
enums for the string values:

```ts
import init, { Schedule, Weekday, CalendarId, OverlayRule, Makeup } from "dateme";

await init();
const schedule = new Schedule({
  freq: { type: "weekly", days: [Weekday.Mon], time: "17:30" },
  timezone: "America/New_York",
  overlays: [{ calendar: CalendarId.NyseHoliday, rule: OverlayRule.Exclude }],
  makeup: Makeup.After,
});
```

## Store and reload a schedule

`to_json` round-trips the schedule, so you can persist it (for example in a JSONB
column) and rebuild it later:

```python
blob = schedule.to_json()             # store this string
again = Schedule.from_json(blob)      # rebuild identically
```

The same schedule is also available as a dict via `to_dict()` /
`Schedule.from_dict(...)` (Python) or `toObject()` / `new Schedule(obj)`
(JavaScript).
