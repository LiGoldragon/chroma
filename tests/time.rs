use chroma::{LocalHour, LocalMinute, RampDuration, RampTrigger, RelativeSolarOffset, SignedMinutes};
use core::time::Duration;
use nota_codec::{Decoder, Encoder, NotaDecode, NotaEncode};

fn round_trip_nota<T>(value: &T) -> T
where
    T: NotaEncode + NotaDecode,
{
    let mut encoder = Encoder::new();
    value.encode(&mut encoder).expect("encode");
    let text = encoder.into_string();
    let mut decoder = Decoder::new(&text);
    T::decode(&mut decoder).expect("decode")
}

#[test]
fn ramp_duration_clamps_zero_to_min() {
    assert_eq!(RampDuration::from_seconds(0), RampDuration::MIN);
}

#[test]
fn ramp_duration_minutes_become_seconds() {
    let one_hour = RampDuration::from_minutes(60);
    assert_eq!(one_hour.as_seconds(), 3600);
    assert_eq!(one_hour.as_duration(), Duration::from_secs(3600));
}

#[test]
fn ramp_duration_display_uses_minute_seconds() {
    assert_eq!(format!("{}", RampDuration::from_seconds(45)), "45s");
    assert_eq!(format!("{}", RampDuration::from_seconds(60)), "1m");
    assert_eq!(format!("{}", RampDuration::from_seconds(125)), "2m5s");
}

#[test]
fn signed_minutes_clamps_below_min() {
    assert_eq!(SignedMinutes::new(-2000).as_i16(), SignedMinutes::MIN.as_i16());
}

#[test]
fn signed_minutes_clamps_above_max() {
    assert_eq!(SignedMinutes::new(2000).as_i16(), SignedMinutes::MAX.as_i16());
}

#[test]
fn signed_minutes_passes_in_range_unchanged() {
    assert_eq!(SignedMinutes::new(-30).as_i16(), -30);
    assert_eq!(SignedMinutes::new(0).as_i16(), 0);
    assert_eq!(SignedMinutes::new(60).as_i16(), 60);
}

#[test]
fn signed_minutes_display_signs_explicitly() {
    assert_eq!(format!("{}", SignedMinutes::new(-30)), "-30m");
    assert_eq!(format!("{}", SignedMinutes::new(0)), "+0m");
    assert_eq!(format!("{}", SignedMinutes::new(60)), "+60m");
}

#[test]
fn signed_minutes_default_is_zero() {
    assert_eq!(SignedMinutes::default(), SignedMinutes::ZERO);
}

#[test]
fn signed_minutes_negativity() {
    assert!(SignedMinutes::new(-1).is_negative());
    assert!(!SignedMinutes::new(0).is_negative());
    assert!(!SignedMinutes::new(1).is_negative());
}

#[test]
fn relative_solar_offsets_map_to_exact_minutes() {
    assert_eq!(RelativeSolarOffset::ExtremelyEarly.signed_minutes(), SignedMinutes::new(-120));
    assert_eq!(RelativeSolarOffset::VeryEarly.signed_minutes(), SignedMinutes::new(-60));
    assert_eq!(RelativeSolarOffset::Early.signed_minutes(), SignedMinutes::new(-30));
    assert_eq!(RelativeSolarOffset::OnTime.signed_minutes(), SignedMinutes::ZERO);
    assert_eq!(RelativeSolarOffset::Late.signed_minutes(), SignedMinutes::new(30));
    assert_eq!(RelativeSolarOffset::VeryLate.signed_minutes(), SignedMinutes::new(60));
    assert_eq!(RelativeSolarOffset::ExtremelyLate.signed_minutes(), SignedMinutes::new(120));
}

#[test]
fn local_hour_clamps() {
    assert_eq!(LocalHour::new(0).as_u8(), 0);
    assert_eq!(LocalHour::new(23).as_u8(), 23);
    assert_eq!(LocalHour::new(24).as_u8(), 23);
    assert_eq!(LocalHour::new(255).as_u8(), 23);
}

