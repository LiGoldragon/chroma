use ethos_zero::{Actualizing, File, Generating, Potential};
use std::{fs, path::PathBuf, process::Command};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = fs::read_to_string(root.join("chroma.ethos")).expect("read Chroma Ethos map");
    let file = Potential::<File>::from(source.as_str()).actualize().expect("read Chroma Ethos map through Protos");
    let rust = file.generate().expect("generate Chroma Datom library");
    let output = root.join("src/generated.rs");
    fs::write(&output, rust).expect("write generated Rust");
    assert!(Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(&output)
        .status()
        .expect("run rustfmt")
        .success());
}
