# HARD CONSTRAINTS — chroma

These are architecture locks. A change that weakens one needs a
design report first, not compatibility code.

## HC-CHROMA-001 — Native Theme Concerns

Theme application is owned by Chroma concern actors. There is no
configured apply command and no shell-script theme boundary.

Test: `hc_chroma_001_apply_command_records_are_rejected_not_interpreted`.

## HC-CHROMA-002 — No Removed-Schema Compatibility

Removed config schemas fail loudly. Chroma does not retain old
`ApplyTargets`, `Legacy`, or migration interpretation paths.

Tests:

- `hc_chroma_002_apply_targets_records_are_rejected_not_migrated`
- `hc_chroma_003_legacy_theme_concern_is_rejected_not_retained`

## HC-CHROMA-003 — NOTA-Only Data Inputs

Configuration and palette data inputs are NOTA. YAML/YML inputs
are invalid at the Chroma boundary.

Test: `hc_chroma_004_yaml_data_inputs_are_rejected_in_favor_of_nota`.

## HC-CHROMA-004 — No Global Live-Terminal Fanout

Chroma may persist terminal theme state for future shells. It
must not scan `/dev/pts`, write OSC sequences to terminals, or
trigger global terminal reload files from the daemon or CLI.
Running terminals are not mutated automatically by `SetTheme`.
Any future live terminal update path must be an explicit
per-window protocol with bounded acknowledgement before the next
window is touched.

Tests:

- `hc_chroma_005_terminal_concern_does_not_broadcast_to_pty_inventory`
- `hc_chroma_006_daemon_does_not_trigger_global_terminal_reload_files`
- `hc_chroma_007_cli_does_not_emit_live_terminal_palette_sequences`

## HC-CHROMA-005 — Kameo Actor Runtime, No Hand-Rolled Task Actors

Runtime concerns are Kameo actors. Chroma must not rebuild an
actor runtime out of raw `tokio::spawn`, unbounded mpsc channels,
`AbortHandle` cancellation slots, or shared mutex state.

Tests:

- `hc_chroma_008_runtime_uses_kameo_not_hand_rolled_task_actors`
- `hc_chroma_009_runtime_has_no_shared_mutex_between_actors`

## HC-CHROMA-006 — Ghostty Native Concern, No Shell Reload Path

Ghostty application reads complete read-only Ghostty config
templates selected by mode, copies the selected template to the
mutable user config file `config.ghostty`, and asks the running
Ghostty application to reload through Ghostty's `org.gtk.Actions`
DBus action `reload-config`. The template path may be a Nix store
path; Chroma never writes to that path. There is no shell script,
`systemctl` command, systemd service reload, OSC palette path, or
retained WezTerm reload path in Chroma.

Test: `hc_chroma_010_ghostty_concern_uses_native_config_and_gtk_action_reload`.

## HC-CHROMA-007 — Config Reload Is Filesystem Push, Not Polling

Config reload is owned by a Kameo actor backed by the platform
filesystem notification API. Chroma must not add a config polling
loop or a shell watcher.

Test: `hc_chroma_011_config_reload_uses_push_watcher_not_polling_loop`.

## HC-CHROMA-008 — GeoClue Is A System-Bus Boundary

GeoClue is the platform location authority on the system bus.
Chroma must read it through `zbus::Connection::system`; location
permission remains platform-owned through the host GeoClue
application configuration. Chroma must not ask the session bus for
`org.freedesktop.GeoClue2`.

Test: `hc_chroma_012_geoclue_uses_system_bus`.
