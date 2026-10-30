# How the engine works

`dateme` generates local recurrence times, applies calendar rules, and returns
UTC instants. Separating recurrence from calendar filtering makes the rules
reusable, but makeup and daylight-saving transitions need special handling.

## Occurrences and the series model

A schedule stores a rule rather than a list of dates. An occurrence is one UTC
instant produced by that rule. Queries compute the part of the series they
need, including any bounds and calendar adjustments.

Every query is a question about that series relative to a reference instant:

- `next` and `previous` find one occurrence after or before the reference instant.
- `until` and `since` return a window in ascending or descending order.
- `upcoming` requests a fixed number of future occurrences.

Query bounds are strict: an occurrence exactly at an `after` or `before` bound
is excluded. Calling `next(t)` with the previous result as `t` advances through
the series without repeating it. With the same explicit anchor and a nonempty
window, the first result of `until(end, after)` is `next(after)`. Likewise, the
first result of `since(start, before)` is `previous(before)`.

## Generation, then transformation

The engine first generates base occurrences from the frequency: every Monday,
the first and fifteenth of each month, every third day from an anchor, or each
matching cron minute. Overlays and makeup then keep, drop, or move those local
dates. The engine resolves the resulting local times to UTC in the schedule's
timezone.

The frequency defines when an event would occur without calendar filters.
Overlays test dates independently of that recurrence, so the same holiday rule
can filter a weekly, monthly, or cron schedule.

## Anchors and calendar shape

Monthly, yearly, and quarterly frequencies repeat within calendar units.
`every_n_days` and `every_n_weeks` instead need an explicit `start_date` to fix
the cycle. "Every 3 days" describes three possible series, depending on which
date starts it.

Cron describes matching local minutes rather than a calendar interval. It uses
the same processing as other frequencies: cron generates local datetimes, then
overlays and makeup adjust their dates.

(timezones-and-dst)=

## Timezones and DST

Recurrence times use the schedule's IANA timezone before conversion to UTC.
"17:30 every weekday" therefore stays at 17:30 local time when daylight-saving
time changes, while its UTC offset changes. A schedule with `"timezone": "UTC"`
has no daylight-saving offset changes.

In timezones with daylight-saving transitions, some local times are missing or
occur twice:

- **Spring-forward gap.** Clocks jump forward, so a local time may not exist.
  For non-hourly frequencies, the occurrence shifts to the first valid local
  time after the gap. In New York, 02:30 on the spring transition date shifts to
  03:00.
- **Autumn fall-back overlap.** Clocks repeat an hour, so a local time like 01:30
  happens twice. The engine uses the **earlier** of the two instants.

Hourly schedules skip nonexistent local hours. During a fall-back overlap, they
emit the repeated local hour only once, at its earlier UTC instant. They follow
local hour slots rather than fixed one-hour intervals in UTC.

(overlays-and-makeup)=

## Overlays and makeup

An overlay tests an occurrence's local date against a calendar. A 23:30 New York
occurrence is checked against its New York date, which can differ from its UTC
date. Top-level overlays are ANDed, so every filter must pass. An `any` group
provides an OR condition within those filters.

When an overlay drops a base occurrence, the **makeup** rule decides what happens:

- `none` drops the cycle.
- `before` and `after` search day by day, up to 14 days, for an earlier or later
  date that passes all overlays and destination constraints. They keep the
  original local time of day, subject to DST resolution.
- `nearest` searches both directions and prefers the later date on ties.
- Weekday maps and cascades choose a direction based on context or try fallback
  strategies in order.

The 14-day cap prevents makeup from searching indefinitely when no nearby date
passes the filters. `max_makeup_hops` can lower it. If the search fails,
`makeup_failure` determines whether to skip the occurrence, keep its original
date, or report an error. A selected `none` strategy skips it without applying
the failure policy.

### Why makeup needs care

Makeup can change occurrence order: a later base occurrence can move before an
earlier one. The engine therefore transforms a window of base occurrences and
orders the results by UTC instant. `until` and `upcoming` return ascending results;
`since` reverses them for descending order.

Makeup can also create duplicates. A daily schedule that excludes a Friday
holiday and makes it up on Thursday would produce two occurrences at Thursday's
scheduled time. The engine keeps one occurrence per exact UTC instant. A weekly
Monday schedule moved to Tuesday has no such collision if Tuesday was not
otherwise scheduled.

## Why skip thresholds and gap checks are query rules

`skip_if_consecutive_excluded` looks at the base recurrence before makeup. It is
used when a run of excluded cycles should be skipped instead of moved to nearby
dates. It must run before makeup because it determines which base occurrences
are eligible for adjustment.

`max_skip_gap` checks gaps after overlays, makeup, deduplication, and bounds. It
reports a query error when a checked gap exceeds the limit, without moving or
adding occurrences to fill that gap.

## Why traces are separate from ordinary queries

Ordinary queries return instants. Trace queries add context for callers that
need to explain an occurrence in a UI or audit log, without changing the result
type of ordinary queries.

The reason records whether an occurrence stayed on its base date or moved from
another local date, and whether DST shifted its local time. It does not record
every overlay decision.

## Bounds and termination

`start` and `end` limit the series to a half-open interval: no occurrence before
`start`, none at or after `end`. The engine checks the final instant after
makeup, so an adjustment outside the interval is dropped.

Once no occurrence remains before `end`, `next` returns no result. Default
iteration in Python and JavaScript requires this bound so the loop finishes.
Explicit helpers such as `iter_upcoming` and `iterBetween` use a count or query
window instead.

Queries without an explicit window expand their search until they find enough
occurrences or reach the schedule bound or absolute horizon, about 50 years
from the anchor. They then return what they found. The horizon also stops
searches for schedules that cannot produce an occurrence, such as a Sunday-only
rule restricted to NYSE trading days with no makeup.

## The window, in brief

For a bounded query, the engine adds a local-date margin large enough for makeup
on both sides of the requested window. It generates and transforms base
occurrences in that wider range, orders them, and keeps only final instants
inside the original query window and schedule bounds. The margin includes base
dates outside the query whose makeup could move them into it. `next`,
`previous`, and `upcoming` use the same process within an expanding window.

## Why calendars are pluggable

The engine asks a calendar provider whether a date belongs to a set. Built-in
US federal and NYSE calendars use
[`finance-dates`](https://crates.io/crates/finance-dates); inline date sets and
custom providers use the same filtering rules. Tests can therefore use small,
fixed calendars without depending on holiday data. The WebAssembly build
includes the built-in calendars, so browser schedules can use them too.
