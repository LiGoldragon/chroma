//! Generated Datom request boundary and runtime-only rkyv frame tests.

use chroma::{BrightnessLevel, BrightnessPercent, KelvinTemperature, RampDuration, Request, ThemeMode, WarmthLevel};
use datom_codec::{Actualizable, IncorporationBudget, Potential};

fn request(text: &str) -> Request {
    Potential::<chroma::generated::Request>::from(text)
        .actualize(IncorporationBudget::try_from(4096).expect("positive request budget"))
        .unwrap_or_else(|error| panic!("incorporate Datom request {text:?}: {error:?}"))
        .try_into()
        .expect("validate runtime request")
}

fn round_trip_rkyv(request: &Request) {
    assert_eq!(Request::from_archive(&request.archive().expect("archive")).expect("from archive"), *request);
}

#[test]
fn generated_datom_becomes_validated_runtime_requests() {
    let requests = [
        ("SetTheme.{Dark}", Request::SetTheme { mode: ThemeMode::Dark }),
        ("SetWarmth.{Warm}", Request::SetWarmth { level: WarmthLevel::Warm }),
        ("SetWarmthKelvin.{3500}", Request::SetWarmthKelvin { kelvin: KelvinTemperature::new(3500) }),
        (
            "StartWarmthRamp.{Warmest Minutes.60}",
            Request::StartWarmthRamp { target: WarmthLevel::Warmest, duration: RampDuration::from_minutes(60) },
        ),
        ("SetBrightness.{Mid}", Request::SetBrightness { level: BrightnessLevel::Mid }),
        ("SetBrightnessPercent.{65}", Request::SetBrightnessPercent { percent: BrightnessPercent::new(65) }),
        (
            "StartBrightnessRampPercent.{40 Seconds.10}",
            Request::StartBrightnessRampPercent {
                target: BrightnessPercent::new(40),
                duration: RampDuration::from_seconds(10),
            },
        ),
        ("GetSolarClock", Request::GetSolarClock),
    ];
    for (text, expected) in requests {
        let actual = request(text);
        assert_eq!(actual, expected, "{text}");
        round_trip_rkyv(&actual);
    }
}

#[test]
fn generated_datom_rejects_legacy_parenthesis_syntax() {
    assert!(
        Potential::<chroma::generated::Request>::from("SetTheme.(Dark)")
            .actualize(IncorporationBudget::try_from(4096).expect("positive request budget"))
            .is_err()
    );
}

#[test]
fn runtime_validation_rejects_negative_numeric_values() {
    let data = Potential::<chroma::generated::Request>::from("SetWarmthKelvin.{-1}")
        .actualize(IncorporationBudget::try_from(4096).expect("positive request budget"))
        .expect("shape is data");
    assert!(Request::try_from(data).is_err());
}
