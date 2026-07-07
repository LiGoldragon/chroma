# Agent instructions — chroma

## Repo role

Chroma is the **unified visual-state daemon** for the desktop:
theme, warmth, brightness in one user service, controlled via
NOTA.

It replaces darkman + the `nightshift-*` systemd services + the
`nightshift` and `brightness` shell wrappers + the
`mkApplyScript` orchestration in `CriomOS-home`.

## Carve-outs worth knowing

- **Three independent axes.** Theme, warmth, and brightness are
  coordinated by **proximity in one daemon**, not by **coupling
  of decisions**. The axes share infrastructure (one config, one
  redb, one bounded geoclue location read path, one CLI, one
  socket); their
  scheduled events do not share fires.
- **Native theme concerns.** `chroma` owns terminal,
  desktop/GTK, Ghostty, and Emacs theme application as
  independent concern actors. `SetTheme` returns `Accepted`
  after those actors own the message; do not make CLI requests
  wait on desktop mutation. Do not add configured apply
  commands, shell script boundaries, or retained legacy target
  schemas.
- **No global live-terminal fanout.** The daemon must not scan
  `/dev/pts`, write OSC to terminals, or touch reload files
  watched by every terminal window. `SetTheme` must not mutate
  running terminals unless a future explicit per-window protocol
  exists with bounded acknowledgement.
- **Push-not-poll throughout.** Deadlines push via Kameo delayed
  messages (timerfd-backed); inotify pushes config reloads; UDS
  frames push CLI commands; zbus pushes property writes to
  wl-gammarelay-rs. Geoclue is read only when civil triggers need
  a location and retried by bounded delayed actor message if it
  is unavailable. There is no `loop { check_time(); sleep(N); }`
  anywhere. See the push-not-pull discipline.
- **rkyv on the wire, NOTA at the human boundary.** Daemon ↔
  CLI is rkyv-archived `Request` / `Response` frames
  on a Unix socket (the canonical signal pattern from
  the `signal` repository). The CLI parses NOTA argv into the
  typed request, archives it, and prints the rkyv-deserialised
  reply as NOTA. The daemon never re-parses NOTA from the CLI.
- **NOTA-only data inputs.** Chroma config and palette data are
  NOTA. YAML/YML inputs are rejected at the Chroma boundary.
- **redb + rkyv for state.** Persistent state lives in
  `$XDG_STATE_HOME/chroma/state.redb`. Values are rkyv-archived
  domain records. State is read on boot and re-applied to
  hardware so resume / login / wake never drifts. See
  the Rust discipline §"redb + rkyv".
- **Kameo for stateful components.** Each running concern is a
  Kameo actor with typed per-kind messages. State is owned by
  the actor, not shared through locks. The runtime root owns the
  topology; concern actors never rebuild actor semantics with
  raw tasks, unbounded channels, or shared mutexes. See
  the Kameo discipline and
  the actor-system discipline.

## Style

Per the Rust discipline:

- Methods on types, not free functions.
- Domain values are typed (newtypes; private fields).
- One object in, one object out at boundaries.
- Errors as a typed `Error` enum per crate via `thiserror`.
- Tests live in `tests/`, one file per module exercised.
- Full English words for identifiers (per
  the naming discipline).

Beauty is the criterion (per the design-quality discipline):
ugliness is a diagnostic reading; slow down and find the
structure that makes it beautiful.

## Version control

`jj` (Jujutsu), per the Jujutsu discipline. Standard flow:

```sh
jj commit -m '<short verb + scope>' \
  && jj bookmark set main -r @- \
  && jj git push --bookmark main
```

Push per logical commit; blanket authorisation. No editor
prompts (always `-m '<msg>'`).

## See also

- the Rust discipline — Rust style and shape.
- the push-not-pull discipline — subscription discipline.
- the abstractions discipline — verb-belongs-to-noun.
- the design-quality discipline — beauty as criterion.
- the Kameo discipline — Kameo actor runtime.
- the actor-system discipline — actor-system discipline.
- `lore/rust/rkyv.md` — wire format discipline.
- `HARD-CONSTRAINTS.md` — architecture locks and matching tests.
- the `signal` repository — canonical signal pattern reference.
- the `lojix` repository — typed NOTA client shape.
