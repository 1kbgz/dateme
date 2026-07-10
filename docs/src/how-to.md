# How-to Guides

Practical recipes for common scheduling tasks. The examples use Python unless a
JavaScript call is the point of the recipe. The same schedule JSON works in all
bindings.

For field-level details, see the [Schedule model](schedule-model.md). For API
signatures, see the [Python API](api-python.md) and
[JavaScript API](api-javascript.md).

## How to Skip Market Holidays and Move to the Next Open Day

Use an `exclude` overlay and `makeup: "after"`.

```python
from datetime import datetime, timezone
from dateme import Schedule

schedule = Schedule({
    "freq": {"type": "weekly", "days": ["mon"], "time": "17:30"},
    "timezone": "America/New_York",
    "overlays": [{"calendar": "nyse_holiday", "rule": "exclude"}],
    "makeup": "after",
})

after = datetime(2026, 1, 13, tzinfo=timezone.utc)
schedule.next(after)
# datetime.datetime(2026, 1, 20, 22, 30, tzinfo=datetime.timezone.utc)
```

Use `"makeup": "before"` to move backward. Use `"makeup": "none"` to drop the
cycle.

## How to Run Only on Trading Days

Use an `only` overlay against `nyse_trading_day`.

```python
schedule = Schedule({
    "freq": {"type": "daily", "time": "16:00"},
    "timezone": "America/New_York",
    "overlays": [{"calendar": "nyse_trading_day", "rule": "only"}],
    "makeup": "none",
})
```

## How to Run on the Last Business Day of Each Month

Use the last day of the month, require US business days, and roll back when the
last calendar day is not a business day.

```python
schedule = Schedule({
    "freq": {
        "type": "monthly_by_day",
        "days": [{"type": "last"}],
        "time": "16:00",
    },
    "timezone": "America/New_York",
    "overlays": [{"calendar": "us_business_day", "rule": "only"}],
    "makeup": "before",
})
```

## How to Schedule Multiple Days in One Rule

Put every selected slot in the frequency object.

```python
schedule = Schedule({
    "freq": {
        "type": "weekly",
        "days": ["mon", "wed", "fri"],
        "time": "09:00",
    },
    "timezone": "UTC",
})
```

For monthly slots:

```python
schedule = Schedule({
    "freq": {
        "type": "monthly_by_weekday",
        "weekdays": [
            {"nth": "first", "weekday": "tue"},
            {"nth": "third", "weekday": "tue"},
        ],
        "time": "09:00",
    },
    "timezone": "UTC",
})
```

## How to Create Biweekly or Every-N-Day Schedules

Use `every_n_weeks` when the schedule repeats by week.

```python
schedule = Schedule({
    "freq": {
        "type": "every_n_weeks",
        "interval": 2,
        "start_date": "2026-01-05",
        "days": ["mon", "thu"],
        "time": "17:00",
    },
    "timezone": "UTC",
})
```

Use `every_n_days` when the schedule repeats by elapsed calendar days.

```python
schedule = Schedule({
    "freq": {
        "type": "every_n_days",
        "interval": 3,
        "start_date": "2026-01-01",
        "time": "09:00",
    },
    "timezone": "UTC",
})
```

## How to Schedule Quarterly Events

Use `quarterly`. `month` is the month within each quarter: `1`, `2`, or `3`.

```python
schedule = Schedule({
    "freq": {
        "type": "quarterly",
        "month": 1,
        "day": {"type": "day", "value": 15},
        "time": "12:00",
    },
    "timezone": "UTC",
})
```

## How to Use a Cron Expression

Use `custom_cron` for a five-field cron expression in schedule-local time.

```python
schedule = Schedule({
    "freq": {"type": "custom_cron", "expr": "30 9 * * 1-5"},
    "timezone": "America/New_York",
})
```

## How to Bound a Series

Set `start` and/or `end` as UTC instants.

```python
schedule = Schedule({
    "freq": {"type": "daily", "time": "12:00"},
    "timezone": "UTC",
    "start": "2026-06-01T00:00:00Z",
    "end": "2026-07-01T00:00:00Z",
})
```

`end` is exclusive.

## How to Render Upcoming Occurrences

Use `upcoming` for a count.

```python
rows = schedule.upcoming(10)
```

Use `until` for a window.

```python
from datetime import datetime, timezone

end = datetime(2026, 12, 31, tzinfo=timezone.utc)
rows = schedule.until(end)
```

Use `since` for a reverse-ordered history.

```python
start = datetime(2026, 1, 1, tzinfo=timezone.utc)
recent = schedule.since(start)
```

## How to Annotate Made-Up Occurrences

Use trace methods when a UI needs to show why an occurrence exists.

```python
trace = schedule.next_trace(after)
trace["instant"]
trace["reason"]
```

For a list:

```python
rows = schedule.upcoming_trace(5, after)
```

## How to Check Membership and Count a Window

Use `is_occurrence` for membership and `count_between` for counts.

```python
instant = datetime(2026, 1, 20, 22, 30, tzinfo=timezone.utc)
schedule.is_occurrence(instant)
schedule.count_between(after, end)
```

Python also supports membership syntax.

```python
instant in schedule
```

## How to Iterate a Bounded Schedule

Set `end`, then iterate the schedule.

