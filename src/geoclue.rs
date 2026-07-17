//! Typed GeoClue client protocol values.
//!
//! GeoClue initializes `Client.Location` to the root object path. A client
//! subscribes to `LocationUpdated` before `Start`, then treats the signal's
//! new object path as the authority for the location-property read.

use std::fmt::{Display, Formatter};
use std::pin::Pin;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{Stream, StreamExt};
use zbus::zvariant::OwnedObjectPath;

use crate::error::{Error, Result};
use crate::schedule::Location;

/// Maximum accepted age of a GeoClue fix at the time Chroma receives it.
pub const MAX_LOCATION_AGE: Duration = Duration::from_secs(300);

/// A replacement fix needs enough remaining freshness for an early renewal
/// attempt plus several bounded retries. Shorter-lived cached fixes are not
/// allowed to displace a still-valid held fix.
pub const MINIMUM_SOLAR_CLOCK_VALIDITY: Duration = Duration::from_secs(120);

/// Maximum measurement uncertainty that may drive solar events or apparent
/// solar time. One kilometre keeps the location contribution to solar-time
/// projection small while still allowing physical Wi-Fi positioning.
pub const MAX_SOLAR_LOCATION_ACCURACY_METERS: f64 = 1_000.0;

const SOLAR_CLOCK_REFRESH_LEAD: Duration = MINIMUM_SOLAR_CLOCK_VALIDITY;
const MAX_SOLAR_CLOCK_REFRESH_DELAY: Duration = Duration::from_secs(240);

/// The provenance category Chroma can establish from GeoClue's description.
///
/// The original description remains at the D-Bus boundary: diagnostics expose
/// this closed category, never provider text or a location value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoclueLocationSource {
    /// A direct physical Wi-Fi measurement.
    PhysicalWifi,
    /// A GNSS/GPS measurement.
    Gnss,
    /// A network provider's Internet-protocol fallback rather than a physical
    /// measurement. This includes BeaconDB's GeoClue IP fallback.
    InternetProtocolFallback,
    /// A provider description that does not establish a physical source.
    Unrecognized,
}

impl GeoclueLocationSource {
    /// Classify the provider description once at the GeoClue boundary.
    pub fn from_description(description: &str) -> Self {
        let description = description.trim().to_ascii_lowercase();
        if description.contains("ipf fallback") || description.contains("ip fallback") || description.contains("geoip")
        {
            Self::InternetProtocolFallback
        } else if description.contains("gnss") || description.contains("gps") {
            Self::Gnss
        } else if description.contains("wifi") || description.contains("wi-fi") {
            Self::PhysicalWifi
        } else {
            Self::Unrecognized
        }
    }
}

impl Display for GeoclueLocationSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PhysicalWifi => formatter.write_str("physical Wi-Fi"),
            Self::Gnss => formatter.write_str("GNSS"),
            Self::InternetProtocolFallback => formatter.write_str("IP fallback"),
            Self::Unrecognized => formatter.write_str("unrecognized"),
        }
    }
}

/// The closed solar-use policy result for one GeoClue fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolarLocationQuality {
    /// A physical source with bounded enough uncertainty to drive solar use.
    Authoritative(GeoclueLocationSource),
    /// The source did not establish a physical location measurement.
    RejectedSource(GeoclueLocationSource),
    /// The source was physical, but its uncertainty is too large for solar use.
    RejectedAccuracy,
}

impl SolarLocationQuality {
    /// Convert the policy classification into the typed boundary error.
    pub fn authorize(self, accuracy_meters: f64) -> Result<()> {
        match self {
            Self::Authoritative(_) => Ok(()),
            Self::RejectedSource(location_source) => Err(Error::GeoclueLocationSourceRejected { location_source }),
            Self::RejectedAccuracy => {
                Err(Error::GeoclueLocationAccuracyTooLow { accuracy_meters: accuracy_meters.round() as u64 })
            }
        }
    }
}

/// The object paths carried by GeoClue's `LocationUpdated` signal.
#[derive(Debug)]
pub struct GeoclueLocationUpdate {
    new_location_path: OwnedObjectPath,
}

impl GeoclueLocationUpdate {
    /// Decode the `old_location, new_location` signal body.
    pub fn from_signal_body((_old_location_path, new_location_path): (OwnedObjectPath, OwnedObjectPath)) -> Self {
        Self { new_location_path }
    }

    /// Return the delivered location object path, rejecting GeoClue's root sentinel.
    pub fn location_path(self) -> Result<OwnedObjectPath> {
        if self.new_location_path.as_str() == "/" {
            return Err(Error::GeoclueRootLocation);
        }
        Ok(self.new_location_path)
    }
}

/// An awaited GeoClue location-update subscription.
pub struct GeoclueLocationUpdateAwaiter<Updates> {
    updates: Pin<Box<Updates>>,
}

impl<Updates> GeoclueLocationUpdateAwaiter<Updates> {
    /// Take ownership of a subscription installed before `Client.Start`.
    pub fn new(updates: Updates) -> Self {
        Self { updates: Box::pin(updates) }
    }
}

impl<Updates> GeoclueLocationUpdateAwaiter<Updates>
where
    Updates: Stream<Item = Result<GeoclueLocationUpdate>>,
{
    /// Await the first post-start location delivery without reading `Client.Location`.
    pub async fn location_path(&mut self) -> Result<OwnedObjectPath> {
        let update = self.updates.as_mut().next().await.ok_or(Error::GeoclueLocationUpdateStreamEnded)??;
        update.location_path()
    }
}

