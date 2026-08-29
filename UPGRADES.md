# Upgrades

## 0.3.0 — Datom and Ethos schema migration

Chroma 0.3.0 is a breaking data-boundary release.

- CLI input, CLI output, and `config.datom` are Datom, authored in
  `chroma.ethos`; the generated `src/generated.rs` is committed.
- The old parenthesised data notation and `config.dotos` are not read or
  translated. Replace them with the positional Datom anatomy before upgrading.
- Runtime socket frames remain local rkyv `Request` / `Response` values;
  Datom is embodied before a request becomes a frame and generated again after
  a reply leaves one.

Regenerate the committed anatomy after changing `chroma.ethos`:

```sh
CARGO_TARGET_DIR=target/chroma-regenerate-ethos \
  cargo run --manifest-path tools/regenerate-ethos/Cargo.toml --locked
```

`cargo test --locked --test ethos_contract` proves the generated file is the
byte-for-byte rustfmt output of the pinned Ethos-zero `DatomicLibrary` emitter.
