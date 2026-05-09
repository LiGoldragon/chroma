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
- the configuration grammar (`Config`, NOTA on disk)
- the geoclue2 subscription (when twilight triggers are used)
- the orchestration of ramps (start, interrupt, replace)

Chroma does **not** own:

- the GTK / dconf / Ghostty / Emacs / OSC / fzf application of a
  theme — that lives in the home-manager-built
  `chroma-apply-theme` shell script, invoked by the daemon as an
  opaque executable
- the gamma compositor — wl-gammarelay-rs remains the DBus
  daemon that talks to the wlroots compositor; chroma is its
  sole consumer
- the colour palette itself — Ignis (`ignis.yaml`,
  `ignis-light.yaml`) is the palette; chroma applies it but
  does not generate, edit, or version it
- the geolocation source — geoclue2 is the upstream signal;
  chroma subscribes but does not bypass or replicate

## The three axes

| Axis | Domain values | Apply target |
|---|---|---|
| Theme | `ThemeMode { Dark, Light }` | configured `ApplyCommand` (shell script) |
| Warmth | `WarmthLevel { Cold..Warmest }` + `KelvinTemperature` | wl-gammarelay-rs DBus `Temperature` |
| Brightness | `BrightnessLevel { Dim..Brightest }` + `BrightnessPercent` | wl-gammarelay-rs DBus `Brightness` |

Each axis has:

- its own schedule (waypoints + default)
- its own applier actor
- its own redb table (one row, current value)
- its own CLI verbs (`SetTheme`, `SetWarmth`, `StepBrightnessUp`, …)

Theme has no ramp, but its external apply script is always
spawned outside the CLI request path. `SetTheme` records the
requested mode, starts a latest-wins apply worker, and returns
`(Accepted)` immediately after the process has spawned. Warmth
and brightness support both instant (`SetWarmth`,
`SetBrightness`) and gradual (`StartWarmthRamp`,
`StartBrightnessRamp`) transitions.

## Actor topology

```
Supervisor
├── StateStore                       (redb handle; one row per axis)
├── ThemeApplier                     (apply-command path)
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
  (Theme       (ApplyCommand <path>) (Schedule …))
  (Warmth      (Schedule …))
  (Brightness  (Schedule …)))
```

Each axis schedule is a list of `Waypoint` records + a `Default`.
Triggers: `(CivilDawn (SignedMinutes <n>))`,
`(CivilDusk (SignedMinutes <n>))`, `(TimeOfDay <h> <m>)`.
The geoclue subscription opens iff any axis uses a twilight
trigger.

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
| Daemon ↔ disk (config) | NOTA text record (`Config`) |
| Daemon ↔ wl-gammarelay-rs | zbus property writes (`Temperature` u16, `Brightness` f64) |
| Daemon ↔ geoclue2 | zbus signal subscription |
| Daemon ↔ apply command | process spawn with one positional arg |
| Daemon ↔ human (audit) | NOTA reply printed by the CLI |

JSON / serde appears nowhere in the daemon. The only text
formats are NOTA (config + CLI) and the apply command's argv;
all other bytes are rkyv archives.

## Out of scope (for the first slice)

- per-monitor warmth / brightness (wl-gammarelay-rs is per-output;
  chroma mirrors that today)
- cross-machine visual sync
- a freedesktop appearance portal hosted by chroma (apps fall back
  to dconf, set by the apply command)
- wallpaper as a fourth axis
- migration of CLI ↔ daemon transport off rkyv-on-UDS (Persona
  fabric is the future host)

See the design report for the rationale behind each.
