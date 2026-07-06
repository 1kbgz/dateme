//! Python bindings for the `dateme` recurrence engine (pyo3, abi3).
//!
//! A `Schedule` is built from its JSON form (see the crate spec) and exposes the
//! occurrence queries as methods taking/returning timezone-aware `datetime`s.
//! When a reference instant is omitted it defaults to "now" (UTC).

use chrono::{DateTime, Utc};
use dateme_core::{DefaultCalendars, Schedule as BaseSchedule};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// A recurrence schedule built from its JSON representation.
///
/// A schedule is a frequency in an IANA timezone, plus optional calendar
/// overlays, a makeup strategy, and start/end bounds. Its methods compute the
/// instants the schedule fires. Reference instants are timezone-aware
/// ``datetime`` objects; when omitted they default to the current time (UTC).
#[pyclass(name = "Schedule")]
pub struct Schedule {
    inner: BaseSchedule,
    calendars: DefaultCalendars,
}

#[pymethods]
impl Schedule {
    /// Build a schedule from its JSON representation.
    #[new]
    fn new(json: &str) -> PyResult<Self> {
        let inner: BaseSchedule =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Schedule {
            inner,
            calendars: DefaultCalendars::new(),
        })
    }

    /// Build a schedule from its JSON representation (alias for the constructor).
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Self::new(json)
    }

    /// Serialize back to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Structural validation. Raises `ValueError` on an invalid schedule.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .clone()
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// First occurrence strictly after `after` (default: now).
    #[pyo3(signature = (after=None))]
    fn next(&self, after: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
        self.inner
            .next(after.unwrap_or_else(Utc::now), &self.calendars)
    }

    /// Last occurrence strictly before `before` (default: now).
    #[pyo3(signature = (before=None))]
    fn previous(&self, before: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
        self.inner
            .previous(before.unwrap_or_else(Utc::now), &self.calendars)
    }

    /// Occurrences in `(after, before)`, ascending. `until(end)[0]` == `next()`.
    #[pyo3(signature = (before, after=None))]
    fn until(&self, before: DateTime<Utc>, after: Option<DateTime<Utc>>) -> Vec<DateTime<Utc>> {
        self.inner
            .until(before, after.unwrap_or_else(Utc::now), &self.calendars)
    }

    /// Occurrences in `(after, before)`, descending. `since(start)[0]` == `previous()`.
    #[pyo3(signature = (after, before=None))]
    fn since(&self, after: DateTime<Utc>, before: Option<DateTime<Utc>>) -> Vec<DateTime<Utc>> {
        self.inner
            .since(after, before.unwrap_or_else(Utc::now), &self.calendars)
    }

    /// The next `n` occurrences strictly after `after` (default: now), ascending.
    #[pyo3(signature = (n, after=None))]
    fn upcoming(&self, n: usize, after: Option<DateTime<Utc>>) -> Vec<DateTime<Utc>> {
        self.inner
            .upcoming(n, after.unwrap_or_else(Utc::now), &self.calendars)
    }

    fn __repr__(&self) -> String {
        format!(
            "Schedule({})",
            serde_json::to_string(&self.inner).unwrap_or_default()
        )
    }
}

#[pymodule]
fn dateme(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<Schedule>()?;
    Ok(())
}
