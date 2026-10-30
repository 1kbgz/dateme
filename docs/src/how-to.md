# How-to Guides

Recipes for scheduling tasks, using Python unless a section shows JavaScript or
TypeScript. All bindings accept the same schedule JSON.

For field-level details, see the [Schedule model](schedule-model.md). For API
signatures, see the [Python API](api-python.md) and
[JavaScript API](api-javascript.md).

## How to Skip Market Holidays and Move to the Next Open Day

Exclude `nyse_holiday` dates and use `makeup: "after"` to move excluded Mondays
to the next date that passes the overlay.

```python
from datetime import datetime, timezone
from dateme import Schedule

schedule = Schedule({
    "frequency": {"type": "weekly", "days": ["mon"], "time": "17:30"},
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

Use an `only` overlay with `nyse_trading_day` to exclude weekends and market
holidays.

```python
schedule = Schedule({
    "frequency": {"type": "daily", "time": "16:00"},
    "timezone": "America/New_York",
    "overlays": [{"calendar": "nyse_trading_day", "rule": "only"}],
    "makeup": "none",
})
```

## How to Run on the Last Business Day of Each Month

Schedule the last calendar day of each month and require `us_business_day`.
Use `makeup: "before"` to move back when that date is not a business day.

```python
schedule = Schedule({
    "frequency": {
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

List the selected weekdays in `frequency.days`.

```python
schedule = Schedule({
    "frequency": {
        "type": "weekly",
        "days": ["mon", "wed", "fri"],
        "time": "09:00",
    },
    "timezone": "UTC",
})
```

For selected weekday positions within a month, use `monthly_by_weekday`:

```python
schedule = Schedule({
    "frequency": {
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

Use `every_n_weeks` for a weekly cycle anchored to the week containing
`start_date`.

```python
schedule = Schedule({
    "frequency": {
        "type": "every_n_weeks",
        "interval": 2,
        "start_date": "2026-01-05",
        "days": ["mon", "thu"],
        "time": "17:00",
    },
    "timezone": "UTC",
})
```

Use `every_n_days` for a cycle of calendar days anchored to `start_date`.

```python
schedule = Schedule({
    "frequency": {
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
    "frequency": {
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
    "frequency": {"type": "custom_cron", "expr": "30 9 * * 1-5"},
    "timezone": "America/New_York",
})
```

## How to Bound a Series

Set `start`, `end`, or both as UTC instants.

```python
schedule = Schedule({
    "frequency": {"type": "daily", "time": "12:00"},
    "timezone": "UTC",
    "start": "2026-06-01T00:00:00Z",
    "end": "2026-07-01T00:00:00Z",
})
```

The schedule includes `start` and excludes `end`. Query bounds remain strict.

## How to Render Upcoming Occurrences

Use `upcoming` to request up to a fixed number of occurrences.

```python
rows = schedule.upcoming(10)
```

Use `until` to list occurrences before a cutoff.

```python
from datetime import datetime, timezone

end = datetime(2026, 12, 31, tzinfo=timezone.utc)
rows = schedule.until(end)
```

Use `since` to list past occurrences in descending order.

```python
start = datetime(2026, 1, 1, tzinfo=timezone.utc)
recent = schedule.since(start)
```

## How to Annotate Made-Up Occurrences

Use trace methods to distinguish base occurrences from makeup or DST shifts.

```python
trace = schedule.next_trace(after)
if trace is not None:
    print(trace["instant"], trace["reason"])
```

For a list:

```python
rows = schedule.upcoming_trace(5, after)
```

## How to Check Membership and Count a Window

Use `is_occurrence` to check a single instant and `count_between` to count
occurrences in a strict query window.

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

Set `end`, then iterate the schedule. Iteration uses `start` as an exclusive
anchor, or the current time if `start` is absent.

```python
schedule = Schedule({
    "frequency": {"type": "daily", "time": "12:00"},
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

Combine a makeup direction with constraints on its destination dates.

```python
schedule = Schedule({
    "frequency": {"type": "weekly", "days": ["mon"], "time": "09:00"},
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

Use a weekday map for `makeup` to choose a direction for each excluded date's
weekday.

```python
schedule = Schedule({
    "frequency": {"type": "weekly", "days": ["mon", "fri"], "time": "09:00"},
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

List makeup strategies in the order to try them. End with `"none"` to skip the
occurrence if the preceding strategies fail.

```python
schedule = Schedule({
    "frequency": {"type": "daily", "time": "09:00"},
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

Set `makeup_failure` to `"error"`. This example gives makeup no search days, so
an excluded occurrence triggers the failure policy.

```python
schedule = Schedule({
    "frequency": {"type": "daily", "time": "09:00"},
    "timezone": "UTC",
    "overlays": [{"calendar": {"dates": ["2026-01-02"]}, "rule": "exclude"}],
    "makeup": "after",
    "max_makeup_hops": 0,
    "makeup_failure": "error",
})
```

Python queries raise and JavaScript queries throw when they encounter the
failure.

## How to Skip Runs of Consecutive Exclusions

Set `skip_if_consecutive_excluded` to skip an entire run once that many
consecutive base occurrences are excluded, without attempting makeup.

```python
schedule = Schedule({
    "frequency": {"type": "daily", "time": "09:00"},
    "timezone": "UTC",
    "overlays": [{"calendar": {"dates": ["2026-01-05", "2026-01-06"]}, "rule": "exclude"}],
    "makeup": "after",
    "skip_if_consecutive_excluded": 2,
})
```

## How to Alert on Long Gaps

Set `max_skip_gap` to the largest gap, in days, that queries should allow.

```python
schedule = Schedule({
    "frequency": {"type": "weekly", "days": ["mon"], "time": "09:00"},
    "timezone": "UTC",
    "max_skip_gap": 10,
})
```

Queries fail when a checked gap exceeds the limit. See the
[Schedule model](schedule-model.md#threshold-skip) for which gaps each query
checks.

## How to Combine Calendar Sets

Use `dates` for an inline date set, `union` to combine sets, and `diff` to
subtract all later sets from the first.

```python
schedule = Schedule({
    "frequency": {"type": "daily", "time": "09:00"},
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
        "frequency": {"type": "daily", "time": "09:00"},
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
    frequency=Weekly([Weekday.MON], "17:30"),
    timezone="America/New_York",
    overlays=[Overlay(CalendarId.NYSE_HOLIDAY, OverlayRule.EXCLUDE)],
    makeup=Makeup.AFTER,
)

schedule = Schedule(spec)
```

Use `ScheduleSpec` with the exported enum values in TypeScript.

```ts
import { init, Schedule, Weekday, CalendarId, OverlayRule, Makeup } from "dateme";
import type { ScheduleSpec } from "dateme";

const spec: ScheduleSpec = {
    frequency: { type: "weekly", days: [Weekday.Mon], time: "17:30" },
  timezone: "America/New_York",
  overlays: [{ calendar: CalendarId.NyseHoliday, rule: OverlayRule.Exclude }],
  makeup: Makeup.After,
};

await init();
const schedule = new Schedule(spec);
```

## How to Store and Reload a Schedule

Serialize the schedule to JSON, then pass that JSON to the constructor when
reloading it.

```python
blob = schedule.to_json()
again = Schedule.from_json(blob)
```

In JavaScript:

```js
const blob = JSON.stringify(schedule);
const again = new Schedule(blob);
```