/// A GeoClue location property's measurement metadata.
#[derive(Debug, Clone, Copy)]
pub struct GeoclueLocationFix {
    location: Location,
    source: GeoclueLocationSource,
    accuracy_meters: f64,
    timestamp_seconds: u64,
    timestamp_microseconds: u64,
}

/// A fix that passed GeoClue source, accuracy, and freshness validation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreshGeoclueLocation {
    location: Location,
    expires_at: SystemTime,
}

/// The schedule actor's held authoritative location lease.
///
/// Failed, low-quality, and insufficient-lifetime renewals leave this lease
/// unchanged. That makes rejected renewal the normal case until the held fix
/// actually expires.
#[derive(Debug, Clone, Copy, Default)]
pub struct FreshLocationLease {
    held: Option<FreshGeoclueLocation>,
}

impl FreshLocationLease {
    /// Start without an authoritative live fix.
    pub fn new() -> Self {
        Self::default()
    }

    /// Accept a validated replacement without an intermediate empty state.
    pub fn renew(&mut self, replacement: FreshGeoclueLocation) {
        self.held = Some(replacement);
    }

    /// Whether this actor has ever held a validated fix. An expired lease is
    /// still reason to use the short recovery retry rather than cold-start cadence.
    pub fn has_held(self) -> bool {
        self.held.is_some()
    }

    /// Return the held fix through its actual expiry instant.
    pub fn current_at(self, now: SystemTime) -> Option<FreshGeoclueLocation> {
        self.held.filter(|location| location.is_current_at(now))
    }
}

impl FreshGeoclueLocation {
    /// Location accepted from the one authoritative GeoClue path.
    pub fn location(self) -> Location {
        self.location
    }

    /// Whether the fix is still safe to project into a solar clock.
    pub fn is_current_at(self, now: SystemTime) -> bool {
        now <= self.expires_at
    }

    /// Schedule the next coarse refresh before this accepted fix expires.
    pub fn refresh_delay_at(self, now: SystemTime) -> Duration {
        self.expires_at
            .duration_since(now)
            .unwrap_or(Duration::ZERO)
            .saturating_sub(SOLAR_CLOCK_REFRESH_LEAD)
            .min(MAX_SOLAR_CLOCK_REFRESH_DELAY)
    }
}

impl GeoclueLocationFix {
    /// Construct a fix with an unrecognized source. It is retained for source
    /// compatibility, but cannot authorize solar use.
    pub fn new(location: Location, accuracy_meters: f64, timestamp: (u64, u64)) -> Self {
        Self::with_source(location, GeoclueLocationSource::Unrecognized, accuracy_meters, timestamp)
    }

    /// Construct a fix from GeoClue's location properties and classified source.
    pub fn with_source(
        location: Location,
        source: GeoclueLocationSource,
        accuracy_meters: f64,
        timestamp: (u64, u64),
    ) -> Self {
        Self { location, source, accuracy_meters, timestamp_seconds: timestamp.0, timestamp_microseconds: timestamp.1 }
    }

    /// Classify this fix against the closed solar authority policy.
    pub fn solar_quality(self) -> SolarLocationQuality {
        match self.source {
            GeoclueLocationSource::PhysicalWifi | GeoclueLocationSource::Gnss
                if self.accuracy_meters <= MAX_SOLAR_LOCATION_ACCURACY_METERS =>
            {
                SolarLocationQuality::Authoritative(self.source)
            }
            GeoclueLocationSource::PhysicalWifi | GeoclueLocationSource::Gnss => SolarLocationQuality::RejectedAccuracy,
            source => SolarLocationQuality::RejectedSource(source),
        }
    }

    /// Validate source, accuracy, and freshness before allowing a fix to affect solar scheduling.
    pub fn location_at(self, now: SystemTime) -> Result<FreshGeoclueLocation> {
        if !self.accuracy_meters.is_finite() || self.accuracy_meters < 0.0 {
            return Err(Error::GeoclueInvalidAccuracy);
        }
        self.solar_quality().authorize(self.accuracy_meters)?;
        if self.timestamp_microseconds >= 1_000_000 {
            return Err(Error::GeoclueInvalidTimestamp);
        }
        let timestamp = Duration::from_secs(self.timestamp_seconds)
            .checked_add(Duration::from_micros(self.timestamp_microseconds))
            .ok_or(Error::GeoclueInvalidTimestamp)?;
        let measured_at = UNIX_EPOCH.checked_add(timestamp).ok_or(Error::GeoclueInvalidTimestamp)?;
        let age = now.duration_since(measured_at).map_err(|_| Error::GeoclueInvalidTimestamp)?;
        if age > MAX_LOCATION_AGE {
            return Err(Error::GeoclueLocationStale { age_seconds: age.as_secs() });
        }
        let expires_at = measured_at.checked_add(MAX_LOCATION_AGE).ok_or(Error::GeoclueInvalidTimestamp)?;
        let remaining = expires_at.duration_since(now).map_err(|_| Error::GeoclueInvalidTimestamp)?;
        if remaining < MINIMUM_SOLAR_CLOCK_VALIDITY {
            return Err(Error::GeoclueLocationExpiresSoon { remaining_seconds: remaining.as_secs() });
        }
        Ok(FreshGeoclueLocation { location: self.location, expires_at })
    }
}
