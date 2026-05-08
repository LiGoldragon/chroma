use chroma::{BrightnessLevel, BrightnessPercent};
use nota_codec::{Decoder, Encoder, NotaDecode, NotaEncode};

fn round_trip_nota<T>(value: &T) -> T
where
    T: NotaEncode + NotaDecode + Clone,
{
    let mut encoder = Encoder::nota();
    value.encode(&mut encoder).expect("encode");
    let text = encoder.into_string();
    let mut decoder = Decoder::nota(&text);
    T::decode(&mut decoder).expect("decode")
}

#[test]
fn levels_are_ordered_dim_to_bright_in_percent() {
    let in_order = [
        BrightnessLevel::Dim,
        BrightnessLevel::Dimmer,
        BrightnessLevel::Mid,
        BrightnessLevel::Bright,
        BrightnessLevel::Brighter,
        BrightnessLevel::Brightest,
    ];
    let percents: Vec<u8> = in_order.iter().map(|level| level.percent().as_u8()).collect();
    let mut sorted = percents.clone();
    sorted.sort();
    assert_eq!(percents, sorted, "brighter levels must have higher percent");
}

#[test]
fn brighter_walks_the_ladder_up() {
    let mut level = BrightnessLevel::Dim;
    let walk = std::iter::from_fn(|| {
        let current = level;
        level = level.brighter();
        Some(current)
    })
    .take(8)
    .collect::<Vec<_>>();
    assert_eq!(
        walk,
        vec![
            BrightnessLevel::Dim,
            BrightnessLevel::Dimmer,
            BrightnessLevel::Mid,
            BrightnessLevel::Bright,
            BrightnessLevel::Brighter,
            BrightnessLevel::Brightest,
            BrightnessLevel::Brightest,
            BrightnessLevel::Brightest,
        ]
    );
}

#[test]
fn dimmer_walks_the_ladder_down() {
    let mut level = BrightnessLevel::Brightest;
    let walk = std::iter::from_fn(|| {
        let current = level;
        level = level.dimmer();
        Some(current)
    })
    .take(8)
    .collect::<Vec<_>>();
    assert_eq!(
        walk,
        vec![
            BrightnessLevel::Brightest,
            BrightnessLevel::Brighter,
            BrightnessLevel::Bright,
            BrightnessLevel::Mid,
            BrightnessLevel::Dimmer,
            BrightnessLevel::Dim,
            BrightnessLevel::Dim,
            BrightnessLevel::Dim,
        ]
    );
}

#[test]
fn brighter_and_dimmer_are_almost_inverse() {
    for level in [
        BrightnessLevel::Dimmer,
        BrightnessLevel::Mid,
        BrightnessLevel::Bright,
        BrightnessLevel::Brighter,
    ] {
        assert_eq!(level.brighter().dimmer(), level);
        assert_eq!(level.dimmer().brighter(), level);
    }
}

#[test]
fn default_is_bright() {
    assert_eq!(BrightnessLevel::default(), BrightnessLevel::Bright);
}

#[test]
fn display_uses_lowercase_name() {
    assert_eq!(format!("{}", BrightnessLevel::Brightest), "brightest");
    assert_eq!(format!("{}", BrightnessLevel::Mid), "mid");
}

#[test]
fn percent_clamps_above_max() {
    assert_eq!(BrightnessPercent::new(101).as_u8(), BrightnessPercent::MAX.as_u8());
    assert_eq!(BrightnessPercent::new(u8::MAX).as_u8(), BrightnessPercent::MAX.as_u8());
}

#[test]
fn percent_passes_in_range_unchanged() {
    assert_eq!(BrightnessPercent::new(0).as_u8(), 0);
    assert_eq!(BrightnessPercent::new(50).as_u8(), 50);
    assert_eq!(BrightnessPercent::new(100).as_u8(), 100);
}

#[test]
fn percent_to_fraction_is_one_hundredth() {
    assert_eq!(BrightnessPercent::new(0).as_fraction(), 0.0);
    assert_eq!(BrightnessPercent::new(50).as_fraction(), 0.5);
    assert_eq!(BrightnessPercent::new(100).as_fraction(), 1.0);
}

#[test]
fn lerp_endpoints_match() {
    let dim = BrightnessLevel::Dim.percent();
    let brightest = BrightnessLevel::Brightest.percent();
    assert_eq!(dim.lerp_to(brightest, 0.0), dim);
    assert_eq!(dim.lerp_to(brightest, 1.0), brightest);
}

#[test]
fn lerp_midpoint_is_average() {
    let from = BrightnessPercent::new(20);
    let to = BrightnessPercent::new(80);
    assert_eq!(from.lerp_to(to, 0.5), BrightnessPercent::new(50));
}

#[test]
fn lerp_clamps_fraction_below_zero() {
    let from = BrightnessPercent::new(30);
    let to = BrightnessPercent::new(90);
    assert_eq!(from.lerp_to(to, -1.0), from);
}

#[test]
fn lerp_clamps_fraction_above_one() {
    let from = BrightnessPercent::new(30);
    let to = BrightnessPercent::new(90);
    assert_eq!(from.lerp_to(to, 2.0), to);
}

#[test]
fn percent_display_includes_unit() {
    assert_eq!(format!("{}", BrightnessPercent::new(75)), "75%");
}

#[test]
fn levels_match_canonical_percent() {
    assert_eq!(BrightnessLevel::Dim.percent().as_u8(), 35);
    assert_eq!(BrightnessLevel::Dimmer.percent().as_u8(), 50);
    assert_eq!(BrightnessLevel::Mid.percent().as_u8(), 70);
    assert_eq!(BrightnessLevel::Bright.percent().as_u8(), 85);
    assert_eq!(BrightnessLevel::Brighter.percent().as_u8(), 95);
    assert_eq!(BrightnessLevel::Brightest.percent().as_u8(), 100);
}

#[test]
fn nota_round_trips_every_level() {
    for level in [
        BrightnessLevel::Dim,
        BrightnessLevel::Dimmer,
        BrightnessLevel::Mid,
        BrightnessLevel::Bright,
        BrightnessLevel::Brighter,
        BrightnessLevel::Brightest,
    ] {
        assert_eq!(round_trip_nota(&level), level);
    }
}

#[test]
fn nota_encodes_as_pascal_variant_name() {
    let mut encoder = Encoder::nota();
    BrightnessLevel::Brightest.encode(&mut encoder).expect("encode");
    assert_eq!(encoder.into_string(), "Brightest");
}
