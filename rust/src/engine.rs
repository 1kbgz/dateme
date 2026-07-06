//! Occurrence generation: base enumeration, timezone resolution, overlays,
//! makeup, dedup, and the public query methods (`next`, `previous`, `until`,
//! `since`, `upcoming`).

use crate::calendar::CalendarProvider;
use crate::schedule::{Frequency, Makeup, MonthDay, Nth, Schedule};
use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc, Weekday,
};
use chrono_tz::Tz;
use std::collections::BTreeSet;

/// Maximum days a makeup search scans before giving up (spec §8).
const MAX_MAKEUP_DAYS: i64 = 14;
/// Extra local-date margin so a base occurrence just outside a window whose
/// makeup lands inside is still generated. Any makeup moves at most
/// `MAX_MAKEUP_DAYS`, so this margin makes each window's `< upper` result set
/// complete.
const WINDOW_MARGIN_DAYS: i64 = MAX_MAKEUP_DAYS + 1;
/// Initial expansion horizon for unbounded forward/backward searches.
const INITIAL_HORIZON_DAYS: i64 = 90;
/// Absolute cap on how far `next`/`previous`/`upcoming` scan before giving up
/// (guards against schedules that never fire). ~50 years covers far-future
/// starts and yearly schedules.
const ABSOLUTE_HORIZON_DAYS: i64 = 366 * 50;

/// How to resolve a local wall-clock that does not exist (DST spring-forward
/// gap).
#[derive(Clone, Copy)]
enum Nonexistent {
    /// Skip the occurrence (hourly: the missing hour is simply absent).
    Skip,
    /// Shift to the first valid instant at/after the gap.
    Shift,
}

/// Resolve a local date+time in `tz` to a UTC instant, handling DST per spec §6:
/// ambiguous (fall-back) → earliest valid instant; nonexistent (spring-forward)
/// → per `mode`.
fn resolve_local_to_utc(
    tz: Tz,
    date: NaiveDate,
    time: NaiveTime,
    mode: Nonexistent,
) -> Option<DateTime<Utc>> {
    let naive = date.and_time(time);
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(dt) => Some(dt.with_timezone(&Utc)),
        LocalResult::Ambiguous(earliest, _latest) => Some(earliest.with_timezone(&Utc)),
        LocalResult::None => match mode {
            Nonexistent::Skip => None,
            Nonexistent::Shift => {
                // Step forward one minute at a time to the first valid local time
                // at/after the gap. Minute granularity matches `NaiveTime` here,
                // so this lands exactly on the gap's far edge (e.g. 02:20 -> 03:00)
                // rather than overshooting. Cap at 6h to bound pathological zones.
                let mut t = naive;
                for _ in 0..(6 * 60) {
                    t += Duration::minutes(1);
                    match tz.from_local_datetime(&t) {
                        LocalResult::Single(dt) => return Some(dt.with_timezone(&Utc)),
                        LocalResult::Ambiguous(dt, _) => return Some(dt.with_timezone(&Utc)),
                        LocalResult::None => continue,
                    }
                }
                None
            }
        },
    }
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .expect("valid month")
}

fn days_in_month(year: i32, month: u32) -> u32 {
    last_day_of_month(year, month).day()
}

/// Concrete date for a [`MonthDay`] within `(year, month)`, or `None` if the day
/// does not exist (e.g. day 31 in February) or `month` is out of range. Guarding
/// `month` here keeps an unvalidated `Yearly { month }` from panicking.
fn month_day_date(year: i32, month: u32, md: MonthDay) -> Option<NaiveDate> {
    if !(1..=12).contains(&month) {
        return None;
    }
    match md {
        MonthDay::Day { value } => {
            let v = value as u32;
            if v >= 1 && v <= days_in_month(year, month) {
                NaiveDate::from_ymd_opt(year, month, v)
            } else {
                None
            }
        }
        MonthDay::Last => Some(last_day_of_month(year, month)),
    }
}

fn nth_of(nth: Nth) -> Option<u32> {
    match nth {
        Nth::First => Some(1),
        Nth::Second => Some(2),
        Nth::Third => Some(3),
        Nth::Fourth => Some(4),
        Nth::Fifth => Some(5),
        Nth::Last => None,
    }
}

