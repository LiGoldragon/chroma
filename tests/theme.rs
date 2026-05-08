use chroma::ThemeMode;
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
fn dark_renders_as_dark() {
    assert_eq!(ThemeMode::Dark.as_str(), "dark");
    assert_eq!(format!("{}", ThemeMode::Dark), "dark");
}

#[test]
fn light_renders_as_light() {
    assert_eq!(ThemeMode::Light.as_str(), "light");
    assert_eq!(format!("{}", ThemeMode::Light), "light");
}

#[test]
fn toggled_inverts() {
    assert_eq!(ThemeMode::Dark.toggled(), ThemeMode::Light);
    assert_eq!(ThemeMode::Light.toggled(), ThemeMode::Dark);
}

#[test]
fn toggled_twice_is_identity() {
    assert_eq!(ThemeMode::Dark.toggled().toggled(), ThemeMode::Dark);
    assert_eq!(ThemeMode::Light.toggled().toggled(), ThemeMode::Light);
}

#[test]
fn nota_round_trip_dark() {
    assert_eq!(round_trip_nota(&ThemeMode::Dark), ThemeMode::Dark);
}

#[test]
fn nota_round_trip_light() {
    assert_eq!(round_trip_nota(&ThemeMode::Light), ThemeMode::Light);
}

#[test]
fn nota_encodes_as_pascal_variant_name() {
    let mut encoder = Encoder::nota();
    ThemeMode::Dark.encode(&mut encoder).expect("encode");
    assert_eq!(encoder.into_string(), "Dark");

    let mut encoder = Encoder::nota();
    ThemeMode::Light.encode(&mut encoder).expect("encode");
    assert_eq!(encoder.into_string(), "Light");
}
