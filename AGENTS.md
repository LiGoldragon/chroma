# Agent instructions — chroma

You **MUST** read `~/primary/AGENTS.md` and `lore/AGENTS.md` —
the workspace contract.

## Repo role

Chroma is the **unified visual-state daemon** for the desktop:
theme, warmth, brightness in one user service, controlled via
NOTA.

It replaces darkman + the `nightshift-*` systemd services + the
`nightshift` and `brightness` shell wrappers + the
`mkApplyScript` orchestration in `CriomOS-home`. The design
report is
`~/primary/reports/system-specialist/28-chroma-unified-visual-daemon.md`.

## Carve-outs worth knowing

- **Three independent axes.** Theme, warmth, and brightness are
  coordinated by **proximity in one daemon**, not by **coupling
  of decisions**. The axes share infrastructure (one config, one
  redb, one geoclue subscription, one CLI, one socket); their
  scheduled events do not share fires.
- **Theme apply boundary.** `chroma` invokes a configured apply
  command (a home-manager-built shell script) for theme
  switches. The daemon stays NixOS-agnostic; the apply script
  stays NixOS-specific. Don't fold dconf / GTK ini / OSC /
  emacsclient logic into the daemon.
- **Push-not-poll throughout.** Geoclue location pushes via
  zbus signal; deadlines push via `tokio::time::sleep_until`
  (timerfd-backed); inotify pushes config reloads; UDS frames
  push CLI commands; zbus pushes property writes to
  wl-gammarelay-rs. There is no `loop { check_time(); sleep(N);
  }` anywhere. See `~/primary/skills/push-not-pull.md`.
- **rkyv on the wire, NOTA at the human boundary.** Daemon ↔
  CLI is rkyv-archived `ChromaRequest` / `ChromaResponse` frames
  on a Unix socket (the canonical signal pattern from
  `~/primary/repos/signal`). The CLI parses NOTA argv into the
  typed request, archives it, and prints the rkyv-deserialised
  reply as NOTA. The daemon never re-parses NOTA from the CLI.
- **redb + rkyv for state.** Persistent state lives in
  `$XDG_STATE_HOME/chroma/state.redb`. Values are rkyv-archived
  domain records. State is read on boot and re-applied to
  hardware so resume / login / wake never drifts. See
  `~/primary/skills/rust-discipline.md` §"redb + rkyv".
- **Ractor for stateful components.** Each running concern is
  an actor with its own typed message enum (perfect-specificity
  per request kind). State is owned, not shared. The
  supervisor is the only place bare `Actor::spawn` runs; every
  child is `spawn_linked` from a parent's `pre_start`. See
  `lore/rust/ractor.md`.

## Style

Per `~/primary/skills/rust-discipline.md`:

- Methods on types, not free functions.
- Domain values are typed (newtypes; private fields).
- One object in, one object out at boundaries.
- Errors as a typed `Error` enum per crate via `thiserror`.
- Tests live in `tests/`, one file per module exercised.
- Full English words for identifiers (per
  `~/primary/skills/naming.md`).

Beauty is the criterion (per `~/primary/skills/beauty.md`):
ugliness is a diagnostic reading; slow down and find the
structure that makes it beautiful.

## Version control

`jj` (Jujutsu), per `~/primary/skills/jj.md`. Standard flow:

```sh
jj commit -m '<short verb + scope>' \
  && jj bookmark set main -r @- \
  && jj git push --bookmark main
```

Push per logical commit; blanket authorisation. No editor
prompts (always `-m '<msg>'`).

## See also

- `~/primary/AGENTS.md` — the workspace agent contract.
- `~/primary/reports/system-specialist/28-chroma-unified-visual-daemon.md`
  — the design.
- `~/primary/skills/rust-discipline.md` — Rust style and shape.
- `~/primary/skills/push-not-pull.md` — subscription discipline.
- `~/primary/skills/abstractions.md` — verb-belongs-to-noun.
- `~/primary/skills/beauty.md` — beauty as criterion.
- `lore/rust/ractor.md` — actor template.
- `lore/rust/rkyv.md` — wire format discipline.
- `~/primary/repos/signal` — canonical signal pattern reference.
- `~/primary/repos/lojix-cli` — canonical NOTA-on-argv CLI shape.
