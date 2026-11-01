# Getting started

Build a schedule for **every Monday at 17:30 New York time**, moving NYSE
holidays to the next date that passes the calendar filter. Then query its next,
previous, and upcoming occurrences using fixed dates so the results are
repeatable.

## Before you start

Install the package:

```bash
pip install dateme
```

Open a Python session and import `Schedule` and the datetime types:

```python
from datetime import datetime, timezone
from dateme import Schedule
```

## Step 1 — Describe the schedule

Create a schedule from this JSON:

```python
spec = """
{
  "frequency": { "type": "weekly", "days": ["mon"], "time": "17:30" },
  "timezone": "America/New_York",
  "overlays": [ { "calendar": "nyse_holiday", "rule": "exclude" } ],
  "makeup": "after",
  "start": null,
  "end": null
}
"""

schedule = Schedule.from_json(spec)
```

The `frequency` sets Monday at 17:30 in `America/New_York`. The overlay excludes
NYSE holidays, and `makeup: "after"` moves each excluded occurrence to the next
date that passes the overlay.

## Step 2 — Check it is valid

The constructor validates the schedule. Call `validate()` to check it again;
it returns without output when the schedule is valid:

```python
schedule.validate()
```

## Step 3 — Ask when it next fires

Ask for the first occurrence after January 13, 2026:

```python
after = datetime(2026, 1, 13, tzinfo=timezone.utc)
schedule.next(after)
```

You should see:

```text
datetime.datetime(2026, 1, 20, 22, 30, tzinfo=datetime.timezone.utc)
```

January 19 is Martin Luther King Jr. Day, when the NYSE is closed. The overlay
excludes that Monday, and makeup moves it to **Tuesday, January 20 at 17:30 New
York time**. The returned instant is `22:30Z`, five hours ahead of New York in
January.

## Step 4 — Project several occurrences

Request the next three occurrences from the same reference instant:

```python
schedule.upcoming(3, after)
```

```text
[datetime.datetime(2026, 1, 20, 22, 30, tzinfo=datetime.timezone.utc),
 datetime.datetime(2026, 1, 26, 22, 30, tzinfo=datetime.timezone.utc),
 datetime.datetime(2026, 2, 2, 22, 30, tzinfo=datetime.timezone.utc)]
```

The first is the Tuesday makeup occurrence; the other two are Mondays.

## Step 5 — Look backward too

Use `previous` to find the last occurrence before the same reference instant:

```python
schedule.previous(after)
```

```text
datetime.datetime(2026, 1, 12, 22, 30, tzinfo=datetime.timezone.utc)
```

The result is Monday, January 12 at 17:30 New York time.

## Step 6 — Get a whole series

Use `until` to list occurrences between two instants in ascending order:

```python
end = datetime(2026, 2, 15, tzinfo=timezone.utc)
for occurrence in schedule.until(end, after):
    print(occurrence)
```

```text
2026-01-20 22:30:00+00:00
2026-01-26 22:30:00+00:00
2026-02-02 22:30:00+00:00
2026-02-09 22:30:00+00:00
```

## What you have learned

The schedule uses a market-holiday filter and makeup rule to produce UTC
instants. You queried it with `next`, `previous`, `upcoming`, and `until`. For
the same window in descending order, use `since(after, end)`.

Next steps:

- For scheduling recipes, see the [How-to guides](how-to.md).
- For every field you can put in a schedule, see the [Schedule model](schedule-model.md).
- For recurrence generation and calendar behavior, read [How the engine works](explanation.md).
- To use the same schedule in the browser, see the [JavaScript API](api-javascript.md).