```python
schedule = Schedule({
    "freq": {"type": "daily", "time": "12:00"},
    "timezone": "UTC",
    "start": "2026-01-01T00:00:00Z",
    "end": "2026-01-04T00:00:00Z",
})

for instant in schedule:
    print(instant)
```

Use explicit helpers for caller-provided bounds.

```python
list(schedule.iter_between(after, end))
list(schedule.iter_upcoming(3, after))
```

In JavaScript:

```js
for (const instant of schedule) {
  console.log(instant.toISOString());
}

Array.from(schedule.iterBetween(after, end));
Array.from(schedule.iterUpcoming(3, after));
```

## How to Restrict Makeup Destinations

Use destination constraints with a makeup direction.

```python
schedule = Schedule({
    "freq": {"type": "weekly", "days": ["mon"], "time": "09:00"},
    "timezone": "UTC",
    "overlays": [{"calendar": "us_federal_holiday", "rule": "exclude"}],
    "makeup": "after",
    "makeup_only_on": ["tue", "wed", "thu"],
    "makeup_within_week": True,
    "makeup_exclude_weekends": True,
    "makeup_before_next": True,
})
```

## How to Use Different Makeup Rules by Weekday

Use a weekday map for `makeup`.

```python
schedule = Schedule({
    "freq": {"type": "weekly", "days": ["mon", "fri"], "time": "09:00"},
    "timezone": "UTC",
    "overlays": [{"calendar": "us_federal_holiday", "rule": "exclude"}],
    "makeup": {
        "mon": "after",
        "fri": "before",
        "default": "none",
    },
})
```

## How to Try Fallback Makeup Strategies

Use a makeup cascade.

```python
schedule = Schedule({
    "freq": {"type": "daily", "time": "09:00"},
    "timezone": "UTC",
    "overlays": [{"calendar": "us_federal_holiday", "rule": "exclude"}],
    "makeup": [
        {"direction": "after", "max_hops": 3},
        {"direction": "before", "max_hops": 3},
        "none",
    ],
})
```

## How to Fail When Makeup Cannot Find a Date

Use `makeup_failure: "error"` for strict schedules.

```python
schedule = Schedule({
    "freq": {"type": "daily", "time": "09:00"},
    "timezone": "UTC",
    "overlays": [{"calendar": {"dates": ["2026-01-02"]}, "rule": "exclude"}],
    "makeup": "after",
    "max_makeup_hops": 0,
    "makeup_failure": "error",
})
```

Queries raise/throw when the failure is encountered.

## How to Skip Runs of Consecutive Exclusions

Use `skip_if_consecutive_excluded`.

```python
schedule = Schedule({
    "freq": {"type": "daily", "time": "09:00"},
    "timezone": "UTC",
    "overlays": [{"calendar": {"dates": ["2026-01-05", "2026-01-06"]}, "rule": "exclude"}],
    "makeup": "after",
    "skip_if_consecutive_excluded": 2,
})
```

## How to Alert on Long Gaps

Use `max_skip_gap`.

```python
schedule = Schedule({
    "freq": {"type": "weekly", "days": ["mon"], "time": "09:00"},
    "timezone": "UTC",
    "max_skip_gap": 10,
})
```

Queries raise/throw when the returned stream has a gap longer than the limit.

## How to Combine Calendar Sets

Use `dates` for inline sets, `union` for any child set, and `diff` for the first
set minus later sets.

```python
schedule = Schedule({
    "freq": {"type": "daily", "time": "09:00"},
    "timezone": "UTC",
    "overlays": [
        {
            "calendar": {
                "union": [
                    "us_federal_holiday",
                    {"dates": ["2026-12-24"]},
                    {"custom": "company_shutdown"},
                ]
            },
            "rule": "exclude",
        }
    ],
})
```

## How to Provide Custom Calendars

Use `{ "custom": "name" }` in the schedule and pass a provider to the
constructor.

```python
def contains(name: str, date: str) -> bool:
    return name == "company_shutdown" and date in {"2026-12-24", "2026-12-31"}


schedule = Schedule(
    {
        "freq": {"type": "daily", "time": "09:00"},
        "timezone": "UTC",
        "overlays": [{"calendar": {"custom": "company_shutdown"}, "rule": "exclude"}],
    },
    contains,
)
```

JavaScript providers receive the same `(name, date)` arguments.

```js
const schedule = new Schedule(spec, (name, date) => {
  return name === "company_shutdown" && date === "2026-12-24";
});
```

## How to Build Schedules with Typed Builders

Use `dateme.model` builders in Python.

```python
from dateme import Schedule, Weekly, Weekday, Overlay, CalendarId, OverlayRule, Makeup
from dateme import model as m

spec = m.Schedule(
    freq=Weekly([Weekday.MON], "17:30"),
    timezone="America/New_York",
    overlays=[Overlay(CalendarId.NYSE_HOLIDAY, OverlayRule.EXCLUDE)],
    makeup=Makeup.AFTER,
)

schedule = Schedule(spec)
```

Use `ScheduleSpec` and runtime enum objects in TypeScript.

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

## How to Store and Reload a Schedule

Store the JSON form.

```python
blob = schedule.to_json()
again = Schedule.from_json(blob)
```

In JavaScript:

```js
const blob = JSON.stringify(schedule);
const again = new Schedule(blob);
```
