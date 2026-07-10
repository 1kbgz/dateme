//! WebAssembly bindings for the `dateme` recurrence engine (wasm-bindgen).
//!
//! A `Schedule` is built from its JSON form. Reference instants and results are
//! passed as epoch milliseconds (`number` in JS); the TypeScript wrapper adapts
//! these to `Date` objects and defaults omitted instants to `Date.now()`.

use chrono::{DateTime, NaiveDate, Utc};
use dateme::{CalendarId, CalendarProvider, DefaultCalendars, Schedule as BaseSchedule};
use js_sys::{Function, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub struct Schedule {
    inner: BaseSchedule,
    calendars: JsCalendars,
}

struct JsCalendars {
    defaults: DefaultCalendars,
    custom: Option<JsValue>,
}

impl JsCalendars {
    fn new(custom: Option<JsValue>) -> Self {
        Self {
            defaults: DefaultCalendars::new(),
            custom,
        }
    }
}

impl CalendarProvider for JsCalendars {
    fn contains(&self, id: CalendarId, date: NaiveDate) -> Option<bool> {
        self.defaults.contains(id, date)
    }

    fn contains_custom(&self, name: &str, date: NaiveDate) -> Option<bool> {
        let provider = self.custom.as_ref()?;
        let name = JsValue::from_str(name);
        let date = JsValue::from_str(&date.to_string());
        if provider.is_function() {
            let function: &Function = provider.unchecked_ref();
            return function.call2(&JsValue::NULL, &name, &date).ok()?.as_bool();
        }
        let contains = Reflect::get(provider, &JsValue::from_str("contains")).ok()?;
        let function: Function = contains.dyn_into().ok()?;
        function.call2(provider, &name, &date).ok()?.as_bool()
    }
}

fn from_millis(ms: f64) -> Result<DateTime<Utc>, JsError> {
    DateTime::<Utc>::from_timestamp_millis(ms as i64)
        .ok_or_else(|| JsError::new("timestamp out of range"))
}

fn to_millis(dt: DateTime<Utc>) -> f64 {
    dt.timestamp_millis() as f64
}

#[wasm_bindgen]
impl Schedule {
    /// Build a schedule from its JSON representation. The schedule is validated
    /// on construction; an invalid schedule throws.
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str, calendar_provider: Option<JsValue>) -> Result<Schedule, JsError> {
        let mut inner: BaseSchedule =
            serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
        inner.validate().map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Schedule {
            inner,
            calendars: JsCalendars::new(calendar_provider),
        })
    }

    /// Serialize back to JSON.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.inner).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Structural validation. Throws on an invalid schedule.
    pub fn validate(&self) -> Result<(), JsError> {
        self.inner
            .clone()
            .validate()
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// First occurrence strictly after `after_ms`; `undefined` if none.
    pub fn next(&self, after_ms: f64) -> Result<Option<f64>, JsError> {
        Ok(self
            .inner
            .try_next(from_millis(after_ms)?, &self.calendars)
            .map_err(|e| JsError::new(&e.to_string()))?
            .map(to_millis))
    }

    /// Last occurrence strictly before `before_ms`; `undefined` if none.
    pub fn previous(&self, before_ms: f64) -> Result<Option<f64>, JsError> {
        Ok(self
            .inner
            .try_previous(from_millis(before_ms)?, &self.calendars)
            .map_err(|e| JsError::new(&e.to_string()))?
            .map(to_millis))
    }

    /// Occurrences in `(after_ms, before_ms)`, ascending.
    pub fn until(&self, before_ms: f64, after_ms: f64) -> Result<Vec<f64>, JsError> {
        Ok(self
            .inner
            .try_until(
                from_millis(before_ms)?,
                from_millis(after_ms)?,
                &self.calendars,
            )
            .map_err(|e| JsError::new(&e.to_string()))?
            .into_iter()
            .map(to_millis)
            .collect())
    }

    /// Occurrences in `(after_ms, before_ms)`, descending.
    pub fn since(&self, after_ms: f64, before_ms: f64) -> Result<Vec<f64>, JsError> {
        Ok(self
            .inner
            .try_since(
                from_millis(after_ms)?,
                from_millis(before_ms)?,
                &self.calendars,
            )
            .map_err(|e| JsError::new(&e.to_string()))?
            .into_iter()
            .map(to_millis)
            .collect())
    }

    /// The next `n` occurrences strictly after `after_ms`, ascending.
    pub fn upcoming(&self, n: usize, after_ms: f64) -> Result<Vec<f64>, JsError> {
        Ok(self
            .inner
            .try_upcoming(n, from_millis(after_ms)?, &self.calendars)
            .map_err(|e| JsError::new(&e.to_string()))?
            .into_iter()
            .map(to_millis)
            .collect())
    }

    /// First occurrence trace strictly after `after_ms`; JSON or null.
    #[wasm_bindgen(js_name = nextTraceJSON)]
    pub fn next_trace_json(&self, after_ms: f64) -> Result<Option<String>, JsError> {
        self.inner
            .try_next_trace(from_millis(after_ms)?, &self.calendars)
            .map_err(|e| JsError::new(&e.to_string()))?
            .map(|trace| serde_json::to_string(&trace).map_err(|e| JsError::new(&e.to_string())))
            .transpose()
    }

    /// Last occurrence trace strictly before `before_ms`; JSON or null.
    #[wasm_bindgen(js_name = previousTraceJSON)]
    pub fn previous_trace_json(&self, before_ms: f64) -> Result<Option<String>, JsError> {
        self.inner
            .try_previous_trace(from_millis(before_ms)?, &self.calendars)
            .map_err(|e| JsError::new(&e.to_string()))?
            .map(|trace| serde_json::to_string(&trace).map_err(|e| JsError::new(&e.to_string())))
            .transpose()
    }

    /// Occurrence traces in `(after_ms, before_ms)`, ascending; JSON array.
    #[wasm_bindgen(js_name = untilTraceJSON)]
    pub fn until_trace_json(&self, before_ms: f64, after_ms: f64) -> Result<String, JsError> {
        let traces = self
            .inner
            .try_until_trace(
                from_millis(before_ms)?,
                from_millis(after_ms)?,
                &self.calendars,
            )
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_json::to_string(&traces).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Occurrence traces in `(after_ms, before_ms)`, descending; JSON array.
    #[wasm_bindgen(js_name = sinceTraceJSON)]
    pub fn since_trace_json(&self, after_ms: f64, before_ms: f64) -> Result<String, JsError> {
        let traces = self
            .inner
            .try_since_trace(
                from_millis(after_ms)?,
                from_millis(before_ms)?,
                &self.calendars,
            )
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_json::to_string(&traces).map_err(|e| JsError::new(&e.to_string()))
    }

    /// The next `n` occurrence traces strictly after `after_ms`; JSON array.
    #[wasm_bindgen(js_name = upcomingTraceJSON)]
    pub fn upcoming_trace_json(&self, n: usize, after_ms: f64) -> Result<String, JsError> {
        let traces = self
            .inner
            .try_upcoming_trace(n, from_millis(after_ms)?, &self.calendars)
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_json::to_string(&traces).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Whether `instant_ms` is an occurrence.
    #[wasm_bindgen(js_name = isOccurrence)]
    pub fn is_occurrence(&self, instant_ms: f64) -> Result<bool, JsError> {
        self.inner
            .try_is_occurrence(from_millis(instant_ms)?, &self.calendars)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Count occurrences strictly in `(after_ms, before_ms)`.
    #[wasm_bindgen(js_name = countBetween)]
    pub fn count_between(&self, after_ms: f64, before_ms: f64) -> Result<usize, JsError> {
        self.inner
            .try_count_between(
                from_millis(after_ms)?,
                from_millis(before_ms)?,
                &self.calendars,
            )
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Human-readable summary.
    pub fn describe(&self) -> String {
        self.inner.describe()
    }
}
