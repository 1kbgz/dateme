//! Spec §15 test vectors plus serde / validation coverage. Uses fake calendars
//! for determinism (spec §13).

use crate::schedule::*;
use crate::NoCalendars;
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;

fn utc(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn hm(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn ny() -> Tz {
    "America/New_York".parse().unwrap()
}

fn day(value: u8) -> MonthDay {
    MonthDay::Day { value }
}

// ---- Test vector 1: weekly Monday, NYSE-exclude, makeup after ----

#[test]
fn weekly_monday_nyse_exclude_after() {
    // Fake NYSE holiday: Mon 2026-01-19 (MLK Day).
    let cal = |id: CalendarId, d: NaiveDate| match id {
        CalendarId::NyseHoliday => Some(d == date(2026, 1, 19)),
        _ => Some(false),
    };
    let s = Schedule {
        freq: Frequency::Weekly {
            days: vec![Weekday::Mon],
            time: hm(17, 30),
        },
        timezone: ny(),
        overlays: vec![Overlay {
            calendar: CalendarId::NyseHoliday,
            rule: OverlayRule::Exclude,
        }],
        makeup: Makeup::After,
        max_makeup_hops: None,
        start: None,
        end: None,
    };

    assert_eq!(
        s.next(utc("2026-01-12T00:00:00Z"), &cal),
        Some(utc("2026-01-12T22:30:00Z"))
    );
    // The 01-19 Monday is a holiday ⇒ makeup after ⇒ Tue 2026-01-20 17:30 ET.
    assert_eq!(
        s.next(utc("2026-01-13T00:00:00Z"), &cal),
        Some(utc("2026-01-20T22:30:00Z"))
    );
}

#[test]
fn makeup_hops_cap_drops_when_next_day_is_excluded() {
    let cal = |id: CalendarId, d: NaiveDate| match id {
        CalendarId::NyseHoliday => Some(d == date(2026, 1, 19) || d == date(2026, 1, 20)),
        _ => Some(false),
    };
    let mut s = Schedule {
        freq: Frequency::Weekly {
            days: vec![Weekday::Mon],
            time: hm(17, 30),
        },
        timezone: ny(),
        overlays: vec![Overlay {
            calendar: CalendarId::NyseHoliday,
            rule: OverlayRule::Exclude,
        }],
        makeup: Makeup::After,
        max_makeup_hops: None,
        start: None,
        end: None,
    };

    assert_eq!(
        s.next(utc("2026-01-13T00:00:00Z"), &cal),
        Some(utc("2026-01-21T22:30:00Z"))
    );

    s.max_makeup_hops = Some(1);
    assert_eq!(
        s.next(utc("2026-01-13T00:00:00Z"), &cal),
        Some(utc("2026-01-26T22:30:00Z"))
    );

    s.max_makeup_hops = Some(0);
    assert_eq!(
        s.next(utc("2026-01-13T00:00:00Z"), &cal),
        Some(utc("2026-01-26T22:30:00Z"))
    );
}

// ---- Test vector 2: daily, exclude holiday, before, dedup ----

#[test]
fn daily_exclude_before_dedup() {
    let cal = |_id: CalendarId, d: NaiveDate| Some(d == date(2026, 7, 3));
    let s = Schedule {
        freq: Frequency::Daily { time: hm(9, 0) },
        timezone: Tz::UTC,
        overlays: vec![Overlay {
            calendar: CalendarId::UsFederalHoliday,
            rule: OverlayRule::Exclude,
        }],
        makeup: Makeup::Before,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    // 07-03 makes up before to 07-02, but 07-02 is already scheduled ⇒ dropped.
    let got = s.upcoming(5, utc("2026-07-01T00:00:00Z"), &cal);
    assert_eq!(
        got,
        vec![
            utc("2026-07-01T09:00:00Z"),
            utc("2026-07-02T09:00:00Z"),
            utc("2026-07-04T09:00:00Z"),
            utc("2026-07-05T09:00:00Z"),
            utc("2026-07-06T09:00:00Z"),
        ]
    );
}

// ---- Test vector 3: MonthlyByDay Day(31) ----

#[test]
fn monthly_day_31_skips_short_months() {
    let s = Schedule {
        freq: Frequency::MonthlyByDay {
            days: vec![day(31)],
            time: hm(12, 0),
        },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    let got = s.upcoming(3, utc("2026-01-01T00:00:00Z"), &NoCalendars);
    assert_eq!(
        got,
        vec![
            utc("2026-01-31T12:00:00Z"),
            utc("2026-03-31T12:00:00Z"),
            utc("2026-05-31T12:00:00Z"),
        ]
    );
}

// ---- Test vector 4: MonthlyByWeekday 5th Friday ----

#[test]
fn monthly_fifth_friday() {
    let s = Schedule {
        freq: Frequency::MonthlyByWeekday {
            weekdays: vec![NthWeekday {
                nth: Nth::Fifth,
                weekday: Weekday::Fri,
            }],
            time: hm(12, 0),
        },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    // Jan 2026 has 5 Fridays; the 5th is Jan 30.
    assert_eq!(
        s.next(utc("2026-01-01T00:00:00Z"), &NoCalendars),
        Some(utc("2026-01-30T12:00:00Z"))
    );
}

// ---- Test vector 5: last business day, makeup before ----

#[test]
fn last_business_day_before() {
    // Fake US business day = Mon..Fri.
    let cal = |id: CalendarId, d: NaiveDate| match id {
        CalendarId::UsBusinessDay => Some(!matches!(d.weekday(), Weekday::Sat | Weekday::Sun)),
        _ => Some(false),
    };
    let s = Schedule {
        freq: Frequency::MonthlyByDay {
            days: vec![MonthDay::Last],
            time: hm(16, 0),
        },
        timezone: Tz::UTC,
        overlays: vec![Overlay {
            calendar: CalendarId::UsBusinessDay,
            rule: OverlayRule::Only,
        }],
        makeup: Makeup::Before,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    // 2026-05-31 is Sunday ⇒ makeup before ⇒ Fri 2026-05-29.
    let got = s.until(
        utc("2026-06-01T00:00:00Z"),
        utc("2026-05-01T00:00:00Z"),
        &cal,
    );
    assert_eq!(got, vec![utc("2026-05-29T16:00:00Z")]);
}

// ---- Test vector 6: DST spring-forward ----

#[test]
fn dst_spring_forward() {
    let s = Schedule {
        freq: Frequency::Daily { time: hm(2, 30) },
        timezone: ny(),
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    // 02:30 on 2026-03-08 is in the gap ⇒ first valid instant 03:00 ET = 07:00Z.
    assert_eq!(
        s.next(utc("2026-03-08T00:00:00Z"), &NoCalendars),
        Some(utc("2026-03-08T07:00:00Z"))
    );
}

// ---- Test vector 7: DST fall-back ----

#[test]
fn dst_fall_back() {
    let s = Schedule {
        freq: Frequency::Daily { time: hm(1, 30) },
        timezone: ny(),
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    // 01:30 occurs twice on 2026-11-01 ⇒ earliest instant = 05:30Z.
    assert_eq!(
        s.next(utc("2026-11-01T00:00:00Z"), &NoCalendars),
        Some(utc("2026-11-01T05:30:00Z"))
    );
}

// ---- DST: off-grid spring-forward lands exactly on the gap edge ----

#[test]
fn dst_spring_forward_off_grid_minutes() {
    // Every local time in the 02:00–03:00 gap resolves to the gap edge (03:00 ET
    // = 07:00Z) regardless of its minute — no overshoot.
    for (h, m) in [(2, 0), (2, 20), (2, 30), (2, 59)] {
        let s = Schedule {
            freq: Frequency::Daily { time: hm(h, m) },
            timezone: ny(),
            overlays: vec![],
            makeup: Makeup::None,
            max_makeup_hops: None,
            start: None,
            end: None,
        };
        assert_eq!(
            s.next(utc("2026-03-08T00:00:00Z"), &NoCalendars),
            Some(utc("2026-03-08T07:00:00Z")),
            "gap time {h:02}:{m:02} should resolve to 03:00 ET"
        );
    }
}

// ---- DST: hourly emits 23 hours on spring-forward, 24 on fall-back ----

#[test]
fn dst_hourly_counts() {
    let s = Schedule {
        freq: Frequency::Hourly { minute: 30 },
        timezone: ny(),
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };

    // Spring-forward day: the 02:30 slot is missing ⇒ 23 occurrences.
    let spring = s.until(
        utc("2026-03-09T04:00:00Z"), // 2026-03-09 00:00 ET (EDT, UTC-4)
        utc("2026-03-08T05:00:00Z"), // 2026-03-08 00:00 ET (EST, UTC-5)
        &NoCalendars,
    );
    assert_eq!(spring.len(), 23);

    // Fall-back day: the repeated 01:30 appears once ⇒ 24 occurrences.
    let fall = s.until(
        utc("2026-11-02T05:00:00Z"), // 2026-11-02 00:00 ET
        utc("2026-11-01T04:00:00Z"), // 2026-11-01 00:00 ET
        &NoCalendars,
    );
    assert_eq!(fall.len(), 24);
    // The repeated hour is present exactly once, at its earliest instant (05:30Z).
    assert_eq!(
        fall.iter()
            .filter(|t| **t == utc("2026-11-01T05:30:00Z"))
            .count(),
        1
    );
}

// ---- DST: sub-hour transition zone (Lord Howe, 30-min shift) ----

#[test]
fn dst_sub_hour_zone() {
    // Lord Howe Island springs forward 30 min (02:00 -> 02:30) on 2026-10-04.
    let tz: Tz = "Australia/Lord_Howe".parse().unwrap();
    let s = Schedule {
        freq: Frequency::Daily { time: hm(2, 15) },
        timezone: tz,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    // 02:15 is inside the 30-min gap ⇒ resolves to the edge 02:30 local, and the
    // engine returns a valid instant (not skipped).
    let got = s.next(utc("2026-10-03T00:00:00Z"), &NoCalendars);
    assert!(got.is_some());
    // Reconverting to local yields 02:30 on the transition date.
    let local = got.unwrap().with_timezone(&tz);
    assert_eq!(
        local.format("%Y-%m-%d %H:%M").to_string(),
        "2026-10-04 02:30"
    );
}

// ---- Test vector 8: end bound ----

#[test]
fn end_bound() {
    let s = Schedule {
        freq: Frequency::Daily { time: hm(12, 0) },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: Some(utc("2026-01-03T00:00:00Z")),
    };
    assert_eq!(s.next(utc("2026-01-02T13:00:00Z"), &NoCalendars), None);
}

// ---- Test vector 9: start bound (future start) ----

#[test]
fn start_bound() {
    let s = Schedule {
        freq: Frequency::Daily { time: hm(12, 0) },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: Some(utc("2026-06-01T00:00:00Z")),
        end: None,
    };
    assert_eq!(
        s.next(utc("2026-01-01T00:00:00Z"), &NoCalendars),
        Some(utc("2026-06-01T12:00:00Z"))
    );
}

// ---- Test vector 10: weekly multi-day ----

#[test]
fn weekly_multi_day() {
    let s = Schedule {
        freq: Frequency::Weekly {
            days: vec![Weekday::Mon, Weekday::Wed, Weekday::Fri],
            time: hm(17, 0),
        },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    // after = Sun 2026-01-04.
    let got = s.upcoming(3, utc("2026-01-04T00:00:00Z"), &NoCalendars);
    assert_eq!(
        got,
        vec![
            utc("2026-01-05T17:00:00Z"),
            utc("2026-01-07T17:00:00Z"),
            utc("2026-01-09T17:00:00Z"),
        ]
    );
}

// ---- Robustness: an out-of-range month must not panic ----

#[test]
fn invalid_yearly_month_does_not_panic() {
    // An unvalidated Yearly with month 13 must produce no occurrence rather than
    // panicking inside the calendar math.
    let s = Schedule {
        freq: Frequency::Yearly {
            month: 13,
            day: MonthDay::Day { value: 1 },
            time: hm(12, 0),
        },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    assert_eq!(s.next(utc("2026-01-01T00:00:00Z"), &NoCalendars), None);
    assert!(s
        .upcoming(3, utc("2026-01-01T00:00:00Z"), &NoCalendars)
        .is_empty());
}

// ---- Backward direction: previous / since ----

#[test]
fn previous_and_since_symmetry() {
    let s = Schedule {
        freq: Frequency::Daily { time: hm(12, 0) },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    let before = utc("2026-01-10T00:00:00Z");
    assert_eq!(
        s.previous(before, &NoCalendars),
        Some(utc("2026-01-09T12:00:00Z"))
    );

    // since(after)[0] == previous(before); descending.
    let after = utc("2026-01-07T00:00:00Z");
    let got = s.since(after, before, &NoCalendars);
    assert_eq!(
        got,
        vec![
            utc("2026-01-09T12:00:00Z"),
            utc("2026-01-08T12:00:00Z"),
            utc("2026-01-07T12:00:00Z"),
        ]
    );
    assert_eq!(got.first().copied(), s.previous(before, &NoCalendars));
}

#[test]
fn until_first_equals_next() {
    let s = Schedule {
        freq: Frequency::Daily { time: hm(12, 0) },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    let after = utc("2026-01-01T00:00:00Z");
    let before = utc("2026-01-05T00:00:00Z");
    let series = s.until(before, after, &NoCalendars);
    assert_eq!(series.first().copied(), s.next(after, &NoCalendars));
    assert_eq!(series.len(), 4); // 01,02,03,04 at 12:00
}

#[test]
fn strictly_after_excludes_exact() {
    let s = Schedule {
        freq: Frequency::Daily { time: hm(12, 0) },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    // after exactly on an occurrence ⇒ that instant is excluded.
    assert_eq!(
        s.next(utc("2026-01-02T12:00:00Z"), &NoCalendars),
        Some(utc("2026-01-03T12:00:00Z"))
    );
}

#[test]
fn overlay_removes_everything_terminates() {
    // Only(a calendar that is empty) with makeup none ⇒ nothing ever fires.
    let cal = |_id: CalendarId, _d: NaiveDate| Some(false);
    let s = Schedule {
        freq: Frequency::Daily { time: hm(12, 0) },
        timezone: Tz::UTC,
        overlays: vec![Overlay {
            calendar: CalendarId::NyseTradingDay,
            rule: OverlayRule::Only,
        }],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    assert_eq!(s.next(utc("2026-01-01T00:00:00Z"), &cal), None);
    assert!(s.upcoming(10, utc("2026-01-01T00:00:00Z"), &cal).is_empty());
}

// ---- serde ----

#[test]
fn serde_roundtrip_nyse_monday() {
    let json = r#"{
        "freq": { "type": "weekly", "days": ["mon"], "time": "17:30" },
        "timezone": "America/New_York",
        "overlays": [ { "calendar": "nyse_holiday", "rule": "exclude" } ],
        "makeup": "after",
        "start": null,
        "end": null
    }"#;
    let s: Schedule = serde_json::from_str(json).unwrap();
    assert_eq!(s.timezone, ny());
    assert_eq!(s.makeup, Makeup::After);
    assert!(matches!(s.freq, Frequency::Weekly { .. }));

    // round-trips back to equivalent structure.
    let out = serde_json::to_string(&s).unwrap();
    let s2: Schedule = serde_json::from_str(&out).unwrap();
    assert_eq!(s, s2);
}

#[test]
fn serde_monthday_forms() {
    let json = r#"{ "freq": { "type": "monthly_by_day",
            "days": [ {"type":"day","value":1}, {"type":"last"} ], "time": "12:00" },
        "timezone": "UTC", "overlays": [], "makeup": "none", "start": null, "end": null }"#;
    let s: Schedule = serde_json::from_str(json).unwrap();
    if let Frequency::MonthlyByDay { days, .. } = &s.freq {
        assert_eq!(days, &vec![MonthDay::Day { value: 1 }, MonthDay::Last]);
    } else {
        panic!("wrong freq");
    }
}

// ---- validate ----

#[test]
fn validate_rejects_and_dedupes() {
    let mut bad_minute = Schedule {
        freq: Frequency::Hourly { minute: 60 },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    assert_eq!(bad_minute.validate(), Err(ScheduleError::InvalidMinute(60)));

    let mut dup_days = Schedule {
        freq: Frequency::Weekly {
            days: vec![Weekday::Mon, Weekday::Mon, Weekday::Wed],
            time: hm(9, 0),
        },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: None,
        end: None,
    };
    assert!(dup_days.validate().is_ok());
    if let Frequency::Weekly { days, .. } = &dup_days.freq {
        assert_eq!(days, &vec![Weekday::Mon, Weekday::Wed]);
    }

    let mut bad_bounds = Schedule {
        freq: Frequency::Daily { time: hm(9, 0) },
        timezone: Tz::UTC,
        overlays: vec![],
        makeup: Makeup::None,
        max_makeup_hops: None,
        start: Some(utc("2026-02-01T00:00:00Z")),
        end: Some(utc("2026-01-01T00:00:00Z")),
    };
    assert_eq!(bad_bounds.validate(), Err(ScheduleError::StartNotBeforeEnd));
}

// ---- finance-dates-backed calendars (spec §13 verification) ----

#[cfg(feature = "calendars")]
#[test]
fn default_calendars_coverage() {
    use crate::calendar::CalendarProvider;
    use crate::DefaultCalendars;

    let cal = DefaultCalendars::new();
    let holds = |id, d| cal.contains(id, d).unwrap();

    // NYSE closes on Good Friday (2026-04-03) — a market holiday that is NOT a
    // US federal holiday.
    assert!(holds(CalendarId::NyseHoliday, date(2026, 4, 3)));
    assert!(!holds(CalendarId::UsFederalHoliday, date(2026, 4, 3)));

    // Columbus Day (2026-10-12) is a federal holiday but NYSE stays open.
    assert!(holds(CalendarId::UsFederalHoliday, date(2026, 10, 12)));
    assert!(!holds(CalendarId::NyseHoliday, date(2026, 10, 12)));

    // Juneteenth first observed by NYSE in 2022 (2022-06-20, observed Monday).
    assert!(holds(CalendarId::NyseHoliday, date(2022, 6, 20)));

    // Shared holidays.
    assert!(holds(CalendarId::NyseHoliday, date(2026, 1, 1)));
    assert!(holds(CalendarId::UsFederalHoliday, date(2026, 1, 1)));

    // Trading/business-day predicates exclude weekends.
    assert!(!holds(CalendarId::NyseTradingDay, date(2026, 1, 3))); // Saturday
    assert!(holds(CalendarId::NyseTradingDay, date(2026, 1, 2))); // Friday, open
    assert!(!holds(CalendarId::UsBusinessDay, date(2026, 10, 12))); // Columbus Day
}
