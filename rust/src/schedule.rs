//! Recurrence schedule types and their JSON representation.
//!
//! A [`Schedule`] is pure datetime math: a frequency, a timezone, optional
//! calendar overlays, a makeup strategy, and optional start/end bounds. The
//! computation lives in [`crate::engine`]; this module defines the data model,
//! its serde form, and structural validation.

use chrono::{DateTime, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A recurrence rule. Serialize to JSON for storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    pub freq: Frequency,
    #[serde(with = "tz_serde")]
    pub timezone: Tz,
    /// Calendar filters, ANDed. Empty = no filtering.
    #[serde(default)]
    pub overlays: Vec<Overlay>,
    /// What to do when an overlay drops a base occurrence.
    #[serde(default)]
    pub makeup: Makeup,
    /// Maximum number of days to scan for makeup. `None` keeps the built-in
    /// search limit.
    #[serde(default)]
    pub max_makeup_hops: Option<u32>,
    /// What to do when makeup is enabled but no surviving destination is found.
    #[serde(default)]
    pub makeup_failure: MakeupFailure,
    /// Restrict makeup destination dates to these weekdays.
    #[serde(default, with = "weekday_vec_opt")]
    pub makeup_only_on: Option<Vec<Weekday>>,
    /// Keep makeup destinations within the original ISO week.
    #[serde(default)]
    pub makeup_within_week: bool,
    /// Reject Saturday and Sunday makeup destinations.
    #[serde(default)]
    pub makeup_exclude_weekends: bool,
    /// Do not let makeup land on or cross an adjacent base occurrence.
    #[serde(default)]
    pub makeup_before_next: bool,
    /// Skip excluded base-occurrence runs at or above this length before makeup.
    #[serde(default)]
    pub skip_if_consecutive_excluded: Option<u32>,
    /// No occurrence before this instant (future-start support).
    #[serde(default)]
    pub start: Option<DateTime<Utc>>,
    /// No occurrence at/after this instant (series bound).
    #[serde(default)]
    pub end: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frequency {
    /// Every hour, at `minute` past the hour. Time-of-day is not used.
    Hourly { minute: u8 },
    /// Every day at `time`.
    Daily {
        #[serde(with = "time_hm")]
        time: NaiveTime,
    },
    /// Every selected weekday at `time`.
    Weekly {
        #[serde(with = "weekday_vec")]
        days: Vec<Weekday>,
        #[serde(with = "time_hm")]
        time: NaiveTime,
    },
    /// Selected days-of-month at `time`.
    MonthlyByDay {
        days: Vec<MonthDay>,
        #[serde(with = "time_hm")]
        time: NaiveTime,
    },
    /// Selected nth-weekdays at `time`.
    MonthlyByWeekday {
        weekdays: Vec<NthWeekday>,
        #[serde(with = "time_hm")]
        time: NaiveTime,
    },
    /// Once a year in `month` on `day` at `time`.
    Yearly {
        month: u8,
        day: MonthDay,
        #[serde(with = "time_hm")]
        time: NaiveTime,
    },
}

/// A day within a month: a fixed `1..=31`, or the last day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MonthDay {
    Day { value: u8 },
    Last,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NthWeekday {
    pub nth: Nth,
    #[serde(with = "weekday_one")]
    pub weekday: Weekday,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Nth {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Last,
}

/// A calendar filter. Overlays are ANDed: an occurrence survives only if it
/// passes every overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overlay {
    pub calendar: CalendarId,
    pub rule: OverlayRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayRule {
    /// Drop the occurrence if its local date IS in the calendar set.
    Exclude,
    /// Drop the occurrence if its local date is NOT in the calendar set.
    Only,
}

/// Built-in calendars. Concrete implementations are provided by a
/// [`crate::CalendarProvider`] (see the `calendars` feature).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarId {
    UsFederalHoliday,
    UsBusinessDay,
    NyseHoliday,
    NyseTradingDay,
}

/// What to do when an overlay drops a base occurrence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Makeup {
    /// Skip the cycle entirely.
    #[default]
    None,
    /// Move to the nearest EARLIER surviving day (same time-of-day).
    Before,
    /// Move to the nearest LATER surviving day (same time-of-day).
    After,
    /// Move to the nearest surviving day, preferring later dates on ties.
    Nearest,
    /// Select a makeup direction based on the excluded date's weekday.
    ByWeekday(WeekdayMakeup),
    /// Try makeup strategies in order until one succeeds or disables makeup.
    Cascade(Vec<MakeupStep>),
}

