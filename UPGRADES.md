# Upgrades

## 0.7.0 — ethos-zero 10.0.0 adds Eq and Hash to every generated derive

`ethos-zero` 10.0.0 widens the derive set every generated Ethos type carries
to include `Eq` and `Hash` alongside `Clone`, `Debug`, and `PartialEq`.
Regenerating `src/generated.rs` from the unchanged `chroma.ethos` source
therefore changes every derive line in the committed module. Because
`generated` is a public module, this is a breaking change to Chroma's own
generated surface, not a pure repin.

Repinned to `protos` 0.31.0, `datom-codec` 0.31.0, and `ethos-zero` 10.0.0.
`datom-codec` 0.31.0 removes every datom kind from `f64`; Chroma's Ethos
contract declares no `Decimal` and `src/generated.rs` carries no `f64`, so
this repin needed no codec-side conversion. Chroma's own `f64` sites (gamma
brightness, solar time, ramp interpolation, geoclue location) sit outside any
datom position and are unchanged. `StoredLocation` in `src/state.rs` holds
two `f64` fields that are rkyv-archived for redb persistence, not a datom
position, and are left as-is; they can still hold a non-finite value with
nothing refusing one.

Regenerate `src/generated.rs` the same way as in 0.4.0; `cargo test --test
ethos_contract` proves the committed module matches the authored source.

## 0.6.0 — Composing replaces Compositional in generated Rust

`datom-codec` 0.27.0 gives arity back to `Compositional` — it now carries
`const ARITY` and `from_positions` and states a positional type's own
positions — and renames the kind a datom composes into to `Composing`. The
derive macro chroma's `chroma.ethos` map generates from is renamed to match,
so every derive in the committed `src/generated.rs` reads
`datom_codec::Composing` where it previously read `datom_codec::Compositional`.
Because `generated` is a public module, this is a breaking change to
Chroma's own generated surface, not a pure repin.

Repinned to `protos` 0.30.1, `datom-codec` 0.27.0, and `ethos-zero` 9.0.0.
Regenerate `src/generated.rs` the same way as in 0.4.0; `cargo test --test
ethos_contract` proves the committed module matches the authored source.

## 0.5.0 — direct single-value Datom payloads

Chroma's one-value request and reply wrappers are aliases in the Ethos
contract. Use direct payloads such as `SetTheme.Light` and
`SetWarmthKelvin.3500`; configurations put warmth and brightness schedules
directly in their top-level positions. The old braced wrapper values are no
longer accepted.

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
