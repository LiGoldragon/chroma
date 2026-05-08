use chroma::ThemeMode;

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
