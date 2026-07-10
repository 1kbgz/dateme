# Python API

Reference for the `dateme` Python package.

## Exports

Top-level package exports:

| Name                                                                     | Description                     |
| ------------------------------------------------------------------------ | ------------------------------- |
| `Schedule`                                                               | Query engine class.             |
| `model`                                                                  | Typed builder module.           |
| `Weekday`, `Nth`, `Makeup`, `MakeupFailure`, `OverlayRule`, `CalendarId` | Enum values for typed builders. |
| `MonthDay`, `NthWeekday`                                                 | Date-position builder types.    |
| `Overlay`, `AnyOverlay`                                                  | Overlay builder types.          |
| `CalendarDates`, `CalendarUnion`, `CalendarDiff`, `CustomCalendar`       | Calendar spec builder types.    |
| `Hourly`, `Daily`, `Weekly`, `EveryNDays`, `EveryNWeeks`                 | Frequency builder types.        |
| `MonthlyByDay`, `MonthlyByWeekday`, `Yearly`, `Quarterly`, `CustomCron`  | Frequency builder types.        |
| `MakeupStep`, `WeekdayMakeup`                                            | Makeup builder types.           |

## `Schedule`

`Schedule` is the Python wrapper around the Rust recurrence engine.

### Constructor

```python
Schedule(spec, calendar_provider=None)
```

Parameters:

| Parameter           | Type                                      | Description                                    |
| ------------------- | ----------------------------------------- | ---------------------------------------------- |
| `spec`              | JSON `str`, `dict`, or object `to_dict()` | Schedule model.                                |
| `calendar_provider` | callable or object, optional              | Provider for `{ "custom": "name" }` calendars. |

The constructor validates the schedule. Invalid JSON, invalid enum values,
invalid timezone names, and structural validation failures raise `ValueError`.

Examples:

```python
from dateme import Schedule

schedule = Schedule({
    "freq": {"type": "daily", "time": "09:00"},
    "timezone": "UTC",
})
```

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

### Custom Calendar Providers

Custom calendars are referenced in a schedule with `{ "custom": "name" }`.
Providers receive `(name, date)` where `date` is a `YYYY-MM-DD` string.

A callable provider:

```python
schedule = Schedule(
    {
        "freq": {"type": "daily", "time": "09:00"},
        "timezone": "UTC",
        "overlays": [{"calendar": {"custom": "shutdown"}, "rule": "exclude"}],
    },
    lambda name, date: name == "shutdown" and date == "2026-08-14",
)
```

An object provider:

```python
class Calendars:
    def contains(self, name: str, date: str) -> bool:
        return name == "shutdown" and date in {"2026-08-14", "2026-08-15"}


schedule = Schedule(spec, Calendars())
```

Missing custom calendar values are treated as absent from the set.

## Datetime Handling

| Rule             | Behavior                                                         |
| ---------------- | ---------------------------------------------------------------- |
| Input datetimes  | Timezone-aware `datetime` values are expected.                   |
| Naive inputs     | Interpreted as UTC by PyO3 datetime conversion.                  |
| Returned values  | Timezone-aware UTC `datetime` values.                            |
| Optional anchors | Default to the current UTC time.                                 |
| Query bounds     | Strict: occurrences exactly at `after` or `before` are excluded. |

## Methods

### `Schedule.from_json`

```python
Schedule.from_json(json, calendar_provider=None) -> Schedule
```

Builds a schedule from a JSON string.

```python
schedule = Schedule.from_json('{"freq":{"type":"daily","time":"09:00"},"timezone":"UTC"}')
```

### `Schedule.from_dict`

```python
Schedule.from_dict(spec, calendar_provider=None) -> Schedule
```

Builds a schedule from a `dict` or typed builder object.

```python
schedule = Schedule.from_dict({"freq": {"type": "daily", "time": "09:00"}, "timezone": "UTC"})
```

### `to_json`

```python
schedule.to_json() -> str
```

Returns the JSON representation.

```python
blob = schedule.to_json()
again = Schedule.from_json(blob)
```

### `to_dict`

```python
schedule.to_dict() -> dict
```

Returns the schedule model as a Python dictionary.

```python
spec = schedule.to_dict()
```

### `validate`

```python
schedule.validate() -> None
```

Re-runs structural validation. Raises `ValueError` on failure.

```python
schedule.validate()
```

### `next`

```python
schedule.next(after=None) -> datetime | None
```

Returns the first occurrence strictly after `after`.

```python
from datetime import datetime, timezone

after = datetime(2026, 1, 13, tzinfo=timezone.utc)
schedule.next(after)
```

### `previous`

```python
schedule.previous(before=None) -> datetime | None
```

Returns the last occurrence strictly before `before`.

```python
before = datetime(2026, 1, 13, tzinfo=timezone.utc)
schedule.previous(before)
```

### `until`