/// Date of the `nth` `weekday` in `(year, month)`, or `None` if it does not
/// exist (e.g. a 5th Friday in a 4-Friday month).
fn nth_weekday_date(year: i32, month: u32, weekday: Weekday, nth: Nth) -> Option<NaiveDate> {
    match nth_of(nth) {
        Some(n) => {
            let first = NaiveDate::from_ymd_opt(year, month, 1)?;
            let offset = (7 + weekday.num_days_from_monday() as i64
                - first.weekday().num_days_from_monday() as i64)
                % 7;
            let day = 1 + offset + 7 * (n as i64 - 1);
            if day <= days_in_month(year, month) as i64 {
                NaiveDate::from_ymd_opt(year, month, day as u32)
            } else {
                None
            }
        }
        None => {
            let last = last_day_of_month(year, month);
            let back = (7 + last.weekday().num_days_from_monday() as i64
                - weekday.num_days_from_monday() as i64)
                % 7;
            last.checked_sub_signed(Duration::days(back))
        }
    }
}

/// Iterate `(year, month)` pairs from `lo` through `hi` inclusive.
fn months_between(lo: NaiveDate, hi: NaiveDate) -> Vec<(i32, u32)> {
    let mut out = Vec::new();
    let (mut y, mut m) = (lo.year(), lo.month());
    let (ey, em) = (hi.year(), hi.month());
    while (y, m) <= (ey, em) {
        out.push((y, m));
        if m == 12 {
            y += 1;
            m = 1;
        } else {
            m += 1;
        }
    }
    out
}

impl Schedule {
    /// Base occurrences (before overlays/makeup) whose local date lies in
    /// `[lo, hi]`, as `(local date, local time-of-day)` pairs.
    fn enumerate_base(&self, lo: NaiveDate, hi: NaiveDate) -> Vec<(NaiveDate, NaiveTime)> {
        let mut out = Vec::new();
        match &self.freq {
            Frequency::Hourly { minute } => {
                let mut d = lo;
                while d <= hi {
                    for hour in 0..24u32 {
                        if let Some(t) = NaiveTime::from_hms_opt(hour, *minute as u32, 0) {
                            out.push((d, t));
                        }
                    }
                    match d.succ_opt() {
                        Some(n) => d = n,
                        None => break,
                    }
                }
            }
            Frequency::Daily { time } => {
                let mut d = lo;
                while d <= hi {
                    out.push((d, *time));
                    match d.succ_opt() {
                        Some(n) => d = n,
                        None => break,
                    }
                }
            }
            Frequency::Weekly { days, time } => {
                let mut d = lo;
                while d <= hi {
                    if days.contains(&d.weekday()) {
                        out.push((d, *time));
                    }
                    match d.succ_opt() {
                        Some(n) => d = n,
                        None => break,
                    }
                }
            }
            Frequency::MonthlyByDay { days, time } => {
                for (y, m) in months_between(lo, hi) {
                    for md in days {
                        if let Some(date) = month_day_date(y, m, *md) {
                            if date >= lo && date <= hi {
                                out.push((date, *time));
                            }
                        }
                    }
                }
            }
            Frequency::MonthlyByWeekday { weekdays, time } => {
                for (y, m) in months_between(lo, hi) {
                    for nw in weekdays {
                        if let Some(date) = nth_weekday_date(y, m, nw.weekday, nw.nth) {
                            if date >= lo && date <= hi {
                                out.push((date, *time));
                            }
                        }
                    }
                }
            }
            Frequency::Yearly { month, day, time } => {
                for year in lo.year()..=hi.year() {
                    if let Some(date) = month_day_date(year, *month as u32, *day) {
                        if date >= lo && date <= hi {
                            out.push((date, *time));
                        }
                    }
                }
            }
        }
        out
    }

    /// Whether a local `date` passes all overlays.
    fn survives(&self, date: NaiveDate, cal: &dyn CalendarProvider) -> bool {
        use crate::schedule::OverlayRule;
        for ov in &self.overlays {
            let in_set = cal.contains(ov.calendar, date).unwrap_or(false);
            let pass = match ov.rule {
                OverlayRule::Exclude => !in_set,
                OverlayRule::Only => in_set,
            };
            if !pass {
                return false;
            }
        }
        true
    }

    /// Apply the makeup rule to a dropped base `date`; returns the surviving
    /// destination date, or `None` if makeup is disabled or exhausted.
    fn make_up(&self, date: NaiveDate, cal: &dyn CalendarProvider) -> Option<NaiveDate> {
        let step = match self.makeup {
            Makeup::None => return None,
            Makeup::Before => -1,
            Makeup::After => 1,
        };
        for k in 1..=MAX_MAKEUP_DAYS {
            let d = date.checked_add_signed(Duration::days(step * k))?;
            if self.survives(d, cal) {
                return Some(d);
            }
        }
        None
    }

