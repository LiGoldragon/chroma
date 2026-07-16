//! Typed GeoClue client protocol values.
//!
//! GeoClue initializes `Client.Location` to the root object path. A client
//! subscribes to `LocationUpdated` before `Start`, then treats the signal's
//! new object path as the authority for the location-property read.

use std::pin::Pin;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{Stream, StreamExt};
use zbus::zvariant::OwnedObjectPath;

use crate::error::{Error, Result};
use crate::schedule::Location;

/// Maximum accepted age of a GeoClue fix at the time Chroma receives it.
pub const MAX_LOCATION_AGE: Duration = Duration::from_secs(300);

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
    accuracy_meters: f64,
    timestamp_seconds: u64,
    timestamp_microseconds: u64,
}

impl GeoclueLocationFix {
    /// Construct a fix from the GeoClue location object's properties.
    pub fn new(location: Location, accuracy_meters: f64, timestamp: (u64, u64)) -> Self {
        Self { location, accuracy_meters, timestamp_seconds: timestamp.0, timestamp_microseconds: timestamp.1 }
    }

    /// Validate metadata before allowing a fix to affect solar scheduling.
    pub fn location_at(self, now: SystemTime) -> Result<Location> {
        if !self.accuracy_meters.is_finite() || self.accuracy_meters < 0.0 {
            return Err(Error::GeoclueInvalidAccuracy);
        }
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
        Ok(self.location)
    }
}
