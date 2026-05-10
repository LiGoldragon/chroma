# chroma

One Rust daemon for the colour state of the desktop — theme,
warmth, and brightness — controlled via NOTA.

`chroma` replaces darkman, the `nightshift` shell wrapper, the
three `nightshift-*` systemd services, and the `brightness` shell
wrapper with a single user-service component:

- **Three independent axes** (theme, warmth, brightness), each
  with its own schedule and applier.
- **Geolocation-driven schedules** via geoclue2 (civil dawn /
  civil dusk ± offset; per-axis ramp durations).
- **Persisted state** in redb + rkyv, crash-consistent across
  resume / login / wake.
- **One CLI** that takes a single NOTA record on argv —
  `chroma '(SetWarmth Warm)'` — and signals the daemon over a
  Unix domain socket using length-prefixed rkyv frames.
- **Native theme concerns** — Chroma owns terminal, desktop,
  Ghostty, and Emacs theme application as independent concern
  actors. `SetTheme` returns `(Accepted)` once those actors own
  the request; slow desktop or app work cannot hold terminal
  updates hostage.

The daemon is named for what it manages — *chroma*, the colour
state of the display. It applies the existing **Ignis** colour
scheme from NOTA palette data; Ignis is the palette, Chroma is
the agent that schedules and applies it.

For the design report, see
`~/primary/reports/system-specialist/28-chroma-unified-visual-daemon.md`.

For the agent contract, see [`AGENTS.md`](AGENTS.md). For the
system shape, see [`ARCHITECTURE.md`](ARCHITECTURE.md). For
project-specific intent and invariants, see
[`skills.md`](skills.md).
