# How the engine works

This page explains how `dateme` turns a schedule into concrete instants, and why
it is built the way it is. It is background reading — you do not need any of it to
use the library, but it clarifies the corner cases and the guarantees the API
makes.

## Occurrences and the series model

A schedule does not store a list of dates. It is a *rule*, and an **occurrence**
is one instant that rule produces — the wall-clock moment the recurring event
fires. A recurring event is therefore an open-ended series of occurrences, most
of which do not "exist" anywhere until you ask for them.

Every query is a question about that series relative to a reference instant:

- `next` / `previous` — the single occurrence immediately after / before it.
- `until` / `since` — the whole series in a window, ascending / descending.
- `upcoming` — a fixed count going forward.

All queries are **strict**: an occurrence exactly at a bound is excluded. This is
what makes the series composable — `next(t)` never returns `t` itself, so
repeatedly calling `next` walks the series without repeating. It is also why
`until(end)[0]` is exactly `next()` and `since(start)[0]` is exactly `previous()`:
the count-free series and the single-step queries are the same computation viewed
at different resolutions.

## Generation, then transformation

Occurrences are produced in two stages. First the **base occurrences** come
purely from the frequency and timezone — every Monday, every 1st and 15th, every
third day from an anchor, every matching cron minute, and so on. Then each base
occurrence is **transformed** by the overlays and makeup rule: kept, dropped, or
moved to a nearby day.

Keeping these stages separate is what lets the calendar rules stay simple. The
frequency knows nothing about holidays; the overlays know nothing about
weekdays-versus-month-days. They compose.

## Anchors and calendar shape

Some frequencies are anchored to calendar structure: monthly rules are anchored
to months, yearly rules to years, and quarterly rules to the three-month rhythm
of quarters. `every_n_days` and `every_n_weeks` need an explicit `start_date`
because their rhythm is relative rather than intrinsic to the calendar. Without
that anchor, "every 3 days" is underspecified: there are three equally valid
series depending on which date starts the cycle.

Cron is different again. It describes a set of matching local minutes rather
than a human calendar unit. `dateme` keeps it in the same pipeline as other
frequencies: cron creates base local datetimes, and overlays and makeup transform
them afterward.

(timezones-and-dst)=

## Timezones and DST

Occurrences are generated in the schedule's IANA timezone and then converted to
UTC. This is deliberate: "17:30 every weekday" should mean 17:30 *local* all year,
so the underlying UTC instant must shift when daylight-saving time changes. Anchor
the schedule to UTC directly (`"timezone": "UTC"`) if you want a fixed offset.

Two moments each year have no clean local-to-UTC mapping, and the engine resolves
them consistently:

- **Spring-forward gap.** Clocks jump forward, so a local time like 02:30 may not
  exist on that date. The occurrence moves to the first valid instant at or after
  the gap (03:00 local). The cycle still happens; it is nudged past the missing
  hour.
- **Autumn fall-back overlap.** Clocks repeat an hour, so a local time like 01:30
  happens twice. The engine uses the **earlier** of the two instants.

Hourly schedules are the one exception to the gap rule: the missing hour is simply
absent rather than nudged, because "every hour" already implies one occurrence per
real hour.

(overlays-and-makeup)=

## Overlays and makeup

An overlay tests an occurrence's **local date** against a calendar and either
keeps or drops it. The local date matters: a 23:30 New York occurrence is judged
on its New York calendar day, which can differ from its UTC day. Multiple overlays
are ANDed, which lets two independent senses compose — "skip holidays" and "only
on trading days" are just two overlays that must both pass.

When an overlay drops a base occurrence, the **makeup** rule decides what happens:

- `none` drops the cycle.
- `before` / `after` search outward day by day — up to 14 days — for the nearest
  day that passes *all* overlays, and move the occurrence there at the same
  time-of-day.
- `nearest` searches both directions and prefers the later date on ties.
- Weekday maps and cascades choose a direction based on context or try fallback
  strategies in order.

