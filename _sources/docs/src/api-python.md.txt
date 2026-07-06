# Python API

The Python package exposes a single class, `dateme.Schedule`. Build it from the
JSON [Schedule model](schedule-model.md); every method takes and returns
timezone-aware `datetime` objects (UTC). Where a reference instant is optional it
defaults to the current time.

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

`from_json` raises `ValueError` for malformed JSON or an unknown timezone/enum
value. `validate` raises `ValueError` for a structurally invalid schedule (see
[Validation](#validation)).

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
| `from_json(json)`          | `Schedule` (static)      | —          |

`until(end)[0]` equals `next()`; `since(start)[0]` equals `previous()`. All
results are strictly between the two bounds and deduplicated by instant.
