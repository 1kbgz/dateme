//! `dateme` — a recurrence / scheduling engine.
//!
//! Pure datetime math: given a [`Schedule`] (a frequency in an IANA timezone,
//! plus optional calendar overlays, a makeup strategy, and start/end bounds),
//! compute the instants a recurring event fires:
//!
//! - [`Schedule::next`] / [`Schedule::previous`] — the single next/previous
//!   occurrence.
//! - [`Schedule::until`] / [`Schedule::since`] — the ascending/descending series
//!   between two instants.
//! - [`Schedule::upcoming`] — the next `n` occurrences.
//!
//! Calendars are abstracted behind [`CalendarProvider`] so the engine is unit
//! testable with fakes; the `calendars` feature supplies real US-federal / NYSE
//! data via `finance-dates`.

mod calendar;
mod engine;
mod schedule;

pub use calendar::{Calendar, CalendarProvider, NoCalendars};
pub use schedule::{
    CalendarId, Frequency, Makeup, MakeupFailure, MonthDay, Nth, NthWeekday, Overlay, OverlayRule,
    Schedule, ScheduleError,
};

#[cfg(feature = "calendars")]
pub use calendar::DefaultCalendars;

#[cfg(test)]
mod tests;
