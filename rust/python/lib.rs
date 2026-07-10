//! Python bindings for the `dateme` recurrence engine (pyo3, abi3).
//!
//! A `Schedule` is built from its JSON form (see the crate spec) and exposes the
//! occurrence queries as methods taking/returning timezone-aware `datetime`s.
//! When a reference instant is omitted it defaults to "now" (UTC).

use chrono::{DateTime, NaiveDate, Utc};
use dateme_core::{
    CalendarId, CalendarProvider, DefaultCalendars, OccurrenceTrace, Schedule as BaseSchedule,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// A recurrence schedule built from its JSON representation.
///
/// A schedule is a frequency in an IANA timezone, plus optional calendar
/// overlays, a makeup strategy, and start/end bounds. Its methods compute the
/// instants the schedule fires. Reference instants are timezone-aware
/// ``datetime`` objects; when omitted they default to the current time (UTC).
#[pyclass(name = "Schedule")]
pub struct Schedule {
    inner: BaseSchedule,
    calendars: PythonCalendars,
}

struct PythonCalendars {
    defaults: DefaultCalendars,
    custom: Option<Py<PyAny>>,
}

impl PythonCalendars {
    fn new(custom: Option<Py<PyAny>>) -> Self {
        Self {
            defaults: DefaultCalendars::new(),
            custom,
        }
    }
}

impl CalendarProvider for PythonCalendars {
    fn contains(&self, id: CalendarId, date: NaiveDate) -> Option<bool> {
        self.defaults.contains(id, date)
    }

    fn contains_custom(&self, name: &str, date: NaiveDate) -> Option<bool> {
        let provider = self.custom.as_ref()?;
        Python::attach(|py| {
            let provider = provider.bind(py);
            let date = date.to_string();
            let result = if provider.hasattr("contains").ok()? {
                provider.call_method1("contains", (name, date))
            } else {
                provider.call1((name, date))
            };
            result.ok()?.extract::<bool>().ok()
        })
    }
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

fn trace_to_dict<'py>(py: Python<'py>, trace: OccurrenceTrace) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    out.set_item("instant", trace.instant)?;
    out.set_item("reason", trace.reason)?;
    Ok(out)
}

impl Schedule {
    /// Parse and validate a JSON schedule, returning a ready-to-query engine.
    fn build(json: &str, calendar_provider: Option<Py<PyAny>>) -> PyResult<Self> {
        let mut inner: BaseSchedule =
            serde_json::from_str(json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        inner
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Schedule {
            inner,
            calendars: PythonCalendars::new(calendar_provider),
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
    #[pyo3(signature = (spec, calendar_provider=None))]
    fn new(spec: &Bound<'_, PyAny>, calendar_provider: Option<Py<PyAny>>) -> PyResult<Self> {
        let json = spec_to_json(spec)?;
        Self::build(&json, calendar_provider)
    }

    /// Build a schedule from its JSON representation.
    #[staticmethod]
    #[pyo3(signature = (json, calendar_provider=None))]
    fn from_json(json: &str, calendar_provider: Option<Py<PyAny>>) -> PyResult<Self> {
        Self::build(json, calendar_provider)
    }

    /// Build a schedule from a ``dict`` or typed ``dateme.model`` builder.
    #[staticmethod]
    #[pyo3(signature = (spec, calendar_provider=None))]
    fn from_dict(spec: &Bound<'_, PyAny>, calendar_provider: Option<Py<PyAny>>) -> PyResult<Self> {
        Self::new(spec, calendar_provider)
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
    fn next(&self, after: Option<DateTime<Utc>>) -> PyResult<Option<DateTime<Utc>>> {
        self.inner
            .try_next(after.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Last occurrence strictly before `before` (default: now).
    #[pyo3(signature = (before=None))]
    fn previous(&self, before: Option<DateTime<Utc>>) -> PyResult<Option<DateTime<Utc>>> {
        self.inner
            .try_previous(before.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Occurrences in `(after, before)`, ascending. `until(end)[0]` == `next()`.
    #[pyo3(signature = (before, after=None))]
    fn until(
        &self,
        before: DateTime<Utc>,
        after: Option<DateTime<Utc>>,
    ) -> PyResult<Vec<DateTime<Utc>>> {
        self.inner
            .try_until(before, after.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Occurrences in `(after, before)`, descending. `since(start)[0]` == `previous()`.
    #[pyo3(signature = (after, before=None))]
    fn since(
        &self,
        after: DateTime<Utc>,
        before: Option<DateTime<Utc>>,
    ) -> PyResult<Vec<DateTime<Utc>>> {
        self.inner
            .try_since(after, before.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// The next `n` occurrences strictly after `after` (default: now), ascending.
    #[pyo3(signature = (n, after=None))]
    fn upcoming(&self, n: usize, after: Option<DateTime<Utc>>) -> PyResult<Vec<DateTime<Utc>>> {
        self.inner
            .try_upcoming(n, after.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// First occurrence trace strictly after `after` (default: now).
    #[pyo3(signature = (after=None))]
    fn next_trace<'py>(
        &self,
        py: Python<'py>,
        after: Option<DateTime<Utc>>,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.inner
            .try_next_trace(after.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .map(|trace| trace_to_dict(py, trace))
            .transpose()
    }

    /// Last occurrence trace strictly before `before` (default: now).
    #[pyo3(signature = (before=None))]
    fn previous_trace<'py>(
        &self,
        py: Python<'py>,
        before: Option<DateTime<Utc>>,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.inner
            .try_previous_trace(before.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .map(|trace| trace_to_dict(py, trace))
            .transpose()
    }

    /// Occurrence traces in `(after, before)`, ascending.
    #[pyo3(signature = (before, after=None))]
    fn until_trace<'py>(
        &self,
        py: Python<'py>,
        before: DateTime<Utc>,
        after: Option<DateTime<Utc>>,
    ) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.inner
            .try_until_trace(before, after.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .into_iter()
            .map(|trace| trace_to_dict(py, trace))
            .collect()
    }

    /// Occurrence traces in `(after, before)`, descending.
    #[pyo3(signature = (after, before=None))]
    fn since_trace<'py>(
        &self,
        py: Python<'py>,
        after: DateTime<Utc>,
        before: Option<DateTime<Utc>>,
    ) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.inner
            .try_since_trace(after, before.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .into_iter()
            .map(|trace| trace_to_dict(py, trace))
            .collect()
    }

    /// The next `n` occurrence traces strictly after `after` (default: now), ascending.
    #[pyo3(signature = (n, after=None))]
    fn upcoming_trace<'py>(
        &self,
        py: Python<'py>,
        n: usize,
        after: Option<DateTime<Utc>>,
    ) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.inner
            .try_upcoming_trace(n, after.unwrap_or_else(Utc::now), &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
            .into_iter()
            .map(|trace| trace_to_dict(py, trace))
            .collect()
    }

    /// Whether `instant` is an occurrence of this schedule.
    fn is_occurrence(&self, instant: DateTime<Utc>) -> PyResult<bool> {
        self.inner
            .try_is_occurrence(instant, &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Count occurrences strictly in `(after, before)`.
    fn count_between(&self, after: DateTime<Utc>, before: DateTime<Utc>) -> PyResult<usize> {
        self.inner
            .try_count_between(after, before, &self.calendars)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Human-readable summary.
    fn describe(&self) -> String {
        self.inner.describe()
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
