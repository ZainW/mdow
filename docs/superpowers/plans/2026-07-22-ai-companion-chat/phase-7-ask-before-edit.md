# Phase 7. Ask-before-edit (follow-up)

[Overview](./overview.md)

## Goal

Allow the agent to propose file edits under explicit user approval, without opening a free shell/agent by default.

## Changes

- Advertise limited ACP filesystem write capability only in an explicit "propose edits" mode.
- On `fs/write_text_file` / edit proposals, pause and show a diff approval UI.
- Add main-process `writeFile` gated by path validation and open-folder allowlist.
- Keep shell/terminal refused until a later full-agent phase.
- Refresh watched tabs after approved writes.

## Data structures

- `EditProposal`: path, before, after, status (`pending` | `approved` | `rejected`)
- Companion mode: `ask` | `propose-edits`

## Verification

Static: refusal tests for shell. Approval/reject paths for writes.
Runtime: agent proposes an edit, user rejects (file unchanged), user approves (file updates, tab refreshes).