```python
schedule.until(before, after=None) -> list[datetime]
```

Returns occurrences in `(after, before)`, ascending.

```python
end = datetime(2026, 2, 1, tzinfo=timezone.utc)
schedule.until(end, after)
```

### `since`

```python
schedule.since(after, before=None) -> list[datetime]
```

Returns occurrences in `(after, before)`, descending.

```python
start = datetime(2026, 1, 1, tzinfo=timezone.utc)
schedule.since(start)
```

### `upcoming`

```python
schedule.upcoming(n, after=None) -> list[datetime]
```

Returns the next `n` occurrences strictly after `after`, ascending.

```python
schedule.upcoming(5, after)
```

### Trace Methods

Trace methods return dictionaries with `instant` and `reason`.

| Method                            | Return type    | Order      |
| --------------------------------- | -------------- | ---------- |
| `next_trace(after=None)`          | `dict \| None` | —          |
| `previous_trace(before=None)`     | `dict \| None` | —          |
| `until_trace(before, after=None)` | `list[dict]`   | ascending  |
| `since_trace(after, before=None)` | `list[dict]`   | descending |
| `upcoming_trace(n, after=None)`   | `list[dict]`   | ascending  |

Reason strings include:

| Reason form                    | Meaning                                         |
| ------------------------------ | ----------------------------------------------- |
| `base`                         | Base occurrence was kept.                       |
| `makeup_from(YYYY-MM-DD)`      | Occurrence was moved from the local date.       |
| `base,shifted_dst`             | Base occurrence was shifted through DST gap.    |
| `makeup_from(...),shifted_dst` | Made-up occurrence was shifted through DST gap. |

```python
trace = schedule.next_trace(after)
# {"instant": datetime(..., tzinfo=timezone.utc), "reason": "base"}
```

### `is_occurrence`

```python
schedule.is_occurrence(instant) -> bool
```

Returns whether `instant` is an occurrence of the schedule.

```python
schedule.is_occurrence(datetime(2026, 1, 20, 22, 30, tzinfo=timezone.utc))
```

### `count_between`

```python
schedule.count_between(after, before) -> int
```

Returns the number of occurrences strictly in `(after, before)`.

```python
schedule.count_between(after, end)
```

### `describe`

```python
schedule.describe() -> str
```

Returns a human-readable summary of the base recurrence, timezone, overlay
count, and makeup presence.

```python
schedule.describe()
# "Every Monday at 17:30 America/New_York, with 1 overlay(s), with makeup"
```

### Iteration

```python
iter(schedule) -> iterator[datetime]
schedule.iter_between(after, before) -> iterator[datetime]
schedule.iter_upcoming(n, after=None) -> iterator[datetime]
instant in schedule -> bool
```

`iter(schedule)` requires the schedule to have an `end` bound. It starts from
`start`, or from the current UTC time when `start` is absent.

```python
for instant in bounded_schedule:
    print(instant)

list(schedule.iter_between(after, end))
list(schedule.iter_upcoming(3, after))
instant in schedule
```

## Typed Model

`dateme.model` mirrors the [Schedule model](schedule-model.md) as dataclasses
and enums. Builders expose `to_dict()` and schedules expose `to_json()`.

```python
from dateme import (
    Schedule,
    MonthlyByWeekday,
    NthWeekday,
    Nth,
    Weekday,
    MonthDay,
    Overlay,
    CalendarId,
    OverlayRule,
    Makeup,
)
from dateme import model as m

spec = m.Schedule(
    freq=MonthlyByWeekday([NthWeekday(Nth.THIRD, Weekday.FRI)], "16:00"),
    timezone="America/New_York",
    overlays=[Overlay(CalendarId.NYSE_TRADING_DAY, OverlayRule.ONLY)],
    makeup=Makeup.NONE,
)
schedule = Schedule(spec)
```

Builder families:

| Family         | Builders                                                                                                                          |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Frequencies    | `Hourly`, `Daily`, `Weekly`, `EveryNDays`, `EveryNWeeks`, `MonthlyByDay`, `MonthlyByWeekday`, `Yearly`, `Quarterly`, `CustomCron` |
| Calendar specs | `CalendarId`, `CalendarDates`, `CalendarUnion`, `CalendarDiff`, `CustomCalendar`                                                  |
| Overlays       | `Overlay`, `AnyOverlay`, `OverlayRule`                                                                                            |
| Makeup         | `Makeup`, `MakeupFailure`, `MakeupStep`, `WeekdayMakeup`                                                                          |
| Date helpers   | `MonthDay`, `NthWeekday`, `Nth`, `Weekday`                                                                                        |

```{eval-rst}
.. autoclass:: dateme.Schedule
   :members:
   :special-members: __init__, __iter__, __contains__
   :member-order: bysource

.. automodule:: dateme.model
   :members:
   :member-order: bysource
```
