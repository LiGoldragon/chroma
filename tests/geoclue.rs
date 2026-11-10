//! Regression coverage for GeoClue's asynchronous location protocol.

use std::time::Duration;

use chroma::{
    Error, GeoclueLocationFix, GeoclueLocationUpdate, GeoclueLocationUpdateAwaiter, Location, LocationAuthority,
    MAX_LOCATION_ACCURACY_METERS,
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

#[tokio::test]
async fn one_lifelong_subscription_delivers_each_location_updated_in_turn() {
    let updates = stream::iter([
        Ok(location_update("/org/freedesktop/GeoClue2/Location/first")),
        Ok(location_update("/org/freedesktop/GeoClue2/Location/replacement")),
    ]);
    let mut awaiter = GeoclueLocationUpdateAwaiter::new(updates);

    assert!(awaiter.location_path().await.expect("the first delivery is readable").as_str().ends_with("/first"));
    assert!(
        awaiter
            .location_path()
            .await
            .expect("the live subscription remains open for a replacement")
            .as_str()
            .ends_with("/replacement")
    );
}

#[test]
fn root_location_update_is_rejected_before_any_location_property_read() {
    let error = location_update("/").location_path().expect_err("root path is GeoClue's unavailable sentinel");

    assert!(matches!(error, Error::GeoclueRootLocation));
}

#[tokio::test]
async fn ended_subscription_is_the_event_that_reports_a_lost_bus() {
    let mut awaiter = GeoclueLocationUpdateAwaiter::new(stream::empty::<chroma::Result<GeoclueLocationUpdate>>());

    assert!(matches!(awaiter.location_path().await, Err(Error::GeoclueLocationUpdateStreamEnded)));
}

#[test]
fn precise_geoclue_fix_is_announced_for_solar_schedule_projection() {
    let location = Location { latitude: 1.0, longitude: 1.0 };
    let fix = GeoclueLocationFix::new(location, MAX_LOCATION_ACCURACY_METERS);

    assert_eq!(fix.validated().expect("a fix within 1 km is accepted").location(), location);
}

#[test]
fn imprecise_geoclue_fix_is_rejected_before_solar_schedule_projection() {
    let fix = GeoclueLocationFix::new(Location { latitude: 1.0, longitude: 1.0 }, MAX_LOCATION_ACCURACY_METERS + 0.1);

    let error = fix.validated().expect_err("a fix wider than 1 km cannot select solar waypoints");
    assert!(
        matches!(error, Error::GeoclueLocationImprecise { accuracy_meters } if accuracy_meters > MAX_LOCATION_ACCURACY_METERS)
    );
}

#[test]
fn malformed_accuracy_is_rejected() {
    for accuracy_meters in [f64::NAN, f64::INFINITY, -1.0] {
        let fix = GeoclueLocationFix::new(Location { latitude: 1.0, longitude: 1.0 }, accuracy_meters);
        assert!(matches!(fix.validated(), Err(Error::GeoclueInvalidAccuracy)));
    }
}

#[test]
fn held_location_is_kept_until_a_location_updated_replaces_it() {
    let held = Location { latitude: 1.0, longitude: 1.0 };
    let replacement = Location { latitude: 2.0, longitude: 2.0 };
    let mut authority = LocationAuthority::default();
    authority.announce(GeoclueLocationFix::new(held, 25.0).validated().expect("initial fix is accepted"));

    // Holding takes no clock: nothing ages the held location away.
    assert_eq!(authority.location(), Some(held));

    // A rejected fix announces nothing, so the held location stays.
    assert!(GeoclueLocationFix::new(replacement, MAX_LOCATION_ACCURACY_METERS + 1.0).validated().is_err());
    assert_eq!(authority.location(), Some(held));

    authority.announce(GeoclueLocationFix::new(replacement, 25.0).validated().expect("replacement is accepted"));
    assert_eq!(authority.location(), Some(replacement));
}

#[test]
fn authority_starts_explicitly_unlocated() {
    assert!(matches!(LocationAuthority::default(), LocationAuthority::Unlocated));
    assert_eq!(LocationAuthority::default().location(), None);
}
