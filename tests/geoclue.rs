//! Regression coverage for GeoClue's asynchronous location protocol.

use std::time::{Duration, UNIX_EPOCH};

use chroma::{
    Error, FreshLocationLease, GeoclueLocationFix, GeoclueLocationSource, GeoclueLocationUpdate,
    GeoclueLocationUpdateAwaiter, Location, MAX_LOCATION_AGE, MAX_SOLAR_LOCATION_ACCURACY_METERS,
    MINIMUM_SOLAR_CLOCK_VALIDITY, SolarLocationQuality,
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

fn physical_fix(source: GeoclueLocationSource, location: Location, timestamp: u64) -> GeoclueLocationFix {
    GeoclueLocationFix::with_source(location, source, 100.0, (timestamp, 0))
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
async fn renewal_subscription_can_advance_from_cached_fix_to_provider_replacement() {
    let updates = stream::iter([
        Ok(location_update("/org/freedesktop/GeoClue2/Location/cached")),
        Ok(location_update("/org/freedesktop/GeoClue2/Location/replacement")),
    ]);
    let mut awaiter = GeoclueLocationUpdateAwaiter::new(updates);

    assert!(awaiter.location_path().await.expect("cached delivery is readable").as_str().ends_with("/cached"));
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

#[test]
fn stale_geoclue_fix_is_rejected_before_solar_schedule_projection() {
    let now = UNIX_EPOCH + MAX_LOCATION_AGE + Duration::from_secs(1);
    let fix = physical_fix(GeoclueLocationSource::Gnss, Location { latitude: 0.0, longitude: 0.0 }, 0);

    let error = fix.location_at(now).expect_err("stale fix cannot select solar waypoints");
    assert!(matches!(error, Error::GeoclueLocationStale { age_seconds } if age_seconds > MAX_LOCATION_AGE.as_secs()));
}

#[test]
fn ip_fallback_cannot_update_authoritative_lease_or_solar_schedule() {
    let measured_at = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let source = GeoclueLocationSource::from_description("ipf fallback (from WiFi data)");
    let fallback = physical_fix(source, Location { latitude: 1.0, longitude: 1.0 }, 1_000_000);
    let lease = FreshLocationLease::new();

    assert_eq!(
        fallback.solar_quality(),
        SolarLocationQuality::RejectedSource(GeoclueLocationSource::InternetProtocolFallback)
    );
    let error = fallback.location_at(measured_at).expect_err("IP fallback cannot become an authoritative solar fix");
    assert!(matches!(
        error,
        Error::GeoclueLocationSourceRejected { location_source: GeoclueLocationSource::InternetProtocolFallback }
    ));
    assert!(lease.current_at(measured_at).is_none(), "rejected source leaves the authoritative lease empty");
    assert!(
        fallback.location_at(measured_at).ok().map(|location| location.location()).is_none(),
        "the solar schedule has no location to project from a rejected source"
    );
}

#[test]
fn physical_wifi_and_gnss_fixes_can_authorize_solar_use() {
    let measured_at = UNIX_EPOCH + Duration::from_secs(1_000_000);
    for source in [GeoclueLocationSource::PhysicalWifi, GeoclueLocationSource::Gnss] {
        let location = Location { latitude: 1.0, longitude: 1.0 };
        let fix = physical_fix(source, location, 1_000_000);

        assert_eq!(fix.solar_quality(), SolarLocationQuality::Authoritative(source));
        assert_eq!(fix.location_at(measured_at).expect("physical source is usable for solar use").location(), location);
    }
}

#[test]
fn low_accuracy_physical_fix_is_rejected_before_solar_schedule_projection() {
    let measured_at = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let fix = GeoclueLocationFix::with_source(
        Location { latitude: 1.0, longitude: 1.0 },
        GeoclueLocationSource::PhysicalWifi,
        MAX_SOLAR_LOCATION_ACCURACY_METERS + 1.0,
        (1_000_000, 0),
    );

    assert_eq!(fix.solar_quality(), SolarLocationQuality::RejectedAccuracy);
    assert!(matches!(
        fix.location_at(measured_at),
        Err(Error::GeoclueLocationAccuracyTooLow { accuracy_meters }) if accuracy_meters > MAX_SOLAR_LOCATION_ACCURACY_METERS as u64
    ));
}

#[test]
fn nearly_expired_geoclue_fix_is_rejected_before_it_can_flash_solar_time() {
    let measured_at = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let now = measured_at + MAX_LOCATION_AGE - MINIMUM_SOLAR_CLOCK_VALIDITY + Duration::from_secs(1);
    let fix = physical_fix(GeoclueLocationSource::Gnss, Location { latitude: 1.0, longitude: 1.0 }, 1_000_000);

    let error = fix.location_at(now).expect_err("near-expiry fix cannot drive a status-bar projection");
    assert!(matches!(error, Error::GeoclueLocationExpiresSoon { remaining_seconds }
            if remaining_seconds < MINIMUM_SOLAR_CLOCK_VALIDITY.as_secs()));
}

#[test]
fn held_fix_survives_renewal_failures_until_actual_expiry() {
    let measured_at = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let location = Location { latitude: 1.0, longitude: 1.0 };
    let held = physical_fix(GeoclueLocationSource::Gnss, location, 1_000_000)
        .location_at(measured_at)
        .expect("initial fix has its full lease");
    let mut lease = FreshLocationLease::new();
    lease.renew(held);

    for retry_offset in [180, 200, 240, 299, 300] {
        assert_eq!(
            lease
                .current_at(measured_at + Duration::from_secs(retry_offset))
                .expect("failed renewal retains a still-valid fix")
                .location(),
            location
        );
    }
    assert!(
        lease.current_at(measured_at + Duration::from_secs(301)).is_none(),
        "the held fix becomes unavailable only after its actual expiry"
    );
}

#[test]
fn rejected_ip_fallback_replacement_does_not_evict_a_still_valid_trusted_fix() {
    let measured_at = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let held_location = Location { latitude: 1.0, longitude: 1.0 };
    let mut lease = FreshLocationLease::new();
    lease.renew(
        physical_fix(GeoclueLocationSource::Gnss, held_location, 1_000_000)
            .location_at(measured_at)
            .expect("initial trusted fix is accepted"),
    );

    let renewal_time = measured_at + Duration::from_secs(60);
    let rejected_replacement = physical_fix(
        GeoclueLocationSource::InternetProtocolFallback,
        Location { latitude: 2.0, longitude: 2.0 },
        1_000_060,
    );
    assert!(matches!(
        rejected_replacement.location_at(renewal_time),
        Err(Error::GeoclueLocationSourceRejected { location_source: GeoclueLocationSource::InternetProtocolFallback })
    ));
    assert_eq!(
        lease.current_at(renewal_time).expect("rejected replacement cannot clear held trusted fix").location(),
        held_location
    );
}
