use chroma::{ThemeMode, ThemePalette};
use nota_next::{NotaDecode, NotaEncode, NotaSource};

fn round_trip_nota<T>(value: &T) -> T
where
    T: NotaEncode + NotaDecode + Clone,
{
    let text = value.to_nota();
    NotaSource::new(&text).parse().expect("decode")
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
    let text = ThemeMode::Dark.to_nota();
    assert_eq!(text, "Dark");

    let text = ThemeMode::Light.to_nota();
    assert_eq!(text, "Light");
}

#[test]
fn ghostty_palette_lines_emit_sixteen_indexed_entries() {
    let palette = ThemePalette::from_base16_slots([
        "#000000", "#111111", "#222222", "#333333", "#444444", "#555555", "#666666", "#777777", "#888888", "#999999",
        "#aaaaaa", "#bbbbbb", "#cccccc", "#dddddd", "#eeeeee", "#ffffff",
    ]);

    let lines = palette.ghostty_palette_lines();

    assert!(lines.contains("palette = 0=#000000"));
    assert!(lines.contains("palette = 1=#888888"));
    assert!(lines.contains("palette = 15=#777777"));
    assert_eq!(lines.lines().count(), 16);
}
