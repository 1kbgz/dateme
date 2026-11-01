# Schedule model

Schedules use the same JSON format for storage and construction in every
binding. Recurrence times are local wall-clock times in the schedule's
`timezone`, with seconds and nanoseconds set to zero. `start` and `end` are UTC
instants.

Typed alternatives to JSON are `dateme.model` dataclasses in
[Python](api-python.md) and the `ScheduleSpec` type in
[JavaScript/TypeScript](api-javascript.md).

## Schedule object

| Field                          | Type                              | Required | Default  | Description                                                       |
| ------------------------------ | --------------------------------- | -------- | -------- | ----------------------------------------------------------------- |
| `frequency`                    | [Frequency](#frequency)           | yes      | —        | The base recurrence; `freq` is a legacy input alias.              |
| `timezone`                     | string (IANA name)                | yes      | —        | Timezone occurrences are generated in, e.g. `"America/New_York"`. |
| `overlays`                     | array of [Overlay](#overlays)     | no       | `[]`     | Calendar filters, ANDed. Empty means no filtering.                |
| `makeup`                       | [Makeup](#makeup)                 | no       | `"none"` | What to do when an overlay drops an occurrence.                   |
| `max_makeup_hops`              | integer or null                   | no       | `null`   | Maximum days to scan for makeup; `null` uses the built-in limit.  |
| `makeup_failure`               | [Makeup failure](#makeup-failure) | no       | `"skip"` | What to do when makeup cannot find an allowed destination.        |
| `makeup_only_on`               | array of weekdays or null         | no       | `null`   | Allowed makeup destination weekdays.                              |
| `makeup_within_week`           | boolean                           | no       | `false`  | Keep makeup within the original ISO week.                         |
| `makeup_exclude_weekends`      | boolean                           | no       | `false`  | Reject Saturday/Sunday makeup destinations.                       |
| `makeup_before_next`           | boolean                           | no       | `false`  | Reject makeup on or across adjacent base occurrences.             |
| `skip_if_consecutive_excluded` | integer or null                   | no       | `null`   | Skip excluded base-occurrence runs at or above this length.       |
| `max_skip_gap`                 | integer or null                   | no       | `null`   | Error if a query window has a gap longer than this many days.     |
| `start`                        | RFC 3339 datetime or null         | no       | `null`   | No occurrence before this instant.                                |
| `end`                          | RFC 3339 datetime or null         | no       | `null`   | No occurrence at or after this instant.                           |

`start` and `end` use RFC 3339 timestamps such as `"2026-06-01T00:00:00Z"`. Bounds
apply to the final occurrence instant, after makeup.

### Frequency key compatibility

JSON/native schedule constructors accept `frequency` (preferred) or `freq`
(legacy), but reject both together, even if the values match. Serialization
outputs only `frequency`.

- Python model schedules prefer `model.Schedule(frequency=..., timezone=...)`
  and `.frequency`. The `freq=...` keyword and `.freq` attribute remain supported.
- TypeScript `ScheduleSpec` accepts exactly one of `frequency` or `freq`.
- Rust's public struct field remains `Schedule::freq` for source compatibility;
  its JSON representation uses `frequency`.

(frequency)=

## Frequency

A tagged object whose `type` field selects the recurrence variant. The remaining
fields depend on that variant.

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

A fixed day that does not exist in a month, such as February 31, is skipped for
that month. It is not clamped to an earlier day.

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

An nth weekday that does not exist in a month, such as a fifth Friday in a month
with four Fridays, is skipped for that month.

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

### `every_n_days`

One occurrence every `interval` days, anchored to `start_date`, at `time`.

| Field        | Type             | Description                |
| ------------ | ---------------- | -------------------------- |
| `type`       | `"every_n_days"` |                            |
| `interval`   | integer >= 1     | Number of days per step.   |
| `start_date` | date string      | Anchor date, `YYYY-MM-DD`. |
| `time`       | string `"HH:MM"` | Time of day.               |

```json
{ "type": "every_n_days", "interval": 3, "start_date": "2026-01-01", "time": "09:00" }
```

### `every_n_weeks`

One occurrence on each selected weekday every `interval` weeks. The repeating
week cycle is anchored to the week containing `start_date`.

| Field        | Type                     | Description                          |
| ------------ | ------------------------ | ------------------------------------ |
| `type`       | `"every_n_weeks"`        |                                      |
| `interval`   | integer >= 1             | Number of weeks per step.            |
| `start_date` | date string              | Anchor date, `YYYY-MM-DD`.           |
| `days`       | array of weekday strings | Non-empty; `"mon"`…`"sun"`. Deduped. |
| `time`       | string `"HH:MM"`         | Time of day.                         |

```json
{ "type": "every_n_weeks", "interval": 2, "start_date": "2026-01-05", "days": ["mon", "thu"], "time": "17:00" }
```

### `quarterly`

One occurrence every quarter, at `time`, using `month` as the month within each
quarter. `month: 1` means Jan/Apr/Jul/Oct, `2` means Feb/May/Aug/Nov, and `3`
means Mar/Jun/Sep/Dec. A nonexistent [MonthDay](#monthday) is skipped for that
quarter.

| Field   | Type                  | Description                   |
| ------- | --------------------- | ----------------------------- |
| `type`  | `"quarterly"`         |                               |
| `month` | integer 1-3           | Month within each quarter.    |
| `day`   | [MonthDay](#monthday) | Day within the quarter month. |
| `time`  | string `"HH:MM"`      | Time of day.                  |

```json
{ "type": "quarterly", "month": 1, "day": { "type": "day", "value": 15 }, "time": "12:00" }
```

### `custom_cron`

One occurrence for every local minute matching a five-field cron expression:
`minute hour day-of-month month day-of-week`.

| Field  | Type            | Description       |
| ------ | --------------- | ----------------- |
| `type` | `"custom_cron"` |                   |
| `expr` | string          | Five cron fields. |

Fields support `*`, comma lists, ranges, and step syntax such as `*/15` or
`1-5/2`. Day-of-week accepts `0`–`7`, where `0` and `7` both mean Sunday. When both
day-of-month and day-of-week are constrained, both must match.

```json
{ "type": "custom_cron", "expr": "30 9 * * 1-5" }
```

(monthday)=

## MonthDay

A tagged object identifying a day within a month.

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

An overlay filters occurrences against a calendar or a group of overlays. It
tests the occurrence's local date in the schedule's timezone, not its UTC date.
Top-level overlays are ANDed: an occurrence is kept only if it passes every
overlay.

Calendar overlay fields:

| Field      | Type                    | Description                                            |
| ---------- | ----------------------- | ------------------------------------------------------ |
| `calendar` | [Calendar](#calendars)  | Which calendar set.                                    |
| `rule`     | `"exclude"` or `"only"` | How to apply it.                                       |
| `makeup`   | [Makeup](#makeup)       | Optional makeup override when this overlay drops date. |

| Rule      | Effect                                                            |
| --------- | ----------------------------------------------------------------- |
| `exclude` | Drop the occurrence if its local date **is** in the calendar.     |
| `only`    | Drop the occurrence if its local date is **not** in the calendar. |

For example, `{"calendar": "nyse_holiday", "rule": "exclude"}` skips NYSE holidays;
`{"calendar": "nyse_trading_day", "rule": "only"}` keeps only NYSE session days
(also removing weekends).

Use `any` to create an OR group. The group passes when any child overlay passes.

```json
{
  "any": [
    { "calendar": "us_federal_holiday", "rule": "exclude" },
    { "calendar": "nyse_holiday", "rule": "exclude" }
  ],
  "makeup": "none"
}
```

When a date is dropped, the engine uses the first failing top-level overlay's
`makeup` override, falling back to the schedule-level `makeup` if that overlay
has none. For a failing `any` group, the group's override takes precedence over
the first child override present.

(calendars)=

## Calendars

The `calendar` field accepts built-in identifiers or inline calendar specs.
Built-in calendars are backed by the
[`finance-dates`](https://crates.io/crates/finance-dates) dataset.

| Identifier           | Date set                                                                                                     |
| -------------------- | ------------------------------------------------------------------------------------------------------------ |
| `us_federal_holiday` | Observed US federal holidays.                                                                                |
| `us_business_day`    | Weekdays that are not US federal holidays.                                                                   |
| `nyse_holiday`       | NYSE full-day market closures.                                                                               |
| `nyse_trading_day`   | NYSE session days (a weekday that is not an NYSE holiday). An early-close day still counts as a trading day. |

Inline date sets:

```json
{ "dates": ["2026-07-03", "2026-07-04", "2026-12-25"] }
```

Calendar set algebra:

```json
{ "union": ["nyse_holiday", "us_federal_holiday"] }
{ "diff": ["us_federal_holiday", "nyse_holiday"] }
```

`union` contains dates present in any child calendar. `diff` contains dates in
the first child calendar, excluding dates present in any of the remaining
calendars. Children can be built-ins or nested calendar specs.

Custom provider calendars:

```json
{ "custom": "company_shutdown" }
```

Python and JavaScript constructors accept an optional provider used to resolve
`custom` calendars at query time. Missing custom calendars are treated as not in
the set.

(makeup)=

## Makeup

Makeup determines what happens when an overlay drops a base occurrence. A single
direction applies to every excluded date:

| Value       | Effect                                                                            |
| ----------- | --------------------------------------------------------------------------------- |
| `"none"`    | Skip the cycle entirely.                                                          |
| `"before"`  | Move to the nearest **earlier** day that passes all overlays, at the same time.   |
| `"after"`   | Move to the nearest **later** day that passes all overlays, at the same time.     |
| `"nearest"` | Move to the nearest day that passes all overlays, preferring later dates on ties. |

A weekday map selects a direction by the excluded date's weekday:

```json
{
  "mon": "after",
  "fri": "before",
  "default": "none"
}
```

Weekday keys are optional. If an excluded date's weekday is not present, the
engine uses `default`; if `default` is absent, the occurrence is skipped.

A cascade specifies fallback strategies:

```json
[
  { "direction": "after", "max_hops": 3 },
  { "direction": "before", "max_hops": 3 },
  "none"
]
```

Steps are tried in order until one finds a destination or disables makeup. A
step can be a direction string or an object with `direction` and optional
`max_hops`.

The makeup search scans at most 14 days in each permitted direction.
`max_makeup_hops` can lower that limit: `null` or an absent field uses 14 days,
`0` prevents any search, and a positive integer is capped at 14. A zero limit
with an enabled direction triggers `makeup_failure` for excluded occurrences.
Cascade steps use the schedule-level limit unless their `max_hops` sets a lower
one.

Set `makeup_only_on` to restrict makeup destination dates to specific weekdays:

```json
{ "makeup_only_on": ["tue", "wed", "thu"] }
```

Additional target constraints:

| Field                     | Effect                                                        |
| ------------------------- | ------------------------------------------------------------- |
| `makeup_within_week`      | Keep makeup within the original excluded date's ISO week.     |
| `makeup_exclude_weekends` | Reject Saturday and Sunday makeup destinations.               |
| `makeup_before_next`      | Reject makeup that lands on or crosses an adjacent base date. |

Occurrences with the same final UTC instant are deduplicated. See
[How the engine works](explanation.md).

(makeup-failure)=

## Makeup Failure

The failure policy applies when an enabled makeup strategy cannot find an
allowed destination within its search limit. It includes `"before"`, `"after"`,
`"nearest"`, weekday-selected directions, and exhausted cascades. One of:

| Value             | Effect                                                   |
| ----------------- | -------------------------------------------------------- |
| `"skip"`          | Drop the occurrence silently.                            |
| `"keep_original"` | Emit the occurrence on its original excluded day.        |
| `"error"`         | Raise/throw from fallible queries and Python/JS methods. |

When the selected direction is `"none"`, including a cascade step, the cycle is
skipped and `makeup_failure` is ignored.

(threshold-skip)=

## Threshold Skip

Set `skip_if_consecutive_excluded` to skip runs of excluded base occurrences
before makeup is applied.

```json
{ "skip_if_consecutive_excluded": 2 }
```

The value is a positive integer. `null` or an absent field disables the rule. A
run is counted over consecutive entries in the base recurrence series. When a
run length is at least the threshold, every excluded base occurrence in that run
is dropped and does not use `makeup` or `makeup_failure`.

Set `max_skip_gap` to make queries fail if the returned occurrence stream has a
gap longer than the configured number of days.

```json
{ "max_skip_gap": 5 }
```

`until` and `since` check the complete query window. `next`, `previous`, and
`upcoming` check the gap from the anchor through returned occurrences, but do
not treat the open search horizon after the last returned occurrence as a gap.

## Serialization notes

- The recurrence key is always `frequency`, even when input used `freq`.
- `timezone` is the IANA name string (`"UTC"`, `"America/New_York"`, …).
- `time` is `"HH:MM"` (24-hour). Seconds are always zero.
- Weekdays are the lowercase three-letter strings `"mon"`…`"sun"`.
- `start` and `end` are RFC 3339 datetimes, or `null`.
- Enum-valued fields (`type`, `rule`, `nth`, `makeup`, weekday makeup values,
  `makeup_failure`, `calendar`) use the lowercase `snake_case` spellings shown
  above.
- `skip_if_consecutive_excluded` must be `null`, absent, or an integer at least
  `1`.
- `max_skip_gap` must be `null`, absent, or an integer at least `1`.
- `makeup_only_on` must be `null`, absent, or an array of weekday strings.
- `every_n_days.interval` and `every_n_weeks.interval` must be at least `1`.
- `quarterly.month` must be `1`, `2`, or `3`.
- `custom_cron.expr` must be a valid five-field expression.
- `union` and `diff` calendar groups must have at least one child calendar.
- `custom` calendar names must be non-empty strings.

(validation)=

## Validation

`validate` checks the schedule's structure and raises/throws on:

| Condition                                         | Message                     |
| ------------------------------------------------- | --------------------------- |
| `hourly.minute` outside 0–59                      | minute out of range         |
| Empty `days` / `weekdays` selection               | selection is empty          |
| A `MonthDay` `value` outside 1–31                 | month day out of range      |
| `yearly.month` outside 1–12                       | month out of range          |
| `every_n_days` / `every_n_weeks` interval below 1 | interval out of range       |
| `quarterly.month` outside 1–3                     | quarter month out of range  |
| Invalid five-field cron expression                | invalid cron expression     |
| `skip_if_consecutive_excluded` less than 1        | skip threshold out of range |
| `max_skip_gap` less than 1                        | max skip gap out of range   |
| Empty overlay `any` group                         | overlay group is empty      |
| Empty `union` / `diff` calendar group             | calendar group is empty     |
| Empty `custom` calendar name                      | custom calendar name empty  |
| `start` not strictly before `end` (when both set) | start must be before end    |

Duplicate entries in `days` / `weekdays` are removed rather than rejected.
