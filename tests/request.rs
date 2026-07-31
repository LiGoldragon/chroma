//! Round-trip tests for the [`chroma::Request`] enum.
//!
//! Every variant must survive both wire formats:
//! DOTOS (the CLI's argv-parse path) and rkyv (the daemon's UDS frame).

use chroma::{BrightnessLevel, BrightnessPercent, KelvinTemperature, RampDuration, Request, ThemeMode, WarmthLevel};

fn round_trip_dotos(text: &str) -> Request {
    let request = Request::from_dotos(text).expect("dotos decode");
    let rendered = request.to_dotos().expect("dotos encode");
    let again = Request::from_dotos(&rendered).expect("dotos re-decode");
    assert_eq!(request, again, "dotos round-trip changed value");
    request
}

fn round_trip_rkyv(request: &Request) {
    let bytes = request.archive().expect("archive");
    let restored = Request::from_archive(&bytes).expect("from_archive");
    assert_eq!(*request, restored, "rkyv round-trip changed value");
}

#[test]
fn set_warmth_named_preset() {
    let request = round_trip_dotos("SetWarmth.(Warm)");
    assert_eq!(request, Request::SetWarmth { level: WarmthLevel::Warm });
    round_trip_rkyv(&request);
}

#[test]
fn set_warmth_kelvin_arbitrary() {
    let request = round_trip_dotos("SetWarmthKelvin.(3500)");
    assert_eq!(request, Request::SetWarmthKelvin { kelvin: KelvinTemperature::new(3500) });
    round_trip_rkyv(&request);
}

#[test]
fn get_warmth_unit() {
    let request = round_trip_dotos("GetWarmth");
    assert_eq!(request, Request::GetWarmth);
    round_trip_rkyv(&request);
}

#[test]
fn get_solar_clock_never_carries_a_coordinate() {
    let request = round_trip_dotos("GetSolarClock");
    assert_eq!(request, Request::GetSolarClock);
    round_trip_rkyv(&request);
}

#[test]
fn set_theme() {
    let dark = round_trip_dotos("SetTheme.(Dark)");
    assert_eq!(dark, Request::SetTheme { mode: ThemeMode::Dark });

    let light = round_trip_dotos("SetTheme.(Light)");
    assert_eq!(light, Request::SetTheme { mode: ThemeMode::Light });

    round_trip_rkyv(&dark);
    round_trip_rkyv(&light);
}

#[test]
fn start_warmth_ramp_named() {
    let request = round_trip_dotos("StartWarmthRamp.(Warmest Minutes.(60))");
    assert_eq!(
        request,
        Request::StartWarmthRamp { target: WarmthLevel::Warmest, duration: RampDuration::from_minutes(60) }
    );
    round_trip_rkyv(&request);
}

#[test]
fn start_warmth_ramp_kelvin() {
    let request = round_trip_dotos("StartWarmthRampKelvin.(2700 Minutes.(30))");
    assert_eq!(
        request,
        Request::StartWarmthRampKelvin {
            target: KelvinTemperature::new(2700),
            duration: RampDuration::from_minutes(30),
        }
    );
    round_trip_rkyv(&request);
}

#[test]
fn start_warmth_ramp_seconds() {
    let request = round_trip_dotos("StartWarmthRampKelvin.(3500 Seconds.(45))");
    assert_eq!(
        request,
        Request::StartWarmthRampKelvin {
            target: KelvinTemperature::new(3500),
            duration: RampDuration::from_seconds(45),
        }
    );
    round_trip_rkyv(&request);
}

#[test]
fn interrupt_warmth() {
    let request = round_trip_dotos("InterruptWarmth");
    assert_eq!(request, Request::InterruptWarmth);
    round_trip_rkyv(&request);
}

#[test]
fn set_brightness_named() {
    let request = round_trip_dotos("SetBrightness.(Mid)");
    assert_eq!(request, Request::SetBrightness { level: BrightnessLevel::Mid });
    round_trip_rkyv(&request);
}

#[test]
fn set_brightness_percent() {
    let request = round_trip_dotos("SetBrightnessPercent.(65)");
    assert_eq!(request, Request::SetBrightnessPercent { percent: BrightnessPercent::new(65) });
    round_trip_rkyv(&request);
}

#[test]
fn start_brightness_ramp_named() {
    let request = round_trip_dotos("StartBrightnessRamp.(Dim Minutes.(5))");
    assert_eq!(
        request,
        Request::StartBrightnessRamp { target: BrightnessLevel::Dim, duration: RampDuration::from_minutes(5) }
    );
    round_trip_rkyv(&request);
}

#[test]
fn start_brightness_ramp_percent() {
    let request = round_trip_dotos("StartBrightnessRampPercent.(40 Seconds.(10))");
    assert_eq!(
        request,
        Request::StartBrightnessRampPercent {
            target: BrightnessPercent::new(40),
            duration: RampDuration::from_seconds(10),
        }
    );
    round_trip_rkyv(&request);
}

#[test]
fn interrupt_brightness() {
    let request = round_trip_dotos("InterruptBrightness");
    assert_eq!(request, Request::InterruptBrightness);
    round_trip_rkyv(&request);
}

#[test]
fn get_state() {
    let request = round_trip_dotos("GetState");
    assert_eq!(request, Request::GetState);
    round_trip_rkyv(&request);
}