impl Makeup {
    pub(crate) fn steps_for(&self, weekday: Weekday) -> Vec<MakeupStep> {
        match self {
            Makeup::None => vec![MakeupStep::direction(MakeupDirection::None)],
            Makeup::Before => vec![MakeupStep::direction(MakeupDirection::Before)],
            Makeup::After => vec![MakeupStep::direction(MakeupDirection::After)],
            Makeup::Nearest => vec![MakeupStep::direction(MakeupDirection::Nearest)],
            Makeup::ByWeekday(map) => vec![MakeupStep::direction(map.direction_for(weekday))],
            Makeup::Cascade(steps) => steps.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MakeupStep {
    Direction(MakeupDirection),
    Options(MakeupStepOptions),
}

impl MakeupStep {
    fn direction(direction: MakeupDirection) -> Self {
        Self::Direction(direction)
    }

    pub(crate) fn parts(&self) -> (MakeupDirection, Option<u32>) {
        match self {
            MakeupStep::Direction(direction) => (*direction, None),
            MakeupStep::Options(options) => (options.direction, options.max_hops),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeupStepOptions {
    pub direction: MakeupDirection,
    #[serde(default)]
    pub max_hops: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeekdayMakeup {
    pub mon: Option<MakeupDirection>,
    pub tue: Option<MakeupDirection>,
    pub wed: Option<MakeupDirection>,
    pub thu: Option<MakeupDirection>,
    pub fri: Option<MakeupDirection>,
    pub sat: Option<MakeupDirection>,
    pub sun: Option<MakeupDirection>,
    pub default: Option<MakeupDirection>,
}

impl WeekdayMakeup {
    fn direction_for(&self, weekday: Weekday) -> MakeupDirection {
        let selected = match weekday {
            Weekday::Mon => self.mon,
            Weekday::Tue => self.tue,
            Weekday::Wed => self.wed,
            Weekday::Thu => self.thu,
            Weekday::Fri => self.fri,
            Weekday::Sat => self.sat,
            Weekday::Sun => self.sun,
        };
        selected.or(self.default).unwrap_or(MakeupDirection::None)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MakeupDirection {
    #[default]
    None,
    Before,
    After,
    Nearest,
}

impl From<MakeupDirection> for Makeup {
    fn from(value: MakeupDirection) -> Self {
        match value {
            MakeupDirection::None => Makeup::None,
            MakeupDirection::Before => Makeup::Before,
            MakeupDirection::After => Makeup::After,
            MakeupDirection::Nearest => Makeup::Nearest,
        }
    }
}

impl From<Makeup> for MakeupDirection {
    fn from(value: Makeup) -> Self {
        match value {
            Makeup::None | Makeup::ByWeekday(_) | Makeup::Cascade(_) => MakeupDirection::None,
            Makeup::Before => MakeupDirection::Before,
            Makeup::After => MakeupDirection::After,
            Makeup::Nearest => MakeupDirection::Nearest,
        }
    }
}

impl Serialize for Makeup {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Makeup::None => MakeupDirection::None.serialize(serializer),
            Makeup::Before => MakeupDirection::Before.serialize(serializer),
            Makeup::After => MakeupDirection::After.serialize(serializer),
            Makeup::Nearest => MakeupDirection::Nearest.serialize(serializer),
            Makeup::ByWeekday(map) => map.serialize(serializer),
            Makeup::Cascade(steps) => {
                let mut seq = serializer.serialize_seq(Some(steps.len()))?;
                for step in steps {
                    seq.serialize_element(step)?;
                }
                seq.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Makeup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MakeupVisitor;

        impl<'de> Visitor<'de> for MakeupVisitor {
            type Value = Makeup;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a makeup direction string or weekday makeup map")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let direction = match value {
                    "none" => MakeupDirection::None,
                    "before" => MakeupDirection::Before,
                    "after" => MakeupDirection::After,
                    "nearest" => MakeupDirection::Nearest,
                    _ => {
                        return Err(E::unknown_variant(
                            value,
                            &["none", "before", "after", "nearest"],
                        ))
                    }
                };
                Ok(direction.into())
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                WeekdayMakeup::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(Makeup::ByWeekday)
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                Vec::<MakeupStep>::deserialize(de::value::SeqAccessDeserializer::new(seq))
                    .map(Makeup::Cascade)
            }
        }

        deserializer.deserialize_any(MakeupVisitor)
    }
}

impl Serialize for WeekdayMakeup {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        if let Some(value) = self.mon {
            map.serialize_entry("mon", &value)?;
        }
        if let Some(value) = self.tue {
            map.serialize_entry("tue", &value)?;
        }
        if let Some(value) = self.wed {
            map.serialize_entry("wed", &value)?;
        }
        if let Some(value) = self.thu {
            map.serialize_entry("thu", &value)?;
        }
        if let Some(value) = self.fri {
            map.serialize_entry("fri", &value)?;
        }
        if let Some(value) = self.sat {
            map.serialize_entry("sat", &value)?;
        }
        if let Some(value) = self.sun {
            map.serialize_entry("sun", &value)?;
        }
        if let Some(value) = self.default {
            map.serialize_entry("default", &value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for WeekdayMakeup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Raw {
            #[serde(default)]
            mon: Option<MakeupDirection>,
            #[serde(default)]
            tue: Option<MakeupDirection>,
            #[serde(default)]
            wed: Option<MakeupDirection>,
            #[serde(default)]
            thu: Option<MakeupDirection>,
            #[serde(default)]
            fri: Option<MakeupDirection>,
            #[serde(default)]
            sat: Option<MakeupDirection>,
            #[serde(default)]
            sun: Option<MakeupDirection>,
            #[serde(default, rename = "default")]
            default_direction: Option<MakeupDirection>,
        }

        let raw = Raw::deserialize(deserializer)?;
        Ok(WeekdayMakeup {
            mon: raw.mon,
            tue: raw.tue,
            wed: raw.wed,
            thu: raw.thu,
            fri: raw.fri,
            sat: raw.sat,
            sun: raw.sun,
            default: raw.default_direction,
        })
    }
}

/// What to do when enabled makeup cannot find a surviving destination.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MakeupFailure {
    /// Drop the occurrence silently.
    #[default]
    Skip,
    /// Emit the occurrence on its original excluded date.
    KeepOriginal,
}

/// Structural validation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduleError {
    InvalidMinute(u8),
    EmptyDays,
    InvalidMonthDay(u8),
    InvalidMonth(u8),
    InvalidSkipThreshold(u32),
    StartNotBeforeEnd,
    NeverFires,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleError::InvalidMinute(m) => write!(f, "minute {m} out of range 0..=59"),
            ScheduleError::EmptyDays => write!(f, "day/weekday selection is empty"),
            ScheduleError::InvalidMonthDay(d) => write!(f, "month day {d} out of range 1..=31"),
            ScheduleError::InvalidMonth(m) => write!(f, "month {m} out of range 1..=12"),
            ScheduleError::InvalidSkipThreshold(n) => {
                write!(f, "skip threshold {n} out of range 1..")
            }
            ScheduleError::StartNotBeforeEnd => write!(f, "start must be strictly before end"),
            ScheduleError::NeverFires => write!(f, "schedule can never produce an occurrence"),
        }
    }
}

impl std::error::Error for ScheduleError {}

impl Schedule {
    /// Structural validation. Call at competition-create time. Dedupes day and
    /// weekday selections in place (see spec §11).
    pub fn validate(&mut self) -> Result<(), ScheduleError> {
        match &mut self.freq {
            Frequency::Hourly { minute } => {
                if *minute > 59 {
                    return Err(ScheduleError::InvalidMinute(*minute));
                }
            }
            Frequency::Daily { .. } => {}
            Frequency::Weekly { days, .. } => {
                dedup_weekdays(days);
                if days.is_empty() {
                    return Err(ScheduleError::EmptyDays);
                }
            }
            Frequency::MonthlyByDay { days, .. } => {
                for d in days.iter() {
                    check_month_day(d)?;
                }
                dedup(days);
                if days.is_empty() {
                    return Err(ScheduleError::EmptyDays);
                }
            }
            Frequency::MonthlyByWeekday { weekdays, .. } => {
                dedup(weekdays);
                if weekdays.is_empty() {
                    return Err(ScheduleError::EmptyDays);
                }
            }
            Frequency::Yearly { month, day, .. } => {
                if *month < 1 || *month > 12 {
                    return Err(ScheduleError::InvalidMonth(*month));
                }
                check_month_day(day)?;
            }
        }

        if let (Some(start), Some(end)) = (self.start, self.end) {
            if start >= end {
                return Err(ScheduleError::StartNotBeforeEnd);
            }
        }

        if let Some(0) = self.skip_if_consecutive_excluded {
            return Err(ScheduleError::InvalidSkipThreshold(0));
        }

        Ok(())
    }
}

fn check_month_day(d: &MonthDay) -> Result<(), ScheduleError> {
    if let MonthDay::Day { value } = d {
        if *value < 1 || *value > 31 {
            return Err(ScheduleError::InvalidMonthDay(*value));
        }
    }
    Ok(())
}

fn dedup<T: PartialEq + Clone>(v: &mut Vec<T>) {
    let mut out: Vec<T> = Vec::with_capacity(v.len());
    for x in v.iter() {
        if !out.contains(x) {
            out.push(x.clone());
        }
    }
    *v = out;
}

fn dedup_weekdays(v: &mut Vec<Weekday>) {
    let mut out: Vec<Weekday> = Vec::with_capacity(v.len());
    for x in v.iter() {
        if !out.contains(x) {
            out.push(*x);
        }
    }
    *v = out;
}

fn weekday_to_str(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    }
}

fn weekday_from_str(s: &str) -> Option<Weekday> {
    Some(match s {
        "mon" => Weekday::Mon,
        "tue" => Weekday::Tue,
        "wed" => Weekday::Wed,
        "thu" => Weekday::Thu,
        "fri" => Weekday::Fri,
        "sat" => Weekday::Sat,
        "sun" => Weekday::Sun,
        _ => return None,
    })
}

mod time_hm {
    use chrono::{NaiveTime, Timelike};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(t: &NaiveTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:02}:{:02}", t.hour(), t.minute()))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveTime, D::Error> {
        let s = String::deserialize(d)?;
        NaiveTime::parse_from_str(&s, "%H:%M").map_err(serde::de::Error::custom)
    }
}

mod weekday_one {
    use super::{weekday_from_str, weekday_to_str};
    use chrono::Weekday;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(w: &Weekday, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(weekday_to_str(*w))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Weekday, D::Error> {
        let s = String::deserialize(d)?;
        weekday_from_str(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid weekday: {s}")))
    }
}

mod weekday_vec {
    use super::{weekday_from_str, weekday_to_str};
    use chrono::Weekday;
    use serde::ser::SerializeSeq;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(days: &[Weekday], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(days.len()))?;
        for d in days {
            seq.serialize_element(weekday_to_str(*d))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Weekday>, D::Error> {
        let strs = Vec::<String>::deserialize(d)?;
        strs.iter()
            .map(|s| {
                weekday_from_str(s)
                    .ok_or_else(|| serde::de::Error::custom(format!("invalid weekday: {s}")))
            })
            .collect()
    }
}

mod weekday_vec_opt {
    use super::{weekday_from_str, weekday_vec};
    use chrono::Weekday;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(days: &Option<Vec<Weekday>>, s: S) -> Result<S::Ok, S::Error> {
        match days {
            Some(days) => weekday_vec::serialize(days, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<Weekday>>, D::Error> {
        Option::<Vec<String>>::deserialize(d)?
            .map(|strs| {
                strs.iter()
                    .map(|s| {
                        weekday_from_str(s).ok_or_else(|| {
                            serde::de::Error::custom(format!("invalid weekday: {s}"))
                        })
                    })
                    .collect()
            })
            .transpose()
    }
}

mod tz_serde {
    use chrono_tz::Tz;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;

    pub fn serialize<S: Serializer>(tz: &Tz, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(tz.name())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Tz, D::Error> {
        let s = String::deserialize(d)?;
        Tz::from_str(&s).map_err(serde::de::Error::custom)
    }
}
