//! Occurrence generation: base enumeration, timezone resolution, overlays,
//! makeup, dedup, and the public query methods (`next`, `previous`, `until`,
//! `since`, `upcoming`).

use crate::calendar::CalendarProvider;
use crate::schedule::{
    Frequency, Makeup, MakeupDirection, MakeupFailure, MonthDay, Nth, Overlay, OverlayRule,
    Schedule,
};
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

enum MakeupOutcome {
    Moved(NaiveDate),
    Failed,
    Disabled,
}

/// Runtime query error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError {
    MakeupFailed { date: NaiveDate },
    MaxSkipGapExceeded { max_days: u32 },
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::MakeupFailed { date } => {
                write!(f, "makeup failed for excluded occurrence on {date}")
            }
            QueryError::MaxSkipGapExceeded { max_days } => {
                write!(f, "schedule gap exceeded max_skip_gap of {max_days} days")
            }
        }
    }
}

impl std::error::Error for QueryError {}

#[derive(Clone)]
struct OverlayOutcome {
    passes: bool,
    makeup: Option<Makeup>,
}

enum GapCheck {
    OpenHead,
    OpenTail,
    Closed,
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

    fn overlay_outcome(
        overlay: &Overlay,
        date: NaiveDate,
        cal: &dyn CalendarProvider,
    ) -> OverlayOutcome {
        match overlay {
            Overlay::Calendar(overlay) => {
                let in_set = cal.contains(overlay.calendar, date).unwrap_or(false);
                let passes = match overlay.rule {
                    OverlayRule::Exclude => !in_set,
                    OverlayRule::Only => in_set,
                };
                OverlayOutcome {
                    passes,
                    makeup: if passes { None } else { overlay.makeup.clone() },
                }
            }
            Overlay::Any(group) => {
                let mut first_makeup = None;
                for child in &group.any {
                    let outcome = Self::overlay_outcome(child, date, cal);
                    if outcome.passes {
                        return OverlayOutcome {
                            passes: true,
                            makeup: None,
                        };
                    }
                    if first_makeup.is_none() {
                        first_makeup = outcome.makeup;
                    }
                }
                OverlayOutcome {
                    passes: false,
                    makeup: group.makeup.clone().or(first_makeup),
                }
            }
        }
    }

    /// Whether a local `date` passes all overlays. Returns the first failing
    /// overlay's makeup override, if one is configured.
    fn overlay_result(&self, date: NaiveDate, cal: &dyn CalendarProvider) -> OverlayOutcome {
        for overlay in &self.overlays {
            let outcome = Self::overlay_outcome(overlay, date, cal);
            if !outcome.passes {
                return outcome;
            }
        }
        OverlayOutcome {
            passes: true,
            makeup: None,
        }
    }

    /// Whether a local `date` passes all overlays.
    fn survives(&self, date: NaiveDate, cal: &dyn CalendarProvider) -> bool {
        self.overlay_result(date, cal).passes
    }

    fn skipped_excluded_runs(
        &self,
        base: &[(NaiveDate, NaiveTime)],
        cal: &dyn CalendarProvider,
    ) -> BTreeSet<(NaiveDate, NaiveTime)> {
        let threshold = match self.skip_if_consecutive_excluded {
            Some(n) if n > 0 => n as usize,
            _ => return BTreeSet::new(),
        };
        let mut skipped = BTreeSet::new();
        let mut run = Vec::new();

        for &(date, time) in base {
            if self.survives(date, cal) {
                if run.len() >= threshold {
                    skipped.extend(run.drain(..));
                } else {
                    run.clear();
                }
            } else {
                run.push((date, time));
            }
        }
        if run.len() >= threshold {
            skipped.extend(run);
        }

        skipped
    }

    fn valid_makeup_target(
        &self,
        original: NaiveDate,
        candidate: NaiveDate,
        previous_base: Option<NaiveDate>,
        next_base: Option<NaiveDate>,
        cal: &dyn CalendarProvider,
    ) -> bool {
        if self.makeup_within_week && candidate.iso_week() != original.iso_week() {
            return false;
        }
        if self.makeup_exclude_weekends
            && matches!(candidate.weekday(), Weekday::Sat | Weekday::Sun)
        {
            return false;
        }
        if self.makeup_before_next {
            if next_base.is_some_and(|next| original < next && candidate >= next) {
                return false;
            }
            if previous_base.is_some_and(|previous| previous < original && candidate <= previous) {
                return false;
            }
        }

        self.survives(candidate, cal)
            && self
                .makeup_only_on
                .as_ref()
                .is_none_or(|days| days.contains(&candidate.weekday()))
    }

