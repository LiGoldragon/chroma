//! Generated Datom request boundary and runtime-only rkyv frame tests.

use chroma::{BrightnessLevel, BrightnessPercent, KelvinTemperature, RampDuration, Request, ThemeMode, WarmthLevel};
use datomic::{Text, TextEdge};

fn request(text: &str) -> Request {
    Text::<chroma::generated::Request>::from(text)
        .embody()
        .unwrap_or_else(|error| panic!("embody Datom request {text:?}: {error:?}"))
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
    assert!(Text::<chroma::generated::Request>::from("SetTheme.(Dark)").embody().is_err());
}

#[test]
fn runtime_validation_rejects_negative_numeric_values() {
    let data = Text::<chroma::generated::Request>::from("SetWarmthKelvin.{-1}").embody().expect("shape is data");
    assert!(Request::try_from(data).is_err());
}
