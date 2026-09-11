//! Chroma's public data anatomies are authored in Ethos.

use datom_codec::{Actualizing, Budget, Datomizable, Potential};
use ethos_zero::Generating;
use ethos_zero::{Actualizing as _, File, Potential as EthosPotential};
use protos::{Protosizable, Textualizable};
use std::fs;

use chroma::generated::{Request, ThemeMode};

fn regenerated_rust() -> String {
    let source = fs::read_to_string("chroma.ethos").expect("read Chroma Ethos map");
    EthosPotential::<File>::from(source.as_str()).actualize().expect("read Chroma Ethos map").generate().expect("generate Chroma Datom library")
}

#[test]
fn committed_generated_rust_matches_the_authored_ethos_projection() {
    assert_eq!(fs::read_to_string("src/generated.rs").expect("read committed Rust"), regenerated_rust());
}

#[test]
fn generated_request_keeps_the_datom_boundary_in_one_anatomy() {
    let mut request = Potential::<Request>::from("SetTheme.Light");
    let mut budget = Budget { remaining: 4096, reader: protos::ReaderBudget { remaining: 4096 }, depth: 0, maximum_depth: 4096 };
    let request = request
        .actualize(&mut budget)
        .expect("incorporate Datom request");

    assert!(matches!(request, Request::SetTheme(ThemeMode::Light)));
    assert_eq!(request.datomize(vec![]).protosize().textualize(), "SetTheme.Light");
}
