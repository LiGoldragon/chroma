# INTENT — chroma

*What the psyche has explicitly intended for this project.
Synthesised from psyche statements and applicable workspace
constraints; not embellished. `ARCHITECTURE.md` says what chroma
IS; this file says what the psyche wants it to BE.*

## Purpose

`chroma` is one Rust user-service daemon for the colour state of
the desktop — **theme**, **warmth**, and **brightness** — each a
independent axis with its own schedule, applier, and persisted
state. It replaces darkman, the `nightshift-*` systemd services,
the `nightshift` and `brightness` shell wrappers, and the
`mkApplyScript` orchestration that lives in `CriomOS-home` today.
The daemon is named for what it manages — the colour state of the
display; it applies the existing **Ignis** palette, it does not
author it.

## Constraints

- **NOTA is the only text format at the chroma boundary.** The
  config (`config.nota`) and the CLI argument are NOTA; YAML/YML
  inputs are rejected; JSON and `serde` appear nowhere in the
  daemon. Every other daemon-owned byte is an rkyv archive. Per
  the workspace NOTA discipline (`primary/ESSENCE.md`,
  `primary/skills/nota-design.md`).
- **The CLI takes exactly one NOTA record on argv and signals the
  daemon.** `chroma '(SetWarmth Warm)'` — parse NOTA → rkyv
  archive → length-prefixed frame over the Unix socket → read one
  reply → print as NOTA. The CLI is a thin signal client with one
  daemon peer; it opens no database. Per the single-argument rule
  (`primary/skills/component-triad.md`).
- **Daemon ↔ CLI is the canonical signal pattern.** Unix domain
  socket, 4-byte big-endian length prefix, then the rkyv archive;
  closed `Request` / `Response` enums; FIFO pairing. The version-
  skew guard hard-fails at boot on schema mismatch.
- **State is actor-owned, never shared through mutexes.** Each
  axis has its own applier actor and its own redb table; theme
  concerns (terminal, desktop, Ghostty, Emacs) are independent
  concern actors. Per `primary/skills/kameo.md` and
  `primary/skills/actor-systems.md`.
- **Persist before the hardware write.** Every transition is one
  redb write transaction committed *before* the gamma/theme write,
  so a crash mid-apply leaves redb in the new state and the next
  boot reapplies.
- **Push, not poll.** Schedule deadlines are timer/deadline driven;
  a missing geolocation answer retries on a bounded delayed message
  rather than a polling loop. Per `primary/skills/push-not-pull.md`.

## Anti-patterns — explicitly not to do

- **No global live-terminal fanout.** Do not update running
  terminals by enumerating `/dev/pts`, by touching a reload file
  watched by every terminal window, or by emitting OSC palette
  sequences from `SetTheme` — that turns one theme command into a
  global event that can freeze unrelated agent panes. Ghostty is
  the single named exception (a complete read-only template copied
  to the mutable config, then one bounded native DBus reload
  action). Any future live-terminal update must be an explicit
  per-window protocol with a bounded acknowledgement.
- **Defaults are not a substitute for an unresolved civil
  dawn/dusk answer.** A civil-trigger axis with no held or
  persisted location is left unchanged, not forced to its
  configured default.

## Scope — today, not eventually

chroma is built rightly for today's desktop colour-state need on
today's stack (Rust, redb/rkyv, Unix-socket signal, zbus to
wl-gammarelay-rs and geoclue2). Migration of the CLI ↔ daemon
transport onto the eventual Persona fabric is future work, not a
current concern. Per `primary/ESSENCE.md` §"Today and eventually".

*Source statements live in Spirit intent records and the
project's `ARCHITECTURE.md`. Workspace-shape intent stays in
`primary/INTENT.md` and the named skills above.*
