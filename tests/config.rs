use chroma::{ConfigFile, ThemeApplier, ThemeMode};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct Fixture {
    temporary_directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self { temporary_directory: tempfile::tempdir().expect("create tempdir") }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.temporary_directory.path().join(name)
    }

    fn write_config(&self, apply_command: &Path) -> PathBuf {
        let config = self.path("config.nota");
        fs::write(
            &config,
            format!(
                r#"(Config
  (Theme
    (ApplyCommand "{}")
    (Schedule (Manual Light)))
  (Warmth (Schedule (Manual Neutral)))
  (Brightness (Schedule (Manual Bright))))"#,
                apply_command.display()
            ),
        )
        .expect("write config");
        config
    }

    fn write_apply_script(&self) -> (PathBuf, PathBuf) {
        let log = self.path("theme.log");
        let script = self.path("apply-theme");
        let shell = std::env::var("CHROMA_TEST_SHELL").unwrap_or_else(|_| "/usr/bin/env bash".into());
        fs::write(&script, format!("#!{shell}\nset -euo pipefail\nprintf '%s\\n' \"$1\" > '{}'\n", log.display()))
            .expect("write script");
        let mut permissions = fs::metadata(&script).expect("script metadata").permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).expect("make script executable");
        (script, log)
    }
}

#[test]
fn config_file_extracts_theme_apply_command_from_nota_config() {
    let fixture = Fixture::new();
    let (script, _) = fixture.write_apply_script();
    let config = fixture.write_config(&script);

    let apply_command = ConfigFile::from_path(config).theme_apply_command().expect("apply command decodes");

    assert_eq!(apply_command.as_path(), script.as_path());
}

#[test]
fn theme_applier_passes_lowercase_mode_to_configured_script() {
    let fixture = Fixture::new();
    let (script, log) = fixture.write_apply_script();
    let apply_command =
        ConfigFile::from_path(fixture.write_config(&script)).theme_apply_command().expect("apply command decodes");
    let applier = ThemeApplier::from_apply_command(apply_command);

    applier.apply(ThemeMode::Dark).expect("theme applies");

    assert_eq!(fs::read_to_string(log).expect("read log"), "dark\n");
}
