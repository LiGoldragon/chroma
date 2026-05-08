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
one redb, one geoclue subscription, one CLI, one socket); their
scheduled events do not share fires.

The design context lives in
`~/primary/reports/system-specialist/28-chroma-unified-visual-daemon.md`.
Read it before substantive changes — it carries the
trade-offs against the alternative (separate `warmth` and
`darkman` daemons) and the case for unification.

---

## Invariants

These are non-negotiable; an edit that breaks them needs a
report, not a pull request.

1. **Theme apply lives outside the daemon.** Chroma invokes a
   configured `ApplyCommand` shell script for theme switches;
   it does not embed dconf / GTK / Ghostty / OSC / Emacs
   knowledge. The script is NixOS-specific; the daemon is not.

2. **rkyv on the wire, NOTA at the human boundary.** Daemon ↔
   CLI is the signal pattern (length-prefixed rkyv frames over
   UDS). NOTA appears only on the CLI argv, the disk config,
   and the printed reply. The daemon never re-parses NOTA from
   the CLI request frame.

3. **State lives in redb + rkyv, not text files.** No
   `~/.local/state/chroma/current-mode` text file; no JSON
   sidecar; no flat-file logs as durable state. The
   `~/.local/state/darkman/current-mode` and `fzf-theme.sh`
   files are written by the *apply command* (because the zsh
   init hook reads them), not by the daemon.

4. **Push, not poll.** Schedule fires are `tokio::time::sleep_until`
   deadlines (timerfd-backed). Geoclue is a zbus signal
   subscription. Config reload is inotify. CLI commands are UDS
   frames. Property writes are zbus method calls. There is no
   loop-and-check anywhere. The carve-outs in
   `~/primary/skills/push-not-pull.md` (reachability probes,
   backpressure-aware pacing, deadline timers) are the only
   exceptions.

5. **Three axes, perfect specificity.** Each axis has its own
   typed level (`ThemeMode`, `WarmthLevel`, `BrightnessLevel`),
   its own applier actor, its own table row in redb, its own
   CLI verbs. There is no polymorphic `Set(axis, value)` enum
   that mixes concerns.

6. **The supervisor is the only `spawn` site.** Every other
   actor is `spawn_linked` from its parent's `pre_start`.
   Failures escalate.

7. **No `Arc<Mutex<T>>` between actors.** State is owned;
   communication is messages.

---

## What this repo does NOT own

- The Ignis colour palette (`ignis.yaml`, `ignis-light.yaml`).
  Lives in `CriomOS-home` and `Stylix`. Chroma applies it via
  the apply command; it does not edit, generate, or version
  the YAML.
- The wl-gammarelay-rs daemon. Chroma is its sole consumer; the
  daemon's lifecycle is owned by the home-manager systemd unit.
- The geoclue2 service. Subscribed, not embedded.
- The systemd graphical-session target. Chroma's user service
  declares `After=wl-gammarelay-rs.service` and
  `WantedBy=graphical-session.target`; the rest of the boot
  graph is the platform's.
- Per-app theme adapters (Firefox, Electron, Qt). They read
  dconf / portal; the apply command writes dconf.

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
- `~/primary/reports/system-specialist/28-chroma-unified-visual-daemon.md`
  — the design report.
- `~/primary/skills/rust-discipline.md` — methods on types,
  domain newtypes, errors, ractor, redb + rkyv.
- `~/primary/skills/push-not-pull.md` — subscription discipline.
- `~/primary/skills/abstractions.md` — verb belongs to noun.
- `~/primary/skills/micro-components.md` — one capability per
  crate per repo.
- `~/primary/skills/jj.md` — version-control discipline.
- `~/primary/skills/nix-discipline.md` — flake hygiene.
- `lore/rust/ractor.md` — actor template, perfect-specificity
  messages, supervision.
- `lore/rust/rkyv.md` — wire-format discipline, feature pinning.
- `~/primary/repos/signal` — the canonical signal pattern.
- `~/primary/repos/lojix-cli` — canonical NOTA-on-argv CLI.
