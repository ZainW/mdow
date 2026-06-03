# Settings Sidebar Footer Design

## Goal

Restore a visible in-app path to Settings after the sidebar mode tab redesign removed the old
permanent rail action. The new entry point should be discoverable, quiet, keyboard-accessible,
and should not compete with the Recents, Folder, and Outline mode tabs.

## Chosen Approach

Add a Settings icon button in a dedicated footer at the bottom of the sidebar.

This treats Settings as a global app utility instead of a fourth sidebar mode. It keeps the mode
tabs focused on document navigation, avoids crowding the sidebar header, and gives users a stable
visible target in the app chrome.

## UI Details

- Add a footer region inside the existing sidebar surface, below the sidebar content.
- Use the existing `Button` component with a ghost treatment and compact icon sizing.
- Use the `Settings` icon from `lucide-react` to match the project's current icon library.
- Provide `aria-label="Settings"` and `title="Settings"` so the icon-only button is accessible and
  discoverable.
- Add a subtle top border to separate the footer from changing sidebar content.
- Keep the control always visible while the sidebar is open; do not rely on hover-only visibility.
- Keep the app menu and `Cmd+,` / `Ctrl+,` shortcuts unchanged.

## Interaction

Clicking or keyboard-activating the footer button opens the existing `SettingsDialog` by calling
`setSettingsOpen(true)` from the app store.

No new settings state, routing, or dialog behavior is needed.

## Accessibility And Polish

- The icon button must be reachable by keyboard tabbing.
- The button must have a visible focus state via the existing `Button` focus styling.
- The footer height should be stable so switching sidebar modes does not cause layout shift.
- The control should use a ghost style so it remains lower priority than active content and tabs.
- No new animation is needed for this frequently used product control.

## Testing

- Update sidebar tests to expect the new Settings button in the sidebar.
- Add or update a test that clicking Settings sets `settingsOpen` to `true`.
- Keep the existing tests that verify the old rail actions are gone, but distinguish Settings as the
  new footer utility rather than an old rail action.
- Run targeted sidebar tests after implementation, then project verification before completion.

## Non-Goals

- Do not reintroduce the old permanent action rail.
- Do not add Settings as a fourth Recents/Folder/Outline mode.
- Do not change the Settings dialog contents.
- Do not remove existing menu or keyboard shortcut access.
