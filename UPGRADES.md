# Upgrades

## 0.8.1 — GeoClue location is held until GeoClue announces another

Chroma no longer polls GeoClue. It holds one GeoClue client for the daemon's
life and answers each `LocationUpdated` signal; a held location stays until the
next announcement replaces it. A fix wider than 1 km, or with malformed
accuracy, is logged and the held location stays. When `org.freedesktop.GeoClue2`
changes owner on the system bus, that `NameOwnerChanged` event reconnects the
client. Resume, config reload and schedule deadlines no longer read GeoClue.
The client still requests GeoClue's `Exact` accuracy level.

Removed: the five-minute fix lease (`MAX_LOCATION_AGE`,
`MINIMUM_SOLAR_CLOCK_VALIDITY`, `FreshLocationLease`, `FreshGeoclueLocation`),
its renewal schedule, the 10 s and 60 s retries, the 135 s request timeout, the
timestamp read, and the `GeoclueLocationStale`, `GeoclueLocationExpiresSoon`
and `GeoclueInvalidTimestamp` errors. `GeoclueLocationFix::new` takes the
location and accuracy only; `validated` returns a `LocationAnnounced` event,
which `LocationAuthority::announce` holds. `GeoclueWatcher` is the new actor.

The config format, the redb state format and the CLI wire are unchanged.
`checks.session-dbus` runs its private-bus witnesses one at a time, since the
theme witness reads the next `NameOwnerChanged` on the shared bus.

## 0.8.0 — ethos-zero 16.0.0: every generated type derives rkyv

`ethos-zero` 16.0.0 makes every generated struct and enum derive rkyv
`Archive`, `Serialize` and `Deserialize` beside `Clone`, `Debug`, `PartialEq`,
`Eq` and `Hash`, and puts `Datomizable` and `Composing` behind a `datom`
feature of the crate that holds the generated Rust. `src/generated.rs` is
regenerated accordingly, so the generated types of `chroma::generated` now
carry the rkyv derives and a `cfg_attr(feature = "datom", ...)` line.

Chroma declares `datom` as a default feature: its library, CLI and config
reader textualize with Datom in every build, so `--no-default-features` is not a
supported build. Repinned to `ethos-zero` 16.0.0, `protos` 0.32.2 and
`datom-codec` 0.32.2, both with their `rkyv` feature.

Regenerate `src/generated.rs` as in 0.4.0; `cargo test --test ethos_contract`
proves the committed module matches the authored source and that a generated
request archives with rkyv and reads back equal. Deploy by rebuilding the
package; the wire and redb formats are those of the generated types, so a
running daemon is restarted with its CLI together.

## 0.7.1 — request exact GeoClue accuracy

Chroma now requests GeoClue's `Exact` accuracy level (the public enum value
8). The former value 1 requested `Country`; GeoClue's static provider added a
3 km privacy margin to the configured 1 km accuracy and Chroma correctly
rejected the resulting 4 km fix. The accepted-location ceiling remains 1 km,
and rejected renewals continue to retain a still-valid precise fix.

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