#[test]
fn local_minute_clamps() {
    assert_eq!(LocalMinute::new(0).as_u8(), 0);
    assert_eq!(LocalMinute::new(59).as_u8(), 59);
    assert_eq!(LocalMinute::new(60).as_u8(), 59);
    assert_eq!(LocalMinute::new(255).as_u8(), 59);
}

#[test]
fn local_hour_display_pads_to_two_digits() {
    assert_eq!(format!("{}", LocalHour::new(0)), "00");
    assert_eq!(format!("{}", LocalHour::new(7)), "07");
    assert_eq!(format!("{}", LocalHour::new(23)), "23");
}

#[test]
fn local_minute_display_pads_to_two_digits() {
    assert_eq!(format!("{}", LocalMinute::new(0)), "00");
    assert_eq!(format!("{}", LocalMinute::new(5)), "05");
    assert_eq!(format!("{}", LocalMinute::new(59)), "59");
}

#[test]
fn civil_dawn_requires_geolocation() {
    let trigger = RampTrigger::CivilDawn(SignedMinutes::new(-30));
    assert!(trigger.requires_geolocation());
}

#[test]
fn sunrise_requires_geolocation() {
    let trigger = RampTrigger::Sunrise(SignedMinutes::new(-30));
    assert!(trigger.requires_geolocation());
}

#[test]
fn civil_dusk_requires_geolocation() {
    let trigger = RampTrigger::CivilDusk(SignedMinutes::new(60));
    assert!(trigger.requires_geolocation());
}

#[test]
fn sunset_requires_geolocation() {
    let trigger = RampTrigger::Sunset(SignedMinutes::new(60));
    assert!(trigger.requires_geolocation());
}

#[test]
fn time_of_day_does_not_require_geolocation() {
    let trigger = RampTrigger::TimeOfDay(LocalHour::new(7), LocalMinute::new(30));
    assert!(!trigger.requires_geolocation());
}

#[test]
fn ramp_trigger_display() {
    assert_eq!(format!("{}", RampTrigger::Sunrise(SignedMinutes::new(-30))), "sunrise -30m");
    assert_eq!(format!("{}", RampTrigger::Sunset(SignedMinutes::new(60))), "sunset +60m");
    assert_eq!(format!("{}", RampTrigger::CivilDawn(SignedMinutes::new(-30))), "civil-dawn -30m");
    assert_eq!(format!("{}", RampTrigger::CivilDusk(SignedMinutes::new(60))), "civil-dusk +60m");
    assert_eq!(format!("{}", RampTrigger::TimeOfDay(LocalHour::new(7), LocalMinute::new(30))), "07:30");
}

#[test]
fn ramp_duration_round_trips_minute_aligned_as_minutes() {
    let one_hour = RampDuration::from_minutes(60);
    let mut encoder = Encoder::new();
    one_hour.encode(&mut encoder).expect("encode");
    assert_eq!(encoder.into_string(), "(Minutes 60)");
    assert_eq!(round_trip_nota(&one_hour), one_hour);
}

#[test]
fn ramp_duration_round_trips_seconds_when_not_minute_aligned() {
    let thirty_seconds = RampDuration::from_seconds(30);
    let mut encoder = Encoder::new();
    thirty_seconds.encode(&mut encoder).expect("encode");
    assert_eq!(encoder.into_string(), "(Seconds 30)");
    assert_eq!(round_trip_nota(&thirty_seconds), thirty_seconds);
}

#[test]
fn ramp_duration_decodes_either_form() {
    let mut decoder_minutes = Decoder::new("(Minutes 5)");
    let from_minutes = RampDuration::decode(&mut decoder_minutes).expect("decode minutes");
    assert_eq!(from_minutes.as_seconds(), 300);

    let mut decoder_seconds = Decoder::new("(Seconds 300)");
    let from_seconds = RampDuration::decode(&mut decoder_seconds).expect("decode seconds");
    assert_eq!(from_seconds.as_seconds(), 300);

    assert_eq!(from_minutes, from_seconds);
}
