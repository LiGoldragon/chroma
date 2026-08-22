# chroma

One Rust daemon for the colour state of the desktop — theme,
warmth, and brightness — controlled via DOTOS.

`chroma` replaces darkman, the `nightshift` shell wrapper, the
three `nightshift-*` systemd services, and the `brightness` shell
wrapper with a single user-service component:

- **Three independent axes** (theme, warmth, brightness), each
  with its own schedule and applier.
- **Geolocation-driven schedules** via geoclue2 (sunrise /
  sunset / civil twilight, exact or readable early/late offsets,
  per-axis ramp durations).
- **Persisted state** in redb + rkyv, crash-consistent across
  resume / login / wake.
- **One CLI** that takes a single DOTOS record on argv —
  `chroma 'SetWarmth.(Warm)'` — and signals the daemon over a
  Unix domain socket using length-prefixed rkyv frames.
- **Native and resident theme concerns** — Chroma owns terminal,
  desktop, Ghostty, and Pi application as independent concern
  actors. Emacs is a resident same-user session-bus consumer:
  `io.github.LiGoldragon.Chroma.Theme1` publishes full desired
  snapshots and receives bounded acknowledgements. `SetTheme` returns `Accepted` once those actors own
  the request; slow desktop or app work cannot hold terminal
  updates hostage.

## Emacs session-bus boundary

Chroma owns `io.github.LiGoldragon.Chroma` on the same-user session
bus, at `/io/github/LiGoldragon/Chroma/Theme`, interface
`io.github.LiGoldragon.Chroma.Theme1`.

- `RegisterConsumer("emacs") -> (state: "Light"|"Dark", revision: uint64)` binds `emacs` to the caller's unique sender and returns the full snapshot.
- `DesiredStateChanged(state, revision)` broadcasts each changed desired snapshot.
- `ReportProjection("emacs", revision, "Applied"|"Failed", code, summary)` has a fixed signature; `Applied` carries empty `code` and `summary`. Failed codes are `configuration`, `load-failed`, `verification-failed`, or `application-failed`; summaries are at most 240 UTF-8 bytes.
- `GetProjectionStatus("emacs") -> (status, revision)` exposes `Unavailable`, `Pending`, `Applied`, or `Failed`.

Reports for the current revision reconcile the postcondition in either
direction: a later `Failed` may replace `Applied`, and a later `Applied` may
replace `Failed`. Stale reports remain no-ops only after their failure payload
passes the same finite code and 240-byte summary bounds as a current report.

The desired `{ThemeMode, revision}` snapshot is atomically persisted. A
theme-only legacy archive migrates once to revision zero. Sender liveness and
acknowledgement status are intentionally transient, beginning `Unavailable`
after a Chroma restart. This is an inspection surface, not a claim that this
wire is permanent.

The private daemon witness runs on a fresh session bus:
`dbus-run-session -- cargo test --lib actual_theme_dbus_service_binds_the_real_protocol_to_unique_bus_owners -- --ignored`.
Nix exposes the same witness as `checks.session-dbus`.

The daemon is named for what it manages — *chroma*, the colour
state of the display. It applies the existing **Ignis** colour
scheme from DOTOS palette data; Ignis is the palette, Chroma is
the agent that schedules and applies it.

For the agent contract, see [`AGENTS.md`](AGENTS.md). For the
system shape, see [`ARCHITECTURE.md`](ARCHITECTURE.md). For
project-specific intent and invariants, see
[`skills.md`](skills.md).
