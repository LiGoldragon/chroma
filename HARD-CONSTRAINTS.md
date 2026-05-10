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
