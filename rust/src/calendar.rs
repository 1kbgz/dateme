//! Calendar abstraction. The engine is generic over date sets so it can be unit
//! tested with hand-built fakes and decoupled from any concrete holiday source.

use crate::schedule::CalendarId;
use chrono::NaiveDate;

/// A set of dates. `contains(d)` = "`d` is in this calendar's set".
pub trait Calendar {
    fn contains(&self, date: NaiveDate) -> bool;
}

/// Resolves a [`CalendarId`] to a concrete [`Calendar`]. The engine consults a
/// provider only when a schedule has overlays; empty-overlay schedules never
/// touch it, so [`NoCalendars`] is a valid provider in that case.
pub trait CalendarProvider {
    /// Whether `date` is in the set for `id`. Returns `None` if this provider
    /// does not supply `id` (the occurrence is then treated as not-in-set).
    fn contains(&self, id: CalendarId, date: NaiveDate) -> Option<bool>;
}

/// A provider that supplies no calendars. Valid only for schedules with no
/// overlays; using it with overlays makes every calendar test resolve to
/// "not in set".
pub struct NoCalendars;

impl CalendarProvider for NoCalendars {
    fn contains(&self, _id: CalendarId, _date: NaiveDate) -> Option<bool> {
        None
    }
}

/// A closure-backed provider, convenient for tests and ad-hoc calendars.
impl<F> CalendarProvider for F
where
    F: Fn(CalendarId, NaiveDate) -> Option<bool>,
{
    fn contains(&self, id: CalendarId, date: NaiveDate) -> Option<bool> {
        self(id, date)
    }
}

#[cfg(feature = "calendars")]
pub use finance::DefaultCalendars;

#[cfg(feature = "calendars")]
mod finance {
    use super::CalendarProvider;
    use crate::schedule::CalendarId;
    use chrono::{NaiveDate, Weekday};
    use finance_dates::holiday::WeekendRoll;
    use finance_dates::{calendar_for_exchange, Calendar, HolidayRule, STANDARD_WEEKMASK};

    /// [`CalendarProvider`] backed by the `finance-dates` crate: real US-federal
    /// and NYSE holiday data. Construct once and reuse — building the calendars
    /// is not free.
    pub struct DefaultCalendars {
        nyse: Calendar,
        us_federal: Calendar,
    }

    impl DefaultCalendars {
        pub fn new() -> Self {
            DefaultCalendars {
                nyse: calendar_for_exchange("XNYS").expect("finance-dates ships the NYSE calendar"),
                us_federal: us_federal_calendar(),
            }
        }
    }

    impl Default for DefaultCalendars {
        fn default() -> Self {
            Self::new()
        }
    }

    impl CalendarProvider for DefaultCalendars {
        fn contains(&self, id: CalendarId, date: NaiveDate) -> Option<bool> {
            Some(match id {
                CalendarId::UsFederalHoliday => self.us_federal.is_holiday(date),
                CalendarId::UsBusinessDay => self.us_federal.is_business_day(date),
                CalendarId::NyseHoliday => self.nyse.is_holiday(date),
                CalendarId::NyseTradingDay => self.nyse.is_business_day(date),
            })
        }
    }

    fn fixed(month: u32, day: u32, since: Option<i32>) -> HolidayRule {
        HolidayRule::Fixed {
            month,
            day,
            roll: WeekendRoll::NearestWeekday,
            since_year: since,
        }
    }

    fn nth(month: u32, weekday: Weekday, n: i32) -> HolidayRule {
        HolidayRule::NthWeekday {
            month,
            weekday,
            n,
            since_year: None,
        }
    }

    /// The 11 US federal holidays (observed). `finance-dates` has no dedicated
    /// federal calendar, so we build one from its holiday-rule primitives.
    fn us_federal_calendar() -> Calendar {
        let rules = vec![
            fixed(1, 1, None),        // New Year's Day
            nth(1, Weekday::Mon, 3),  // Martin Luther King Jr. Day
            nth(2, Weekday::Mon, 3),  // Washington's Birthday
            nth(5, Weekday::Mon, -1), // Memorial Day
            fixed(6, 19, Some(2021)), // Juneteenth
            fixed(7, 4, None),        // Independence Day
            nth(9, Weekday::Mon, 1),  // Labor Day
            nth(10, Weekday::Mon, 2), // Columbus Day
            fixed(11, 11, None),      // Veterans Day
            nth(11, Weekday::Thu, 4), // Thanksgiving
            fixed(12, 25, None),      // Christmas Day
        ];
        Calendar::new("US_FEDERAL", STANDARD_WEEKMASK, rules, None)
    }
}
