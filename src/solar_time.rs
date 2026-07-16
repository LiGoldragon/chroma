//! Local apparent solar-time projection.
//!
//! The projection follows NOAA's published fractional-year equation of time:
//! apparent solar time is UTC plus four minutes per degree of longitude plus
//! the equation of time.  It is deliberately independent of civil time zones
//! and daylight-saving rules.  The daemon exposes only this derived offset,
//! never a coordinate.

use chrono::{DateTime, Datelike, Timelike, Utc};

use crate::schedule::Location;

/// A date-specific correction that converts UTC into local apparent solar time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolarClockProjection {
    utc_offset_seconds: i32,
    equation_of_time_valid_until_unix_seconds: i64,
}

impl SolarClockProjection {
    /// Compute a local apparent-solar correction from a fresh GeoClue location.
    pub fn at(location: Location, now: DateTime<Utc>) -> Self {
        let fractional_hour = now.hour() as f64 + now.minute() as f64 / 60.0 + now.second() as f64 / 3600.0;
        let days_in_year = if now.date_naive().leap_year() { 366.0 } else { 365.0 };
        let fractional_year =
            2.0 * std::f64::consts::PI / days_in_year * (now.ordinal0() as f64 + (fractional_hour - 12.0) / 24.0);
        let equation_of_time_minutes = 229.18
            * (0.000075 + 0.001868 * fractional_year.cos()
                - 0.032077 * fractional_year.sin()
                - 0.014615 * (2.0 * fractional_year).cos()
                - 0.040849 * (2.0 * fractional_year).sin());
        let utc_offset_seconds = ((4.0 * location.longitude + equation_of_time_minutes) * 60.0).round() as i32;
        let next_utc_day = (now.date_naive().succ_opt().expect("UTC dates have a successor"))
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid")
            .and_utc();
        Self { utc_offset_seconds, equation_of_time_valid_until_unix_seconds: next_utc_day.timestamp() }
    }

    /// The correction to add to UTC before formatting the solar clock.
    pub fn utc_offset_seconds(self) -> i32 {
        self.utc_offset_seconds
    }

    /// UTC epoch second after which the equation-of-time correction is refreshed.
    pub fn equation_of_time_valid_until_unix_seconds(self) -> i64 {
        self.equation_of_time_valid_until_unix_seconds
    }
}
