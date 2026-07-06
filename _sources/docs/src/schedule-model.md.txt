# Schedule model

The complete JSON representation of a schedule. This is the storage and
construction format accepted by every binding. All times are wall-clock in the
schedule's `timezone` unless stated otherwise; seconds and nanoseconds are always
zero.

The same model is also available as typed builders — `dateme.model` dataclasses
in [Python](api-python.md) and `ScheduleSpec` types in
[JavaScript/TypeScript](api-javascript.md) — so you can construct a schedule from
native objects instead of hand-writing JSON.

## Schedule object

| Field      | Type                          | Required | Default  | Description                                                       |
| ---------- | ----------------------------- | -------- | -------- | ----------------------------------------------------------------- |
| `freq`     | [Frequency](#frequency)       | yes      | —        | The base recurrence.                                              |
| `timezone` | string (IANA name)            | yes      | —        | Timezone occurrences are generated in, e.g. `"America/New_York"`. |
| `overlays` | array of [Overlay](#overlays) | no       | `[]`     | Calendar filters, ANDed. Empty means no filtering.                |
| `makeup`   | [Makeup](#makeup)             | no       | `"none"` | What to do when an overlay drops an occurrence.                   |
| `start`    | RFC 3339 datetime or null     | no       | `null`   | No occurrence before this instant.                                |
| `end`      | RFC 3339 datetime or null     | no       | `null`   | No occurrence at or after this instant.                           |

`start` and `end` are UTC instants (e.g. `"2026-06-01T00:00:00Z"`). Comparison is
against the final occurrence instant, after any makeup.

(frequency)=

## Frequency

A tagged object; the `type` field selects the variant. Each variant sets its own
fields.

### `hourly`

One occurrence every hour, at `minute` past the hour, in local time.

| Field    | Type         | Description             |
| -------- | ------------ | ----------------------- |
| `type`   | `"hourly"`   |                         |
| `minute` | integer 0–59 | Minutes past each hour. |

```json
{ "type": "hourly", "minute": 30 }
```

### `daily`

One occurrence per local calendar day at `time`.

| Field  | Type             | Description  |
| ------ | ---------------- | ------------ |
| `type` | `"daily"`        |              |
| `time` | string `"HH:MM"` | Time of day. |

```json
{ "type": "daily", "time": "09:00" }
```

### `weekly`

One occurrence at `time` on each listed weekday.

| Field  | Type                     | Description                          |
| ------ | ------------------------ | ------------------------------------ |
| `type` | `"weekly"`               |                                      |
| `days` | array of weekday strings | Non-empty; `"mon"`…`"sun"`. Deduped. |
| `time` | string `"HH:MM"`         | Time of day.                         |

```json
{ "type": "weekly", "days": ["mon", "wed", "fri"], "time": "17:00" }
```

### `monthly_by_day`

One occurrence at `time` for each listed [MonthDay](#monthday) in each month.

| Field  | Type                           | Description         |
| ------ | ------------------------------ | ------------------- |
| `type` | `"monthly_by_day"`             |                     |
| `days` | array of [MonthDay](#monthday) | Non-empty. Deduped. |
| `time` | string `"HH:MM"`               | Time of day.        |

A fixed day that does not exist in a given month (e.g. day 31 in February) is
**skipped** for that month — it is never clamped to an earlier day.

```json
{ "type": "monthly_by_day", "days": [ { "type": "day", "value": 1 }, { "type": "last" } ], "time": "12:00" }
```

### `monthly_by_weekday`

One occurrence at `time` for each listed [NthWeekday](#nthweekday) in each month.

| Field      | Type                               | Description         |
| ---------- | ---------------------------------- | ------------------- |
| `type`     | `"monthly_by_weekday"`             |                     |
| `weekdays` | array of [NthWeekday](#nthweekday) | Non-empty. Deduped. |
| `time`     | string `"HH:MM"`                   | Time of day.        |

An nth-weekday that does not exist in a given month (e.g. a 5th Friday in a
4-Friday month) is **skipped** for that month.

```json
{ "type": "monthly_by_weekday", "weekdays": [ { "nth": "first", "weekday": "tue" } ], "time": "09:00" }
```

### `yearly`

One occurrence per year: the [MonthDay](#monthday) `day` within `month`, at
`time`. A nonexistent day is skipped for that year.

| Field   | Type                  | Description         |
| ------- | --------------------- | ------------------- |
| `type`  | `"yearly"`            |                     |
| `month` | integer 1–12          | Month of the year.  |
| `day`   | [MonthDay](#monthday) | Day within `month`. |
| `time`  | string `"HH:MM"`      | Time of day.        |

```json
{ "type": "yearly", "month": 7, "day": { "type": "day", "value": 4 }, "time": "12:00" }
```

(monthday)=

## MonthDay

A day within a month. A tagged object.

| Form                            | Meaning                             |
| ------------------------------- | ----------------------------------- |
| `{ "type": "day", "value": N }` | The Nth calendar day, N = 1–31.     |
| `{ "type": "last" }`            | The last calendar day of the month. |

(nthweekday)=

## NthWeekday

An ordinal weekday within a month.

| Field     | Type           | Description                    |
| --------- | -------------- | ------------------------------ |
| `nth`     | [Nth](#nth)    | Which occurrence in the month. |
| `weekday` | weekday string | `"mon"`…`"sun"`.               |

(nth)=

### Nth

One of: `"first"`, `"second"`, `"third"`, `"fourth"`, `"fifth"`, `"last"`.

(overlays)=

## Overlays

An overlay filters occurrences against a named calendar. An occurrence's **local
date** (in the schedule's timezone) is tested — not its UTC date. Multiple
overlays are **ANDed**: an occurrence survives only if it passes every overlay.

| Field      | Type                     | Description         |
| ---------- | ------------------------ | ------------------- |
| `calendar` | [CalendarId](#calendars) | Which calendar set. |
| `rule`     | `"exclude"` or `"only"`  | How to apply it.    |

| Rule      | Effect                                                            |
| --------- | ----------------------------------------------------------------- |
| `exclude` | Drop the occurrence if its local date **is** in the calendar.     |
| `only`    | Drop the occurrence if its local date is **not** in the calendar. |

For example `{"calendar": "nyse_holiday", "rule": "exclude"}` skips NYSE holidays;
`{"calendar": "nyse_trading_day", "rule": "only"}` keeps only NYSE session days
(also removing weekends).

(calendars)=

## Calendars

Built-in calendar identifiers for the `calendar` field, backed by the
[`finance-dates`](https://crates.io/crates/finance-dates) dataset.

| Identifier           | Date set                                                                                                     |
| -------------------- | ------------------------------------------------------------------------------------------------------------ |
| `us_federal_holiday` | Observed US federal holidays.                                                                                |
| `us_business_day`    | Weekdays that are not US federal holidays.                                                                   |
| `nyse_holiday`       | NYSE full-day market closures.                                                                               |
| `nyse_trading_day`   | NYSE session days (a weekday that is not an NYSE holiday). An early-close day still counts as a trading day. |

(makeup)=

## Makeup

What to do when an overlay drops a base occurrence. One of:

| Value      | Effect                                                                          |
| ---------- | ------------------------------------------------------------------------------- |
| `"none"`   | Skip the cycle entirely.                                                        |
| `"before"` | Move to the nearest **earlier** day that passes all overlays, at the same time. |
| `"after"`  | Move to the nearest **later** day that passes all overlays, at the same time.   |

The makeup search scans at most 14 days; if no surviving day is found within that
range the occurrence is dropped. A made-up occurrence that coincides with another
occurrence already produced by the schedule is dropped rather than duplicated.
See [Overlays and makeup](#overlays-and-makeup).

## Serialization notes

- `timezone` is the IANA name string (`"UTC"`, `"America/New_York"`, …).
- `time` is `"HH:MM"` (24-hour). Seconds are always zero.
- Weekdays are the lowercase three-letter strings `"mon"`…`"sun"`.
- `start` and `end` are RFC 3339 datetimes, or `null`.
- Enum-valued fields (`type`, `rule`, `nth`, `makeup`, `calendar`) use the
  lowercase `snake_case` spellings shown above.

(validation)=

## Validation

`validate` checks the schedule's structure and raises/throws on:

| Condition                                         | Message                  |
| ------------------------------------------------- | ------------------------ |
| `hourly.minute` outside 0–59                      | minute out of range      |
| Empty `days` / `weekdays` selection               | selection is empty       |
| A `MonthDay` `value` outside 1–31                 | month day out of range   |
| `yearly.month` outside 1–12                       | month out of range       |
| `start` not strictly before `end` (when both set) | start must be before end |

Duplicate entries in `days` / `weekdays` are removed rather than rejected.
