# ARCHITECTURE — chroma

Chroma is a single user-service Rust daemon that manages three
independent visual axes — **theme**, **warmth**, **brightness** —
each with its own schedule, its own applier, and its own
persisted state. The daemon replaces darkman, the
`nightshift-*` systemd services, the `nightshift` and
`brightness` shell wrappers, and the `mkApplyScript`
orchestration that lives in `CriomOS-home` today.

This file describes what the system **is** today (the shape that
ships), and the durable direction it is built toward.

## Direction — today, not eventually

Chroma is built rightly for today's desktop colour-state need on
today's stack (Rust, redb/rkyv, Unix-socket signal, zbus to
wl-gammarelay-rs and geoclue2). The daemon is named for what it
manages — the colour state of the display; it applies the existing
**Ignis** palette, it does not author it. Migration of the CLI ↔
daemon transport onto the eventual Persona fabric is future work,
not a current concern.

## Capability boundary

Chroma owns:

- the schedule across all three axes (when to apply what)
- the persisted current value per axis (in redb + rkyv)
- the typed CLI request grammar (`Request` / `Response`)
- the IPC contract between CLI and daemon (rkyv-on-UDS)
- the configuration and palette grammar (`Config`, DOTOS on disk)
- the bounded geoclue2 system-bus location read when twilight
  triggers are used
- the orchestration of ramps (start, interrupt, replace)
- the native theme application concerns: terminal, desktop/GTK,
  Ghostty, and running Pi sessions
- the resident Emacs desired-state projection on the same-user session bus

Chroma does **not** own:

- the gamma compositor — wl-gammarelay-rs remains the DBus
  daemon that talks to the wlroots compositor; chroma is its
  sole consumer
- the colour palette's authorship — Ignis is the palette;
  chroma reads it as DOTOS data and applies it, but does not
  generate, edit, or version the palette
