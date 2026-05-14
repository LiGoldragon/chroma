# Skill — chroma

*The unified visual-state daemon for the desktop. Three
independent axes, one daemon, NOTA-controlled.*

---

## What this repo is

Chroma is the **canonical owner** of the desktop's visual state:
which colour scheme is active (theme), how warm the screen is
(warmth), how bright (brightness). Anything that schedules or
applies these axes belongs in this repo; anything that only
*observes* them reads from dconf (theme) or wl-gammarelay-rs
(warmth, brightness).

The capability is *animating the visual state of the desktop*.
The three axes are coordinated by proximity in one daemon, not
by coupling of decisions. They share infrastructure (one config,
one redb, one bounded geoclue location read path, one CLI, one
socket); their scheduled events do not share fires.

---

## Invariants

These are non-negotiable; an edit that breaks them needs a
report, not a pull request.

1. **Theme apply is native concern actors.** Chroma owns
   terminal, desktop/GTK, Ghostty, and Emacs theme application
   directly. There is no configured apply command, no shell
   script boundary, and no retained legacy target schema.

2. **No global live-terminal fanout.** The daemon never scans
   `/dev/pts`, never writes OSC sequences to terminals, and
   never triggers a global terminal reload file. `SetTheme` does
   not mutate running terminals automatically; a future live
   terminal path must be explicit, per-window, and
   acknowledgement-bounded.

3. **rkyv on the wire, NOTA at the human boundary.** Daemon ↔
   CLI is the signal pattern (length-prefixed rkyv frames over
   UDS). NOTA appears only on the CLI argv, the disk config,
   and the printed reply. The daemon never re-parses NOTA from
   the CLI request frame.

4. **State lives in redb + rkyv, not ad-hoc data formats.** No
   JSON sidecars and no YAML/YML Chroma inputs. Terminal helper
   files may exist only as integration outputs for tools that
   cannot read the daemon state directly; they are not Chroma's
   source of truth.

5. **Push, not poll.** Schedule fires are Kameo delayed
   messages (timerfd-backed). Geoclue is read only when civil
   triggers need a location and retried by bounded delayed actor
   message if unavailable. Config reload is inotify. CLI
   commands are UDS frames. Property writes are zbus method
   calls. There is no loop-and-check anywhere. The carve-outs in
   `~/primary/skills/push-not-pull.md` (reachability probes,
   backpressure-aware pacing, deadline timers) are the only
   exceptions.

6. **Three axes, perfect specificity.** Each axis has its own
   typed level (`ThemeMode`, `WarmthLevel`, `BrightnessLevel`),
   its own applier actor, its own table row in redb, its own
   CLI verbs. There is no polymorphic `Set(axis, value)` enum
   that mixes concerns.

7. **The runtime root owns actor topology.** Chroma actors are
   Kameo actors. Concern actors do not create hidden runtimes
   with raw tasks, unbounded channels, or shared mutexes.
   Side effects belong to the concern actor that owns them.

8. **No `Arc<Mutex<T>>` between actors.** State is owned;
   communication is messages.

---

## What this repo does NOT own

- The Ignis colour palette's authorship. Palette data enters
  Chroma as NOTA; Chroma applies it but does not edit,
  generate, or version it.
- The wl-gammarelay-rs daemon. Chroma is its sole consumer; the
  daemon's lifecycle is owned by the home-manager systemd unit.
- The geoclue2 service. Read as the upstream authority, not
  embedded or replicated.
- The systemd graphical-session target. Chroma's user service
  declares `After=wl-gammarelay-rs.service` and
  `WantedBy=graphical-session.target`; the rest of the boot
  graph is the platform's.
- Per-app internals (Firefox, Electron, Qt). They read dconf /
  portal / GTK state; Chroma owns the desktop concern that
  updates those signals.

If a change touches one of these, the change goes upstream
(CriomOS-home, Stylix, the relevant project), not into chroma.

---

## How to work in this repo

- **Domain types first, actors last.** Land the typed level
  enums and value newtypes (with tests) before wiring the
  actors that move them.
- **Tests in `tests/`, not in `#[cfg(test)] mod tests`.** One
  test file per module exercised. Per
  `~/primary/skills/rust-discipline.md` §"Tests live in
  separate files".
- **Use existing trait domains.** `FromStr` over inherent
  `parse`. `Display` over inherent `to_string`. `From` /
  `TryFrom` for conversions. `AsRef<str>` for newtype peeks.
- **Methods on types, not free functions.** The only free
  function in either binary crate is `main`. Test helpers are
  methods on a fixture struct.
- **`nix flake check` is the canonical pre-commit runner.**
  Per `~/primary/skills/nix-discipline.md`. `cargo test` is
  fine for inner-loop iteration; the gate is `nix flake check`.
- **Push per logical commit.** Per `~/primary/skills/jj.md`.
  Each commit gets a short verb-plus-scope message and an
  immediate push.

---

## See also

- `ARCHITECTURE.md` — what the system IS.
- `AGENTS.md` — the agent contract for this repo.
- `~/primary/skills/rust-discipline.md` — methods on types,
  domain newtypes, errors, actor shape, redb + rkyv.
- `~/primary/skills/push-not-pull.md` — subscription discipline.
- `~/primary/skills/abstractions.md` — verb belongs to noun.
- `~/primary/skills/micro-components.md` — one capability per
  crate per repo.
- `~/primary/skills/jj.md` — version-control discipline.
- `~/primary/skills/nix-discipline.md` — flake hygiene.
- `HARD-CONSTRAINTS.md` — non-negotiable architecture locks
  and their tests.
- `~/primary/skills/kameo.md` — Kameo actor runtime.
- `~/primary/skills/actor-systems.md` — actor-system discipline.
- `lore/rust/rkyv.md` — wire-format discipline, feature pinning.
- `~/primary/repos/signal` — the canonical signal pattern.
- `~/primary/repos/lojix-cli` — canonical NOTA-on-argv CLI.