    /// Apply the makeup rule to a dropped base `date`.
    fn make_up(
        &self,
        date: NaiveDate,
        previous_base: Option<NaiveDate>,
        next_base: Option<NaiveDate>,
        makeup: Option<&Makeup>,
        cal: &dyn CalendarProvider,
    ) -> MakeupOutcome {
        let default_max_hops = self
            .max_makeup_hops
            .map(i64::from)
            .unwrap_or(MAX_MAKEUP_DAYS)
            .min(MAX_MAKEUP_DAYS);
        let mut attempted = false;
        let makeup = makeup.unwrap_or(&self.makeup);
        for makeup_step in makeup.steps_for(date.weekday()) {
            let (direction, max_hops) = makeup_step.parts();
            let day_step = match direction {
                MakeupDirection::None => return MakeupOutcome::Disabled,
                MakeupDirection::Before => -1,
                MakeupDirection::After => 1,
                MakeupDirection::Nearest => 1,
            };
            let max_hops = max_hops
                .map(i64::from)
                .unwrap_or(default_max_hops)
                .min(default_max_hops);
            if max_hops == 0 {
                attempted = true;
                continue;
            }
            attempted = true;
            for k in 1..=max_hops {
                if matches!(direction, MakeupDirection::Nearest) {
                    for step in [1, -1] {
                        let Some(d) = date.checked_add_signed(Duration::days(step * k)) else {
                            continue;
                        };
                        if self.valid_makeup_target(date, d, previous_base, next_base, cal) {
                            return MakeupOutcome::Moved(d);
                        }
                    }
                    continue;
                }
                let Some(d) = date.checked_add_signed(Duration::days(day_step * k)) else {
                    break;
                };
                if self.valid_makeup_target(date, d, previous_base, next_base, cal) {
                    return MakeupOutcome::Moved(d);
                }
            }
        }
        if attempted {
            MakeupOutcome::Failed
        } else {
            MakeupOutcome::Disabled
        }
    }

    /// Generate all surviving occurrences (overlays + makeup + dedup applied)
    /// whose *base* local date lies in `[lo, hi]`, deduped by exact UTC instant.
    fn generate(
        &self,
        lo: NaiveDate,
        hi: NaiveDate,
        cal: &dyn CalendarProvider,
    ) -> Result<BTreeSet<DateTime<Utc>>, QueryError> {
        let mode = match self.freq {
            Frequency::Hourly { .. } => Nonexistent::Skip,
            _ => Nonexistent::Shift,
        };
        let mut set = BTreeSet::new();
        let mut base = self.enumerate_base(lo, hi);
        base.sort_unstable();
        let skipped_base = self.skipped_excluded_runs(&base, cal);
        for (index, (date, time)) in base.iter().copied().enumerate() {
            if skipped_base.contains(&(date, time)) {
                continue;
            }
            let overlay = self.overlay_result(date, cal);
            let dest = if overlay.passes {
                Some(date)
            } else {
                let previous_base = base[..index].last().map(|(d, _)| *d);
                let next_base = base[index + 1..].first().map(|(d, _)| *d);
                match self.make_up(date, previous_base, next_base, overlay.makeup.as_ref(), cal) {
                    MakeupOutcome::Moved(d) => Some(d),
                    MakeupOutcome::Failed => match self.makeup_failure {
                        MakeupFailure::Skip => None,
                        MakeupFailure::KeepOriginal => Some(date),
                        MakeupFailure::Error => return Err(QueryError::MakeupFailed { date }),
                    },
                    MakeupOutcome::Disabled => None,
                }
            };
            if let Some(d) = dest {
                if let Some(inst) = resolve_local_to_utc(self.timezone, d, time, mode) {
                    // BTreeSet dedups by instant: a made-up occurrence that
                    // collides with an existing one is silently dropped (§8.2).
                    set.insert(inst);
                }
            }
        }
        Ok(set)
    }

    fn check_max_skip_gap(
        &self,
        lower: DateTime<Utc>,
        upper: DateTime<Utc>,
        occurrences: &[DateTime<Utc>],
        gap_check: GapCheck,
    ) -> Result<(), QueryError> {
        let Some(max_days) = self.max_skip_gap else {
            return Ok(());
        };
        let max_gap = Duration::days(i64::from(max_days));
        let mut previous = if matches!(gap_check, GapCheck::OpenHead) {
            occurrences.first().copied().unwrap_or(lower)
        } else {
            lower
        };
        for occurrence in occurrences {
            if *occurrence - previous > max_gap {
                return Err(QueryError::MaxSkipGapExceeded { max_days });
            }
            previous = *occurrence;
        }
        if matches!(gap_check, GapCheck::Closed | GapCheck::OpenHead) && upper - previous > max_gap
        {
            return Err(QueryError::MaxSkipGapExceeded { max_days });
        }
        Ok(())
    }

