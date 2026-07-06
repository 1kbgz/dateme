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

/// Normalize a Python spec into a JSON string. Accepts a JSON ``str``, a
/// ``dict`` (or any JSON-serializable object), or any object exposing a
/// ``to_dict()`` method (e.g. the ``dateme.model`` builders).
fn spec_to_json(spec: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = spec.extract::<String>() {
        return Ok(s);
    }
    let value = if spec.hasattr("to_dict")? {
        spec.call_method0("to_dict")?
    } else {
        spec.clone()
    };
    let json = spec.py().import("json")?;
    json.call_method1("dumps", (value,))?.extract()
}

impl Schedule {
    /// Parse and validate a JSON schedule, returning a ready-to-query engine.
    fn build(json: &str) -> PyResult<Self> {
        let mut inner: BaseSchedule =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Schedule {
            inner,
            calendars: DefaultCalendars::new(),
        })
    }
}

#[pymethods]
impl Schedule {
    /// Build a schedule from a JSON string, a ``dict``, or a typed
    /// ``dateme.model`` builder (any object with a ``to_dict()`` method).
    ///
    /// The schedule is validated on construction; an invalid schedule raises
    /// ``ValueError``.
    #[new]
    fn new(spec: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json = spec_to_json(spec)?;
        Self::build(&json)
    }

    /// Build a schedule from its JSON representation.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        Self::build(json)
    }

    /// Build a schedule from a ``dict`` or typed ``dateme.model`` builder.
    #[staticmethod]
    fn from_dict(spec: &Bound<'_, PyAny>) -> PyResult<Self> {
        Self::new(spec)
    }

    /// Serialize back to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Serialize back to a ``dict``.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let json = py.import("json")?;
        json.call_method1("loads", (self.to_json()?,))
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