    /// Generate all surviving occurrences (overlays + makeup + dedup applied)
    /// whose *base* local date lies in `[lo, hi]`, deduped by exact UTC instant.
    fn generate(
        &self,
        lo: NaiveDate,
        hi: NaiveDate,
        cal: &dyn CalendarProvider,
    ) -> BTreeSet<DateTime<Utc>> {
        let mode = match self.freq {
            Frequency::Hourly { .. } => Nonexistent::Skip,
            _ => Nonexistent::Shift,
        };
        let mut set = BTreeSet::new();
        for (date, time) in self.enumerate_base(lo, hi) {
            let dest = if self.survives(date, cal) {
                Some(date)
            } else {
                self.make_up(date, cal)
            };
            if let Some(d) = dest {
                if let Some(inst) = resolve_local_to_utc(self.timezone, d, time, mode) {
                    // BTreeSet dedups by instant: a made-up occurrence that
                    // collides with an existing one is silently dropped (§8.2).
                    set.insert(inst);
                }
            }
        }
        set
    }

    /// Occurrences with instant strictly in `(lower, upper)`, ascending, with
    /// start/end bounds applied.
    fn collect(
        &self,
        lower: DateTime<Utc>,
        upper: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Vec<DateTime<Utc>> {
        if upper <= lower {
            return Vec::new();
        }
        let margin = Duration::days(WINDOW_MARGIN_DAYS);
        let lo_date = lower.with_timezone(&self.timezone).date_naive() - margin;
        let hi_date = upper.with_timezone(&self.timezone).date_naive() + margin;
        self.generate(lo_date, hi_date, cal)
            .into_iter()
            .filter(|t| {
                *t > lower
                    && *t < upper
                    && self.start.is_none_or(|s| *t >= s)
                    && self.end.is_none_or(|e| *t < e)
            })
            .collect()
    }

    /// Effective hard upper bound for a forward search: the earlier of `end` and
    /// the absolute horizon from `anchor`.
    fn forward_cap(&self, anchor: DateTime<Utc>) -> DateTime<Utc> {
        let cap = anchor + Duration::days(ABSOLUTE_HORIZON_DAYS);
        match self.end {
            Some(e) => e.min(cap),
            None => cap,
        }
    }

    fn backward_cap(&self, anchor: DateTime<Utc>) -> DateTime<Utc> {
        let cap = anchor - Duration::days(ABSOLUTE_HORIZON_DAYS);
        match self.start {
            Some(s) => s.max(cap),
            None => cap,
        }
    }

    /// The first occurrence strictly after `after`. `None` ⇒ the series has
    /// ended or none exists within the search horizon.
    pub fn next(&self, after: DateTime<Utc>, cal: &dyn CalendarProvider) -> Option<DateTime<Utc>> {
        self.upcoming(1, after, cal).into_iter().next()
    }

    /// The last occurrence strictly before `before`. `None` ⇒ none exists within
    /// the search horizon.
    pub fn previous(
        &self,
        before: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Option<DateTime<Utc>> {
        let cap = self.backward_cap(before);
        let mut span = INITIAL_HORIZON_DAYS;
        loop {
            let lower = (before - Duration::days(span)).max(cap);
            let occ = self.collect(lower, before, cal);
            if let Some(last) = occ.last() {
                return Some(*last);
            }
            if lower <= cap {
                return None;
            }
            span = (span * 2).min(ABSOLUTE_HORIZON_DAYS);
        }
    }

    /// Up to `n` occurrences strictly after `after`, ascending, deduped.
    pub fn upcoming(
        &self,
        n: usize,
        after: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Vec<DateTime<Utc>> {
        if n == 0 {
            return Vec::new();
        }
        let cap = self.forward_cap(after);
        let mut span = INITIAL_HORIZON_DAYS;
        loop {
            let upper = (after + Duration::days(span)).min(cap);
            let occ = self.collect(after, upper, cal);
            if occ.len() >= n {
                return occ.into_iter().take(n).collect();
            }
            if upper >= cap {
                return occ;
            }
            span = (span * 2).min(ABSOLUTE_HORIZON_DAYS);
        }
    }

    /// All occurrences strictly in `(after, before)`, ascending. `until(end)[0]`
    /// equals `next(after)`.
    pub fn until(
        &self,
        before: DateTime<Utc>,
        after: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Vec<DateTime<Utc>> {
        self.collect(after, before, cal)
    }

    /// All occurrences strictly in `(after, before)`, **descending**.
    /// `since(start)[0]` equals `previous(before)`.
    pub fn since(
        &self,
        after: DateTime<Utc>,
        before: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Vec<DateTime<Utc>> {
        let mut v = self.collect(after, before, cal);
        v.reverse();
        v
    }
}
