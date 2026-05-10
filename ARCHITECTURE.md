# ARCHITECTURE — chroma

Chroma is a single user-service Rust daemon that manages three
independent visual axes — **theme**, **warmth**, **brightness** —
each with its own schedule, its own applier, and its own
persisted state. The daemon replaces darkman, the
`nightshift-*` systemd services, the `nightshift` and
`brightness` shell wrappers, and the `mkApplyScript`
orchestration that lives in `CriomOS-home` today.

The full design and motivation is in
`~/primary/reports/system-specialist/28-chroma-unified-visual-daemon.md`.
This file describes what the system **is** today (the shape that
ships); the report describes the trade-offs and alternatives.

## Capability boundary

Chroma owns:

- the schedule across all three axes (when to apply what)
- the persisted current value per axis (in redb + rkyv)
- the typed CLI request grammar (`Request` / `Response`)
- the IPC contract between CLI and daemon (rkyv-on-UDS)
- the configuration and palette grammar (`Config`, NOTA on disk)
- the geoclue2 subscription (when twilight triggers are used)
- the orchestration of ramps (start, interrupt, replace)
- the native theme application concerns: terminal, desktop/GTK,
  Ghostty, and Emacs

Chroma does **not** own:

- the gamma compositor — wl-gammarelay-rs remains the DBus
  daemon that talks to the wlroots compositor; chroma is its
  sole consumer
- the colour palette's authorship — Ignis is the palette;
  chroma reads it as NOTA data and applies it, but does not
  generate, edit, or version the palette
- the geolocation source — geoclue2 is the upstream signal;
  chroma subscribes but does not bypass or replicate

## The three axes

| Axis | Domain values | Apply target |
|---|---|---|
| Theme | `ThemeMode { Dark, Light }` | native concern actors |
| Warmth | `WarmthLevel { Cold..Warmest }` + `KelvinTemperature` | wl-gammarelay-rs DBus `Temperature` |
| Brightness | `BrightnessLevel { Dim..Brightest }` + `BrightnessPercent` | wl-gammarelay-rs DBus `Brightness` |

Each axis has:

- its own schedule (waypoints + default)
- its own applier actor
- its own redb table (one row, current value)
- its own CLI verbs (`SetTheme`, `SetWarmth`, `StepBrightnessUp`, …)

Theme has no ramp. `SetTheme` records the requested mode,
enqueues it to one latest-wins actor per theme concern, and
returns `(Accepted)` immediately after those actors own the
message. The terminal concern persists state for future shells
only; it never scans PTYs, writes to other terminals, or forces a
global terminal reload. The CLI also does not write live terminal
palette sequences. Running terminals converge only through a
future explicit per-window protocol, or when their own startup
path reads the persisted state. Warmth and
brightness support both instant (`SetWarmth`,
`SetBrightness`) and gradual (`StartWarmthRamp`,
`StartBrightnessRamp`) transitions.

## Actor topology

```
Supervisor
├── StateStore                       (redb handle; one row per axis)
├── ThemeApplier                     (native concern fanout)
│   ├── TerminalThemeConcern
│   ├── DesktopThemeConcern
│   ├── GhosttyThemeConcern
│   └── EmacsThemeConcern
├── WarmthApplier                    (zbus to wl-gammarelay-rs Temperature)
│   └── WarmthRampSession*           (per active ramp; spawn-linked)
├── BrightnessApplier                (zbus to wl-gammarelay-rs Brightness)
│   └── BrightnessRampSession*       (per active ramp; spawn-linked)
├── ScheduleEngine                   (parsed config, next-fire deadline)
│   └── GeoclueSubscriber†           (zbus to geoclue2; spawned conditionally)
├── SocketServer                     (UDS at $XDG_RUNTIME_DIR/chroma.sock)
└── ConfigWatcher                    (inotify on config.nota)

* zero or one at any moment
† spawned only if any axis schedule uses civil-twilight triggers
```

