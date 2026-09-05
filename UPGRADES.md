# Upgrades

## 0.4.0 — current typed Datom codec

Chroma now uses `datom-codec`, current Protos, and current Ethos-generated
types at its CLI/config/reply boundary. The retired `datomic` crate,
`Text<T>`, and `TextEdge` are unavailable. CLI input and configuration use
`Potential<T>::actualize(IncorporationBudget)`; generated replies use
`Textualizable`.

Current Datom canonicalizes structural whitespace, for example
`SetWarmth.{ Warm }` and `SolarClock.{ -854 1736208000 }`. Existing compact
input remains parsed when it is valid Datom, while old parenthesised Dotos is
rejected. The local rkyv request/reply state and redb durable-store format are
unchanged.

Regenerate `src/generated.rs` from the authored `chroma.ethos` source with:

```sh
ethos-zero 'Generate.{ chroma.ethos . }'
rustfmt --edition 2024 src/generated.rs
```

`cargo test --locked --test ethos_contract` proves the committed generated
module is the rustfmt projection of that authored source.

## 0.3.1 — Hermetic Ethos map source

The Nix package source retains the root authored `chroma.ethos` map alongside
Crane's Cargo sources. This lets the committed-generation contract read the
same Ethos input in a hermetic build without broadening the source filter to
unrelated repository files.

The sandbox fixture still writes canonical Datom string delimiters, now
constructed at runtime so its shell source remains ASCII and passes ShellCheck.

## 0.3.0 — historical Datom and Ethos schema migration

Chroma 0.3.0 is a breaking data-boundary release.

- CLI input, CLI output, and `config.datom` are Datom, authored in
  `chroma.ethos`; the generated `src/generated.rs` is committed.
- The old parenthesised data notation and `config.dotos` are not read or
  translated. Replace them with the positional Datom anatomy before upgrading.
- Runtime socket frames remain local rkyv `Request` / `Response` values;
  Datom is embodied before a request becomes a frame and generated again after
  a reply leaves one.

The generator command in this historical note was superseded in 0.4.0.
