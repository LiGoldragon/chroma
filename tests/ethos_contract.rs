//! Chroma's public data anatomies are authored in Ethos.

use datom_codec::{Actualizable, IncorporationBudget, Potential, Textualizable};
use ethos_zero::Generating;
use protos::Potential as ProtosPotential;
use std::{fs, process::Command};

use chroma::generated::{Request, RequestSetTheme, ThemeMode};

fn regenerated_rust() -> String {
    let source = fs::read_to_string("chroma.ethos").expect("read Chroma Ethos map");
    let file = ProtosPotential::<ethos_zero::File>::from(source.as_str()).actualize(()).expect("read Chroma Ethos map");
    let rust = file.generate().expect("generate Chroma Datom library");
    let directory = tempfile::tempdir().expect("create formatting directory");
    let path = directory.path().join("generated.rs");
    fs::copy("rustfmt.toml", directory.path().join("rustfmt.toml")).expect("copy Chroma rustfmt configuration");
    fs::write(&path, rust).expect("write generated Rust for formatting");
    assert!(Command::new("rustfmt").args(["--edition", "2024"]).arg(&path).status().expect("run rustfmt").success());
    fs::read_to_string(path).expect("read formatted generated Rust")
}

#[test]
fn committed_generated_rust_matches_the_authored_ethos_projection() {
    assert_eq!(fs::read_to_string("src/generated.rs").expect("read committed Rust"), regenerated_rust());
}

#[test]
fn generated_request_keeps_the_datom_boundary_in_one_anatomy() {
    let request = Potential::<Request>::from("SetTheme.{Light}")
        .actualize(IncorporationBudget::try_from(4096).expect("positive request budget"))
        .expect("incorporate Datom request");

    assert!(matches!(request, Request::SetTheme(RequestSetTheme(ThemeMode::Light))));
    assert_eq!(request.textualize(), "SetTheme.{ Light }");
}