- the geolocation source — geoclue2 is the upstream authority;
  chroma reads it directly on the system bus but does not bypass,
  replicate, or self-grant location permission

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
returns `Accepted` immediately after those actors own the
message. The terminal concern persists state for future shells
only; it never scans PTYs, writes to other terminals, or forces a
global terminal reload. The Pi concern pushes one minimal line
frame (`dark\n` or `light\n`) to each Unix-stream socket registered
under the configured runtime registry directory; stale registry entries
are removed on missing or refused sockets, and connection failures or
short timeout failures are logged by that concern without failing the
theme switch. Ghostty is a native
application concern:
Chroma reads the complete Ghostty config template for the target
mode and compares it with the mutable `config.ghostty` file under
the user config directory. Only changed content is atomically
replaced, followed by one bounded `org.gtk.Actions` `reload-config`
DBus action so existing Ghostty windows reload their own config.
Equal content causes neither a replacement nor a reload. The CLI
does not write live terminal palette sequences. Non-Ghostty terminals
converge only through a future explicit per-window protocol, or
when their own startup path reads the persisted state.
Warmth and brightness support both instant (`SetWarmth`,
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
│   └── PiThemeControlConcern
├── ThemeDbusService                 (same-user resident Emacs protocol)
│   └── ThemeOwnerWatcher            (NameOwnerChanged → Unavailable)
├── WarmthApplier                    (zbus to wl-gammarelay-rs Temperature)
│   └── generation-cancelled ramp task owned by WarmthApplier
├── BrightnessApplier                (zbus to wl-gammarelay-rs Brightness)
│   └── generation-cancelled ramp task owned by BrightnessApplier
├── ScheduleEngine                   (parsed config, next-fire deadline)
│   └── Geoclue location read when civil triggers are present
├── SleepTransitionWatcher           (logind PrepareForSleep resume push)
└── Socket accept loop               (UDS at $XDG_RUNTIME_DIR/chroma.sock)

```

Per Kameo discipline (`~/primary/skills/kameo.md` and
`~/primary/skills/actor-systems.md`): actor structs own their
state; messages are typed per kind; side-effect latency is
contained to the actor for that concern; state is not shared
through mutexes.

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

The CLI binary is a thin signal client: parse DOTOS argv into a
typed request → archive with rkyv → length-prefix → send → read
reply → bytecheck-validate → print as DOTOS. Every mutating
request returns `Accepted` after the daemon accepts ownership
of the change; theme scripts, instant gamma writes, and ramp
setup/read work continue asynchronously.

## Configuration

Single DOTOS record at `~/.config/chroma/config.dotos`. Re-parsed
on inotify push. Parses into a typed `Config`:

```
(Config
  (Theme
    (Concerns Terminal Desktop Ghostty Pi)
    (Palettes
      (Dark  (Base00 [#000000]) ... (Base0F [#ff5577]))
      (Light (Base00 [#faf5f0]) ... (Base0F [#cc3355])))
    (Adapters (Dconf <path>))
    (GhosttyConfigTemplates
      (Dark <path-to-complete-dark-ghostty-config>)
      (Light <path-to-complete-light-ghostty-config>))
    (PiThemeControl
      (RegistryDirectory (RuntimeRelative chroma/pi-live-theme.d))
      (ConnectTimeoutMillis 100)
      (WriteTimeoutMillis 100))
    (Schedule …))
  (Warmth      (Schedule …))
  (Brightness  (Schedule …)))
```

Each axis schedule is a list of `Waypoint` records + a `Default`.
Solar triggers: `(Sunrise <offset>)`, `(Sunset <offset>)`,
`(CivilDawn <offset>)`, `(CivilDusk <offset>)`. The `<offset>`
is either exact `(SignedMinutes <n>)` or a readable label:
`ExtremelyEarly`, `VeryEarly`, `Early`, `OnTime`, `Late`,
`VeryLate`, `ExtremelyLate`. Early means before the named solar
event; late means after it. Clock triggers remain
`(TimeOfDay <h> <m>)`. The geoclue read runs iff any axis uses a
solar trigger; if geolocation is unavailable, the schedule actor
retries on a bounded delayed message instead of running a polling
loop.
Daemon startup first reapplies the persisted visual state to the
appliers, then evaluates the schedule. If a civil-trigger axis has
no held or persisted location yet, that axis is left unchanged
instead of applying its configured default; defaults are not a
substitute for an unresolved civil dawn/dusk answer.
Resume from suspend is another schedule input: Chroma subscribes
to systemd-logind's `PrepareForSleep` signal and reconciles the
current wall-clock schedule immediately on the post-resume
transition. Geolocation refresh is a separate delayed flow after
resume so a slow or cold geoclue read cannot block time-of-day
theme, warmth, or brightness changes. If the fresh location differs
from the schedule actor's held location, Chroma persists it and
reconciles the schedule again using that location. Reconciliation projects
wanted state, but the root applies only values that differ from the visual state
it already owns; repeated deadline evaluation and fresh location fixes therefore
do not rewrite theme files, reload applications, restart ramps, or repeat gamma
writes when the projection is unchanged. Each schedule reconciliation increments
a generation counter so stale delayed messages from before suspend or config
reload cannot keep an old deadline chain alive.
Data-format inputs at the Chroma boundary are DOTOS; YAML and YML
inputs are rejected. `GhosttyConfigTemplates` paths are references
to complete Ghostty-native config files produced by the host
profile; Chroma does not parse them as palette data and does not
write back to those source paths.

## Persistence

`$XDG_STATE_HOME/chroma/state.redb` — one redb file. Tables:

| Table | Key | Value |
|---|---|---|
| `theme` | fixed slot `current` | rkyv archive of atomic `{ThemeMode, revision}`; theme-only old records migrate once to revision 0 |
| `warmth-state` | fixed slot `current` | rkyv archive of `StoredWarmthState` (desired target, last relay-confirmed temperature, wall-clock projection, transition state) |
| `brightness` | fixed slot `current` | rkyv archive of `BrightnessState` |
| `location` | fixed slot `last_known` | rkyv archive of `(Latitude, Longitude)` |
| `meta` | fixed slot `version` | `(schema_version, wire_version)` |

Every transition intent is one redb write transaction. Redb-write
happens **before** the hardware write so a crash mid-apply retains the
desired target without claiming that it reached hardware. Relay-confirmed
writes separately advance the applied value. A persisted active transition
is reconciled from the current wall-clock projection before it writes to the
relay after restart. An absent `warmth-state` record leaves warmth unknown;
the obsolete single-value `warmth` table is intentionally ignored rather
than interpreted as a current physical state. The
schedule engine also persists last-known geolocation after a
successful geoclue refresh so daemon startup can perform immediate
civil-trigger reconciliation from the last known position while a
fresh geolocation request runs separately. The version-skew guard at
boot hard-fails on mismatch.

## Boundary contracts

| Boundary | Format |
|---|---|
| In-process: actor ↔ actor | typed Rust values |
| Daemon ↔ CLI | rkyv-archived `Request` / `Response`, length-prefixed |
| Daemon ↔ disk (state) | rkyv values inside redb tables |
| Daemon ↔ disk (config + palettes) | DOTOS text record (`Config`) |
| Daemon ↔ Ghostty config templates | read-only Ghostty-native text files, copied byte-for-byte |
| Daemon ↔ mutable Ghostty config | `$XDG_CONFIG_HOME/ghostty/config.ghostty` |
| Daemon ↔ running Pi sessions | Unix-stream line frame (`dark\n` or `light\n`) at the configured socket |
| Daemon ↔ wl-gammarelay-rs | zbus property writes (`Temperature` u16, `Brightness` f64) |
| Daemon ↔ geoclue2 | bounded zbus system-bus location read |
| Daemon ↔ theme concerns | typed Rust values; no apply-command schema |
| Daemon ↔ human (audit) | DOTOS reply printed by the CLI |

JSON / serde appears nowhere in the daemon. The only text
format accepted as Chroma input is DOTOS (config + CLI); all
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

Ghostty is the named exception because it exposes a native
application action for config reload. Chroma reads a complete
read-only template, atomically replaces the mutable `config.ghostty`
only when its content changed, then sends that single Ghostty-owned
DBus action; it does not enumerate Ghostty windows or panes and does
not emit OSC into their PTYs.

Future improvement: replace full-template replacement with a
Ghostty config codec/parser that can update only the theme keys
while preserving unrelated mutable user settings.

## Out of scope (for the first slice)

- per-monitor warmth / brightness (wl-gammarelay-rs is per-output;
  chroma mirrors that today)
- cross-machine visual sync
- a freedesktop appearance portal hosted by chroma (apps fall
  back to dconf / GTK state set by the desktop concern)
- wallpaper as a fourth axis
- migration of CLI ↔ daemon transport off rkyv-on-UDS (Persona
  fabric is the future host)
