//! Independent reference vectors for Chroma's apparent-solar clock convention.

use chroma::{Location, SolarClockProjection};
use chrono::{TimeZone, Utc};

#[test]
fn noaa_equation_of_time_reference_vectors_keep_the_expected_sign() {
    // US Naval Observatory/NOAA daily equation-of-time references, rounded to
    // the nearest few seconds. Longitude zero makes the projection directly
    // comparable to the published equation of time.
    let february = SolarClockProjection::at(
        Location { latitude: 0.0, longitude: 0.0 },
        Utc.with_ymd_and_hms(2024, 2, 11, 12, 0, 0).unwrap(),
    );
    let november = SolarClockProjection::at(
        Location { latitude: 0.0, longitude: 0.0 },
        Utc.with_ymd_and_hms(2024, 11, 3, 12, 0, 0).unwrap(),
    );

    assert!((february.utc_offset_seconds() + 854).abs() < 30, "February equation of time is negative");
    assert!((november.utc_offset_seconds() - 989).abs() < 30, "November equation of time is positive");
}

#[test]
fn longitude_correction_is_four_minutes_per_degree_and_ignores_civil_dst() {
    let instant = Utc.with_ymd_and_hms(2024, 6, 14, 12, 0, 0).unwrap();
    let prime_meridian = SolarClockProjection::at(Location { latitude: 0.0, longitude: 0.0 }, instant);
    let east = SolarClockProjection::at(Location { latitude: 0.0, longitude: 15.0 }, instant);
    let west = SolarClockProjection::at(Location { latitude: 0.0, longitude: -30.0 }, instant);

    assert!((east.utc_offset_seconds() - prime_meridian.utc_offset_seconds() - 3_600).abs() <= 1);
    assert!((west.utc_offset_seconds() - prime_meridian.utc_offset_seconds() + 7_200).abs() <= 1);
}

#[test]
fn late_december_leap_year_matches_noaa_fractional_year_vectors() {
    // Independent values obtained from NOAA's published fractional-year
    // equation using 366 days for Gregorian leap year 2024, at longitude 0.
    // The old unconditional 365-day denominator misses each by about 27 s.
    let december_twentieth = SolarClockProjection::at(
        Location { latitude: 0.0, longitude: 0.0 },
        Utc.with_ymd_and_hms(2024, 12, 20, 12, 0, 0).unwrap(),
    );
    let december_thirty_first = SolarClockProjection::at(
        Location { latitude: 0.0, longitude: 0.0 },
        Utc.with_ymd_and_hms(2024, 12, 31, 12, 0, 0).unwrap(),
    );

    assert!((december_twentieth.utc_offset_seconds() - 157).abs() <= 1);
    assert!((december_thirty_first.utc_offset_seconds() + 147).abs() <= 1);
}

#[test]
fn utc_date_boundaries_refresh_the_equation_of_time_without_a_timezone() {
    let before_midnight = SolarClockProjection::at(
        Location { latitude: 0.0, longitude: 0.0 },
        Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap(),
    );
    let after_midnight = SolarClockProjection::at(
        Location { latitude: 0.0, longitude: 0.0 },
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
    );

    assert_eq!(
        before_midnight.equation_of_time_valid_until_unix_seconds(),
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap().timestamp()
    );
    assert!(
        after_midnight.equation_of_time_valid_until_unix_seconds()
            > before_midnight.equation_of_time_valid_until_unix_seconds()
    );
}
