//! Recurrence schedule types and their JSON representation.
//!
//! A [`Schedule`] is pure datetime math: a frequency, a timezone, optional
//! calendar overlays, a makeup strategy, and optional start/end bounds. The
//! computation lives in [`crate::engine`]; this module defines the data model,
//! its serde form, and structural validation.

use chrono::{DateTime, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Makeup {
    /// Skip the cycle entirely.
    #[default]
    None,
    /// Move to the nearest EARLIER surviving day (same time-of-day).
    Before,
    /// Move to the nearest LATER surviving day (same time-of-day).
    After,
}

/// Structural validation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduleError {
    InvalidMinute(u8),
    EmptyDays,
    InvalidMonthDay(u8),
    InvalidMonth(u8),
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
