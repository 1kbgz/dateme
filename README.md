# dateme

Dates and intervals

[![Build Status](https://github.com/1kbgz/dateme/actions/workflows/build.yaml/badge.svg?branch=main&event=push)](https://github.com/1kbgz/dateme/actions/workflows/build.yaml)
[![codecov](https://codecov.io/gh/1kbgz/dateme/branch/main/graph/badge.svg)](https://codecov.io/gh/1kbgz/dateme)
[![License](https://img.shields.io/github/license/1kbgz/dateme)](https://github.com/1kbgz/dateme)
[![PyPI](https://img.shields.io/pypi/v/dateme.svg)](https://pypi.python.org/pypi/dateme)

## Overview

A recurrence / scheduling engine: pure datetime math that, given a **schedule**
(a frequency in an IANA timezone, plus optional calendar overlays, a makeup
strategy, and start/end bounds), computes when a recurring event fires. Written
in Rust with bindings for Python (pyo3) and JavaScript (WebAssembly).

A schedule is described as JSON:

```json
{
  "freq": { "type": "weekly", "days": ["mon"], "time": "17:30" },
  "timezone": "America/New_York",
  "overlays": [{ "calendar": "nyse_holiday", "rule": "exclude" }],
  "makeup": "after",
  "start": null,
  "end": null
}
```

Every binding exposes the same five queries over a `Schedule`:

- `next(after=now)` / `previous(before=now)` — the single next/previous occurrence.
- `until(before, after=now)` — the ascending series in `(after, before)`; `until(end)[0] == next()`.
- `since(after, before=now)` — the descending series in `(after, before)`; `since(start)[0] == previous()`.
- `upcoming(n, after=now)` — the next `n` occurrences.

### Python

```python
from datetime import datetime, timezone
from dateme import Schedule

s = Schedule(spec_json)             # JSON string, dict, or a typed model object
s.validate()
s.next(datetime(2026, 1, 13, tzinfo=timezone.utc))   # -> aware datetime | None
s.upcoming(3)                                        # defaults to now
```

Or build the schedule from typed objects instead of JSON:

```python
from dateme import Schedule, model as m
from dateme import Weekly, Overlay, Makeup, CalendarId, OverlayRule, Weekday

s = Schedule(m.Schedule(
    freq=Weekly([Weekday.MON], "17:30"),
    timezone="America/New_York",
    overlays=[Overlay(CalendarId.NYSE_HOLIDAY, OverlayRule.EXCLUDE)],
    makeup=Makeup.AFTER,
))
```

### JavaScript

```js
import init, { Schedule } from "dateme";

await init(); // load the wasm module once
const s = new Schedule(spec);              // string or object
s.next(new Date("2026-01-13T00:00:00Z"));  // -> Date | null
s.upcoming(3);                             // defaults to now
```

Supported frequencies: `hourly`, `daily`, `weekly`, `monthly_by_day` (fixed day
or `last`), `monthly_by_weekday` (nth / last weekday), and `yearly`. Overlays
filter occurrences against built-in calendars (`us_federal_holiday`,
`us_business_day`, `nyse_holiday`, `nyse_trading_day`, backed by
[`finance-dates`](https://crates.io/crates/finance-dates)); `makeup` shifts a
dropped occurrence to the nearest surviving day (`before` / `after`), or drops
it (`none`). DST gaps and overlaps are resolved on conversion to UTC.

> [!NOTE]
> This library was generated using [copier](https://copier.readthedocs.io/en/stable/) from the [Base Python Project Template repository](https://github.com/python-project-templates/base).