Per ractor discipline (lore's `rust/ractor.md`): each actor's
message type is its own enum with one variant per request kind
(perfect specificity); state is owned, not shared; failures
escalate; bare `Actor::spawn` runs only at the supervisor.

## IPC shape — the canonical signal pattern

Daemon ↔ CLI is the **signal pattern** documented in
`~/primary/repos/signal`:

- Transport: Unix domain socket at
  `$XDG_RUNTIME_DIR/chroma.sock`
- Framing: 4-byte big-endian length, then the rkyv archive
- Request: `Request` enum (one variant per CLI verb)
- Reply: `Response` enum (`State(VisualState)`, `Accepted`,
  `Error(Error)`)
- Pairing: by position on the connection (FIFO)

The CLI binary is a thin signal client: parse NOTA argv into a
typed request → archive with rkyv → length-prefix → send → read
reply → bytecheck-validate → print as NOTA. Every mutating
request returns `(Accepted)` after the daemon accepts ownership
of the change; theme scripts, instant gamma writes, and ramp
setup/read work continue asynchronously.

## Configuration

Single NOTA record at `~/.config/chroma/config.nota`. Re-parsed
on inotify push. Parses into a typed `Config`:

```
(Config
  (Theme
    (Concerns Terminal Desktop Ghostty Emacs)
    (Palettes
      (Dark  (Base00 "#000000") ... (Base0F "#ff5577"))
      (Light (Base00 "#faf5f0") ... (Base0F "#cc3355")))
    (Adapters
      (Dconf <path>)
      (Emacsclient <path>))
    (Schedule …))
  (Warmth      (Schedule …))
  (Brightness  (Schedule …)))
```

Each axis schedule is a list of `Waypoint` records + a `Default`.
Triggers: `(CivilDawn (SignedMinutes <n>))`,
`(CivilDusk (SignedMinutes <n>))`, `(TimeOfDay <h> <m>)`.
The geoclue subscription opens iff any axis uses a twilight
trigger. Data-format inputs at the Chroma boundary are NOTA; YAML
and YML inputs are rejected.

## Persistence

`$XDG_STATE_HOME/chroma/state.redb` — one redb file. Tables:

| Table | Key | Value |
|---|---|---|
| `theme` | fixed slot `current` | rkyv archive of `ThemeMode` |
| `warmth` | fixed slot `current` | rkyv archive of `WarmthState` (level + custom kelvin override) |
| `brightness` | fixed slot `current` | rkyv archive of `BrightnessState` |
| `location` | fixed slot `last_known` | rkyv archive of `(Latitude, Longitude)` |
| `meta` | fixed slot `version` | `(schema_version, wire_version)` |

Every transition is one redb write transaction. Redb-write
happens **before** the hardware write so a crash mid-apply
leaves redb in the new state and the next boot reapplies. The
version-skew guard at boot hard-fails on mismatch.

## Boundary contracts

| Boundary | Format |
|---|---|
| In-process: actor ↔ actor | typed Rust values |
| Daemon ↔ CLI | rkyv-archived `Request` / `Response`, length-prefixed |
| Daemon ↔ disk (state) | rkyv values inside redb tables |
| Daemon ↔ disk (config + palettes) | NOTA text record (`Config`) |
| Daemon ↔ wl-gammarelay-rs | zbus property writes (`Temperature` u16, `Brightness` f64) |
| Daemon ↔ geoclue2 | zbus signal subscription |
| Daemon ↔ theme concerns | typed Rust values; no apply-command schema |
| Daemon ↔ human (audit) | NOTA reply printed by the CLI |

JSON / serde appears nowhere in the daemon. The only text
format accepted as Chroma input is NOTA (config + CLI); all
other daemon-owned bytes are rkyv archives.

## Forbidden Pattern: Global Live-Terminal Fanout

Do not update running terminals by enumerating `/dev/pts`, by
touching a reload file watched by every terminal window, or by
emitting OSC palette sequences from `SetTheme`. That shape turns
one user's theme command into a global WezTerm event and can
freeze unrelated agent panes. Any future live terminal update
must be an explicit per-window protocol with a bounded
acknowledgement before the next window is touched. Until that
exists, terminals converge when they start a new shell or when
their own terminal-local integration asks for an update.

## Out of scope (for the first slice)

- per-monitor warmth / brightness (wl-gammarelay-rs is per-output;
  chroma mirrors that today)
- cross-machine visual sync
- a freedesktop appearance portal hosted by chroma (apps fall
  back to dconf / GTK state set by the desktop concern)
- wallpaper as a fourth axis
- migration of CLI ↔ daemon transport off rkyv-on-UDS (Persona
  fabric is the future host)

See the design report for the rationale behind each.