    /// Occurrences with instant strictly in `(lower, upper)`, ascending, with
    /// start/end bounds applied.
    fn try_collect(
        &self,
        lower: DateTime<Utc>,
        upper: DateTime<Utc>,
        gap_check: GapCheck,
        cal: &dyn CalendarProvider,
    ) -> Result<Vec<DateTime<Utc>>, QueryError> {
        if upper <= lower {
            return Ok(Vec::new());
        }
        let margin = Duration::days(WINDOW_MARGIN_DAYS);
        let lo_date = lower.with_timezone(&self.timezone).date_naive() - margin;
        let hi_date = upper.with_timezone(&self.timezone).date_naive() + margin;
        let occurrences: Vec<_> = self
            .generate(lo_date, hi_date, cal)?
            .into_iter()
            .filter(|t| {
                *t > lower
                    && *t < upper
                    && self.start.is_none_or(|s| *t >= s)
                    && self.end.is_none_or(|e| *t < e)
            })
            .collect();
        self.check_max_skip_gap(lower, upper, &occurrences, gap_check)?;
        Ok(occurrences)
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
        self.try_next(after, cal).ok().flatten()
    }

    /// Fallible variant of [`Schedule::next`].
    pub fn try_next(
        &self,
        after: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Result<Option<DateTime<Utc>>, QueryError> {
        Ok(self.try_upcoming(1, after, cal)?.into_iter().next())
    }

    /// The last occurrence strictly before `before`. `None` ⇒ none exists within
    /// the search horizon.
    pub fn previous(
        &self,
        before: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Option<DateTime<Utc>> {
        self.try_previous(before, cal).ok().flatten()
    }

    /// Fallible variant of [`Schedule::previous`].
    pub fn try_previous(
        &self,
        before: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Result<Option<DateTime<Utc>>, QueryError> {
        let cap = self.backward_cap(before);
        let mut span = INITIAL_HORIZON_DAYS;
        loop {
            let lower = (before - Duration::days(span)).max(cap);
            let occ = self.try_collect(lower, before, GapCheck::OpenHead, cal)?;
            if let Some(last) = occ.last() {
                return Ok(Some(*last));
            }
            if lower <= cap {
                return Ok(None);
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
        self.try_upcoming(n, after, cal).unwrap_or_default()
    }

    /// Fallible variant of [`Schedule::upcoming`].
    pub fn try_upcoming(
        &self,
        n: usize,
        after: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Result<Vec<DateTime<Utc>>, QueryError> {
        if n == 0 {
            return Ok(Vec::new());
        }
        let cap = self.forward_cap(after);
        let mut span = INITIAL_HORIZON_DAYS;
        loop {
            let upper = (after + Duration::days(span)).min(cap);
            let occ = self.try_collect(after, upper, GapCheck::OpenTail, cal)?;
            if occ.len() >= n {
                return Ok(occ.into_iter().take(n).collect());
            }
            if upper >= cap {
                return Ok(occ);
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
        self.try_until(before, after, cal).unwrap_or_default()
    }

    /// Fallible variant of [`Schedule::until`].
    pub fn try_until(
        &self,
        before: DateTime<Utc>,
        after: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Result<Vec<DateTime<Utc>>, QueryError> {
        self.try_collect(after, before, GapCheck::Closed, cal)
    }

    /// All occurrences strictly in `(after, before)`, **descending**.
    /// `since(start)[0]` equals `previous(before)`.
    pub fn since(
        &self,
        after: DateTime<Utc>,
        before: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Vec<DateTime<Utc>> {
        self.try_since(after, before, cal).unwrap_or_default()
    }

    /// Fallible variant of [`Schedule::since`].
    pub fn try_since(
        &self,
        after: DateTime<Utc>,
        before: DateTime<Utc>,
        cal: &dyn CalendarProvider,
    ) -> Result<Vec<DateTime<Utc>>, QueryError> {
        let mut v = self.try_collect(after, before, GapCheck::Closed, cal)?;
        v.reverse();
        Ok(v)
    }
}
