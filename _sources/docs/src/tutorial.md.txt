# Getting started

This tutorial walks you through your first schedule with `dateme`. By the end
you will have described a recurring event, asked when it next fires, and
projected a series of upcoming occurrences. You do not need to know anything
about the library yet — follow along and each step will produce the result
shown.

We will build one schedule: **"every Monday at 17:30 New York time, but skip
weeks when the New York Stock Exchange is closed on that Monday, moving those to
the next open day."**

## Before you start

Install the package:

```bash
pip install dateme
```

Open a Python session and import the one class you need:

```python
from datetime import datetime, timezone
from dateme import Schedule
```

## Step 1 — Describe the schedule

A schedule is written as JSON. Paste this in:

```python
spec = """
{
  "freq": { "type": "weekly", "days": ["mon"], "time": "17:30" },
  "timezone": "America/New_York",
  "overlays": [ { "calendar": "nyse_holiday", "rule": "exclude" } ],
  "makeup": "after",
  "start": null,
  "end": null
}
"""

schedule = Schedule.from_json(spec)
```

Read it back to yourself: fire **weekly** on **Monday** at **17:30**, in the
**America/New_York** timezone; **exclude** any date that is an **NYSE holiday**;
and when a Monday is dropped, **make it up after** — move to the next surviving
day.

## Step 2 — Check it is valid

Ask the schedule to validate itself. Nothing happens if it is well-formed — that
is success:

```python
schedule.validate()
```

## Step 3 — Ask when it next fires

Pick a reference instant and ask for the next occurrence after it. We use a
fixed date so your output matches this page exactly:

```python
after = datetime(2026, 1, 13, tzinfo=timezone.utc)
schedule.next(after)
```

You should see:

```text
datetime.datetime(2026, 1, 20, 22, 30, tzinfo=datetime.timezone.utc)
```

Look closely at what happened. The next Monday after January 13 is January 19,
2026 — but that is Martin Luther King Jr. Day and the NYSE is closed. The
`exclude` overlay dropped it, and `makeup: after` moved it to **Tuesday
January 20 at 17:30 New York time**, which is `22:30Z` (New York is five hours
behind UTC in January). The library did the calendar and timezone work for you.

## Step 4 — Project several occurrences

For a "next instances" list, ask for the next few at once:

```python
schedule.upcoming(3, after)
```

```text
[datetime.datetime(2026, 1, 20, 22, 30, tzinfo=datetime.timezone.utc),
 datetime.datetime(2026, 1, 26, 22, 30, tzinfo=datetime.timezone.utc),
 datetime.datetime(2026, 2, 2, 22, 30, tzinfo=datetime.timezone.utc)]
```

The first is the made-up Tuesday; the rest are ordinary Mondays.

## Step 5 — Look backward too

Every forward query has a backward twin. Ask what fired most recently before your
reference instant:

```python
schedule.previous(after)
```

```text
datetime.datetime(2026, 1, 12, 22, 30, tzinfo=datetime.timezone.utc)
```

Monday January 12 — a normal week.

## Step 6 — Get a whole series

To list every occurrence between two instants, use `until` (ascending) or `since`
(descending):

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

You described a recurring event as JSON, validated it, and asked five kinds of
question about it: `next`, `previous`, `upcoming`, `until`, and `since`. You saw
the engine apply a real market-holiday calendar and a makeup rule, and convert
local wall-clock times to UTC across a timezone offset — all without any manual
date arithmetic.

From here:

- To accomplish a specific real-world task, see the [How-to guides](how-to.md).
- For every field you can put in a schedule, see the [Schedule model](schedule-model.md).
- To understand *how* occurrences are computed, read [How the engine works](explanation.md).
- The same schedule works in the browser — see the [JavaScript API](api-javascript.md).
