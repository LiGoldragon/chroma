use chroma::{KelvinTemperature, WarmthLevel};
use dotos::{DotosDecode, DotosEncode, DotosSource};

fn round_trip_dotos<T>(value: &T) -> T
where
    T: DotosEncode + DotosDecode + Clone,
{
    let text = value.to_dotos();
    DotosSource::new(&text).parse().expect("decode")
}

#[test]
fn levels_are_ordered_warm_to_cold_in_kelvin() {
    let in_order = [
        WarmthLevel::Warmest,
        WarmthLevel::Warmer,
        WarmthLevel::Warm,
        WarmthLevel::Neutral,
        WarmthLevel::Cool,
        WarmthLevel::Cold,
    ];
    let kelvins: Vec<u16> = in_order.iter().map(|level| level.kelvin().as_u16()).collect();
    let mut sorted = kelvins.clone();
    sorted.sort();
    assert_eq!(kelvins, sorted, "warmer levels must have lower kelvin");
}

#[test]
fn warmer_walks_the_ladder_up() {
    let mut level = WarmthLevel::Cold;
    let walk = std::iter::from_fn(|| {
        let current = level;
        level = level.warmer();
        Some(current)
    })
    .take(8)
    .collect::<Vec<_>>();
    assert_eq!(
        walk,
        vec![
            WarmthLevel::Cold,
            WarmthLevel::Cool,
            WarmthLevel::Neutral,
            WarmthLevel::Warm,
            WarmthLevel::Warmer,
            WarmthLevel::Warmest,
            WarmthLevel::Warmest,
            WarmthLevel::Warmest,
        ]
    );
}

#[test]
fn cooler_walks_the_ladder_down() {
    let mut level = WarmthLevel::Warmest;
    let walk = std::iter::from_fn(|| {
        let current = level;
        level = level.cooler();
        Some(current)
    })
    .take(8)
    .collect::<Vec<_>>();
    assert_eq!(
        walk,
        vec![
            WarmthLevel::Warmest,
            WarmthLevel::Warmer,
            WarmthLevel::Warm,
            WarmthLevel::Neutral,
            WarmthLevel::Cool,
            WarmthLevel::Cold,
            WarmthLevel::Cold,
            WarmthLevel::Cold,
        ]
    );
}

#[test]
fn warmer_and_cooler_are_almost_inverse() {
    for level in [WarmthLevel::Cool, WarmthLevel::Neutral, WarmthLevel::Warm, WarmthLevel::Warmer] {
        assert_eq!(level.warmer().cooler(), level);
        assert_eq!(level.cooler().warmer(), level);
    }
}

#[test]
fn default_is_neutral() {
    assert_eq!(WarmthLevel::default(), WarmthLevel::Neutral);
}

#[test]
fn display_uses_lowercase_name() {
    assert_eq!(format!("{}", WarmthLevel::Warmest), "warmest");
    assert_eq!(format!("{}", WarmthLevel::Neutral), "neutral");
}

#[test]
fn kelvin_clamps_below_min() {
    assert_eq!(KelvinTemperature::new(0).as_u16(), KelvinTemperature::MIN.as_u16());
    assert_eq!(KelvinTemperature::new(500).as_u16(), KelvinTemperature::MIN.as_u16());
}

#[test]
fn kelvin_clamps_above_max() {
    assert_eq!(KelvinTemperature::new(20_000).as_u16(), KelvinTemperature::MAX.as_u16());
    assert_eq!(KelvinTemperature::new(u16::MAX).as_u16(), KelvinTemperature::MAX.as_u16());
}

#[test]
fn kelvin_passes_in_range_unchanged() {
    assert_eq!(KelvinTemperature::new(3500).as_u16(), 3500);
    assert_eq!(KelvinTemperature::new(6500).as_u16(), 6500);
}

#[test]
fn lerp_endpoints_match() {
    let cold = WarmthLevel::Cold.kelvin();
    let warmest = WarmthLevel::Warmest.kelvin();
    assert_eq!(cold.lerp_to(warmest, 0.0), cold);
    assert_eq!(cold.lerp_to(warmest, 1.0), warmest);
}

#[test]
fn lerp_midpoint_is_average() {
    let cold = WarmthLevel::Cold.kelvin();
    let warmest = WarmthLevel::Warmest.kelvin();
    let middle = cold.lerp_to(warmest, 0.5);
    let expected = (cold.as_u16() as u32 + warmest.as_u16() as u32) / 2;
    assert_eq!(middle.as_u16() as u32, expected);
}

#[test]
fn lerp_clamps_fraction_below_zero() {
    let cold = WarmthLevel::Cold.kelvin();
    let warmest = WarmthLevel::Warmest.kelvin();
    assert_eq!(cold.lerp_to(warmest, -1.0), cold);
}

#[test]
fn lerp_clamps_fraction_above_one() {
    let cold = WarmthLevel::Cold.kelvin();
    let warmest = WarmthLevel::Warmest.kelvin();
    assert_eq!(cold.lerp_to(warmest, 2.0), warmest);
}

#[test]
fn kelvin_display_includes_unit() {
    assert_eq!(format!("{}", KelvinTemperature::new(3500)), "3500K");
}

#[test]
fn levels_match_canonical_kelvin() {
    assert_eq!(WarmthLevel::Cold.kelvin().as_u16(), 6500);
    assert_eq!(WarmthLevel::Cool.kelvin().as_u16(), 5500);
    assert_eq!(WarmthLevel::Neutral.kelvin().as_u16(), 4500);
    assert_eq!(WarmthLevel::Warm.kelvin().as_u16(), 3700);
    assert_eq!(WarmthLevel::Warmer.kelvin().as_u16(), 3200);
    assert_eq!(WarmthLevel::Warmest.kelvin().as_u16(), 2700);
}

#[test]
fn dotos_round_trips_every_level() {
    for level in [
        WarmthLevel::Cold,
        WarmthLevel::Cool,
        WarmthLevel::Neutral,
        WarmthLevel::Warm,
        WarmthLevel::Warmer,
        WarmthLevel::Warmest,
    ] {
        assert_eq!(round_trip_dotos(&level), level);
    }
}

#[test]
fn dotos_encodes_as_pascal_variant_name() {
    let text = WarmthLevel::Warmest.to_dotos();
    assert_eq!(text, "Warmest");
}