The 14-day bound is a safety valve. A pathological overlay set could in principle
remove every nearby day; rather than search forever, the engine gives up and drops
the occurrence. In practice no real market closes for anything close to 14
consecutive sessions, so the bound never bites.

### Why makeup needs care

Makeup can move an occurrence **earlier**, which breaks the naive assumption that
base occurrences come out already in order — a later base occurrence can make up
to a date ahead of an earlier one. The engine therefore generates a window of base
occurrences, transforms them all, and *then* sorts, rather than emitting them one
at a time. The output is always ascending regardless of how makeup reshuffled
things.

Makeup also raises the possibility of duplicates. Consider a daily schedule that
excludes a Friday holiday and makes up *before*: it would land on Thursday — but
Thursday is already a daily occurrence. Emitting both would double-count the
cycle. The engine deduplicates by exact UTC instant, so a made-up occurrence that
collides with an existing one is simply dropped. A weekly-Monday schedule making
up a Monday holiday to Tuesday keeps it, because Tuesday was not otherwise
scheduled.

## Why skip thresholds and gap checks are query rules

`skip_if_consecutive_excluded` looks at the base recurrence before makeup. It is
about the meaning of a cycle: if enough consecutive base cycles are excluded,
the whole run is intentionally skipped rather than compressed into nearby makeup
dates. This belongs before makeup because it decides whether the cycle should be
attempted at all.

`max_skip_gap` is different. It is a monitoring rule over the returned stream:
after overlays, makeup, deduplication, and bounds, is the resulting series too
sparse? That is why it raises during queries instead of changing the schedule
model itself.

## Why traces are separate from ordinary queries

Most callers only need instants. Returning metadata every time would make the
simple path noisier and would force every binding to expose a heavier result
type. Trace queries preserve the original datetime-returning API while giving
UIs and audit workflows a richer stream when they need it.

The trace reason is intentionally compact. It records whether the occurrence was
base or made up from a local date, plus whether DST shifted the local time. It is
not a full proof tree of every overlay decision.

## Bounds and termination

`start` and `end` clip the series to a half-open interval: no occurrence before
`start`, none at or after `end`. Because the comparison is on the final,
post-makeup instant, a made-up occurrence that lands outside the interval is
dropped just like a base one.

`end` also gives the series a definite tail: once the next occurrence would reach
it, `next` returns nothing. Binding-level iteration uses that property. A
`for...of` loop or Python `for` loop must have a finite end, so default iteration
requires `end`; explicit helpers such as `iter_upcoming` and `iterBetween` carry
their own count or window.

For unbounded schedules the engine expands its search window outward until it
either finds enough occurrences or reaches a large absolute horizon (about 50
years), after which it reports what it found. This is what guarantees that even a
schedule which can *never* fire — say, "only on NYSE trading days" applied to a
Sunday-only weekly rule — terminates and returns an empty result instead of
looping.

## The window, in brief

For a bounded query the engine widens the requested window by the makeup limit on
each side, generates every base occurrence whose local date falls in the widened
range, transforms and sorts them, then keeps those whose final instant lands in
the original window. Widening by the makeup limit is what makes each window
*complete*: any base occurrence whose makeup could reach into the window has
already been generated, so nothing is missed at the edges. Unbounded queries
(`next`, `previous`, `upcoming`) apply the same machinery inside an
expanding window.

## Why calendars are pluggable

The engine is generic over a calendar abstraction — "is this date in the set?" —
rather than hard-coding holiday tables. The built-in US-federal and NYSE calendars
are one implementation of that abstraction, supplied by the
[`finance-dates`](https://crates.io/crates/finance-dates) dataset. Keeping the
seam there means the scheduling logic is tested against small, hand-built fake
calendars for determinism, and the real holiday data is swapped in behind the same
interface — including in the WebAssembly build, where the same dataset compiles
and ships to the browser.
