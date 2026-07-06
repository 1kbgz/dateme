//! WebAssembly bindings for the `dateme` recurrence engine (wasm-bindgen).
//!
//! A `Schedule` is built from its JSON form. Reference instants and results are
//! passed as epoch milliseconds (`number` in JS); the TypeScript wrapper adapts
//! these to `Date` objects and defaults omitted instants to `Date.now()`.

use chrono::{DateTime, Utc};
use dateme::{DefaultCalendars, Schedule as BaseSchedule};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Schedule {
    inner: BaseSchedule,
    calendars: DefaultCalendars,
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
    /// Build a schedule from its JSON representation.
    #[wasm_bindgen(constructor)]
    pub fn new(json: &str) -> Result<Schedule, JsError> {
        let inner: BaseSchedule =
            serde_json::from_str(json).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Schedule {
            inner,
            calendars: DefaultCalendars::new(),
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
            .next(from_millis(after_ms)?, &self.calendars)
            .map(to_millis))
    }

    /// Last occurrence strictly before `before_ms`; `undefined` if none.
    pub fn previous(&self, before_ms: f64) -> Result<Option<f64>, JsError> {
        Ok(self
            .inner
            .previous(from_millis(before_ms)?, &self.calendars)
            .map(to_millis))
    }

    /// Occurrences in `(after_ms, before_ms)`, ascending.
    pub fn until(&self, before_ms: f64, after_ms: f64) -> Result<Vec<f64>, JsError> {
        Ok(self
            .inner
            .until(
                from_millis(before_ms)?,
                from_millis(after_ms)?,
                &self.calendars,
            )
            .into_iter()
            .map(to_millis)
            .collect())
    }

    /// Occurrences in `(after_ms, before_ms)`, descending.
    pub fn since(&self, after_ms: f64, before_ms: f64) -> Result<Vec<f64>, JsError> {
        Ok(self
            .inner
            .since(
                from_millis(after_ms)?,
                from_millis(before_ms)?,
                &self.calendars,
            )
            .into_iter()
            .map(to_millis)
            .collect())
    }

    /// The next `n` occurrences strictly after `after_ms`, ascending.
    pub fn upcoming(&self, n: usize, after_ms: f64) -> Result<Vec<f64>, JsError> {
        Ok(self
            .inner
            .upcoming(n, from_millis(after_ms)?, &self.calendars)
            .into_iter()
            .map(to_millis)
            .collect())
    }
}
