//! Regression coverage for GeoClue's asynchronous location protocol.

use std::time::{Duration, UNIX_EPOCH};

use chroma::{
    Error, GeoclueLocationFix, GeoclueLocationUpdate, GeoclueLocationUpdateAwaiter, Location, MAX_LOCATION_AGE,
    MINIMUM_SOLAR_CLOCK_VALIDITY,
};
use futures_util::stream;
use tokio::sync::oneshot;
use tokio::time::timeout;
use zbus::zvariant::OwnedObjectPath;

fn location_update(path: &str) -> GeoclueLocationUpdate {
    GeoclueLocationUpdate::from_signal_body((
        OwnedObjectPath::try_from("/").expect("root object path is valid D-Bus syntax"),
        OwnedObjectPath::try_from(path).expect("test location object path is valid D-Bus syntax"),
    ))
}

#[tokio::test]
async fn delayed_location_updated_delivers_non_root_path_before_location_properties_are_read() {
    let (send_update, receive_update) = oneshot::channel();
    let updates =
        stream::once(async move { receive_update.await.expect("test sender supplies delayed location update") });
    let mut awaiter = GeoclueLocationUpdateAwaiter::new(updates);

    assert!(timeout(Duration::from_millis(10), awaiter.location_path()).await.is_err());

    send_update
        .send(Ok(location_update("/org/freedesktop/GeoClue2/Location/ready")))
        .expect("the delayed subscription is still open");
    let path = timeout(Duration::from_secs(1), awaiter.location_path())
        .await
        .expect("location update arrives after client start")
        .expect("non-root location update is accepted");

    assert_ne!(path.as_str(), "/");
}

#[test]
fn root_location_update_is_rejected_before_any_location_property_read() {
    let error = location_update("/").location_path().expect_err("root path is GeoClue's unavailable sentinel");

    assert!(matches!(error, Error::GeoclueRootLocation));
}

#[test]
fn stale_geoclue_fix_is_rejected_before_solar_schedule_projection() {
    let now = UNIX_EPOCH + MAX_LOCATION_AGE + Duration::from_secs(1);
    let fix = GeoclueLocationFix::new(Location { latitude: 0.0, longitude: 0.0 }, 25_000.0, (0, 0));

    let error = fix.location_at(now).expect_err("stale fix cannot select solar waypoints");
    assert!(matches!(error, Error::GeoclueLocationStale { age_seconds } if age_seconds > MAX_LOCATION_AGE.as_secs()));
}

#[test]
fn fresh_geoclue_fix_is_accepted_for_solar_schedule_projection() {
    let measured_at = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let location = Location { latitude: 1.0, longitude: 1.0 };
    let fix = GeoclueLocationFix::new(location, 25_000.0, (1_000_000, 0));

    assert_eq!(
        fix.location_at(measured_at + Duration::from_secs(1)).expect("fresh fix is accepted").location(),
        location
    );
}

#[test]
fn nearly_expired_geoclue_fix_is_rejected_before_it_can_flash_solar_time() {
    let measured_at = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let now = measured_at + MAX_LOCATION_AGE - MINIMUM_SOLAR_CLOCK_VALIDITY + Duration::from_secs(1);
    let fix = GeoclueLocationFix::new(Location { latitude: 1.0, longitude: 1.0 }, 25_000.0, (1_000_000, 0));

    let error = fix.location_at(now).expect_err("near-expiry fix cannot drive a status-bar projection");
    assert!(matches!(error, Error::GeoclueLocationExpiresSoon { remaining_seconds }
            if remaining_seconds < MINIMUM_SOLAR_CLOCK_VALIDITY.as_secs()));
}
