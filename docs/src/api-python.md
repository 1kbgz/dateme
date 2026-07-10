# Python API

The Python package exposes the `dateme.Schedule` engine plus a typed
[`dateme.model`](#typed-model) builder layer. Every query method takes and
returns timezone-aware `datetime` objects (UTC); where a reference instant is
optional it defaults to the current time.

## Constructing a schedule

`Schedule(spec)` accepts any of three forms, so you can construct from a native
object as well as JSON:

```python
from dateme import Schedule, model as m
from dateme import Weekly, Overlay, Makeup, CalendarId, OverlayRule, Weekday

# 1. Typed builder (validated as you build it)
spec = m.Schedule(
    freq=Weekly([Weekday.MON], "17:30"),
    timezone="America/New_York",
    overlays=[Overlay(CalendarId.NYSE_HOLIDAY, OverlayRule.EXCLUDE)],
    makeup=Makeup.AFTER,
)
schedule = Schedule(spec)

# 2. A plain dict
schedule = Schedule(spec.to_dict())

# 3. A JSON string
schedule = Schedule(spec.to_json())
```

Any object with a `to_dict()` method is accepted, which is what makes the typed
builders work directly. `Schedule.from_json(str)` and `Schedule.from_dict(obj)`
are explicit alternatives to the constructor.

Pass an optional custom calendar provider as the second argument when the spec
uses `{"custom": "name"}` calendar refs. The provider can be a callable or an
object with `contains(name, date)`, where `date` is a `"YYYY-MM-DD"` string:

```python
spec = {
    "freq": {"type": "daily", "time": "09:00"},
    "timezone": "UTC",
    "overlays": [{"calendar": {"custom": "shutdown"}, "rule": "exclude"}],
}

schedule = Schedule(spec, lambda name, date: name == "shutdown" and date == "2026-08-14")
```

```{eval-rst}
.. autoclass:: dateme.Schedule
   :members:
   :special-members: __init__
   :member-order: bysource
```

## Datetime handling

- Inputs should be timezone-aware `datetime` objects. Naive datetimes are
  interpreted as UTC.
- Returned datetimes are always timezone-aware and in UTC.
- Methods whose reference instant is optional (`next`, `previous`, `upcoming`,
  and the near bound of `until` / `since`) default it to the current UTC time.

## Errors

The constructor validates the schedule and raises `ValueError` for malformed
JSON/dict input, an unknown timezone/enum value, or a structurally invalid
schedule (see [Validation](#validation)) — so a `Schedule` you hold is always
well-formed. `validate()` re-runs the same check on demand. The typed builders
additionally raise `ValueError` at build time for out-of-range values (see
below).

## Method summary

| Method                     | Returns                  | Order      |
| -------------------------- | ------------------------ | ---------- |
| `next(after=now)`          | `datetime` or `None`     | —          |
| `previous(before=now)`     | `datetime` or `None`     | —          |
| `until(before, after=now)` | `list[datetime]`         | ascending  |
| `since(after, before=now)` | `list[datetime]`         | descending |
| `upcoming(n, after=now)`   | `list[datetime]`         | ascending  |
| `validate()`               | `None` (raises on error) | —          |
| `to_json()`                | `str`                    | —          |
| `to_dict()`                | `dict`                   | —          |
| `from_json(json)`          | `Schedule` (static)      | —          |
| `from_dict(obj)`           | `Schedule` (static)      | —          |

`until(end)[0]` equals `next()`; `since(start)[0]` equals `previous()`. All
results are strictly between the two bounds and deduplicated by instant.

(typed-model)=

## Typed model

`dateme.model` mirrors the [Schedule model](schedule-model.md) as dataclasses and
enums. Build a structure from them and pass it to `Schedule` (or call `to_dict()`
/ `to_json()`). Construction performs light validation — an out-of-range minute,
an empty weekday list, or a month day outside 1–31 raises `ValueError`
immediately. The enums (`Makeup`, `OverlayRule`, `CalendarId`, `Nth`, `Weekday`)
and the frequency/`MonthDay`/`NthWeekday`/`Overlay`/calendar builders are
re-exported at the package top level for convenience.

```{eval-rst}
.. automodule:: dateme.model
   :members:
   :member-order: bysource
```
