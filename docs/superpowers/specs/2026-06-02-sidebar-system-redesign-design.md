# Sidebar System Redesign

Date: 2026-06-02

## Goal

Redesign Mdow's sidebar so it feels like one quiet navigation surface instead of a narrow
activity rail plus a second drawer. The app should keep fast access to Recents, Folder, and
Outline, but the document remains the primary visual focus.

## Current Problem

The current sidebar has two adjacent surfaces:

- A 36px left rail with three sidebar mode icons plus global actions.
- A 244px drawer that displays the selected mode's content.

This works functionally, but the rail and drawer read as two separate sidebars. The rail also
mixes two jobs: switching sidebar content and launching global app actions.

## Selected Direction

Use a single persistent sidebar with compact tabs in the sidebar header.

The sidebar keeps the existing three content modes:

- Recents
- Folder
- Outline

The mode switcher moves from the vertical icon rail into a compact horizontal tab control at the
top of the sidebar. Each tab uses a small icon plus text when space allows. The active state uses a
soft muted background and foreground color change, not a filled primary color.

Global actions leave the left edge:

- Quick Open remains available through the command palette shortcut. Do not add a replacement
  button in this iteration.
- Open File remains available through existing menu/shortcut flows and the welcome screen. Do not
  add a replacement button in this iteration.
- Open Folder remains available through existing menu/shortcut flows, drag-and-drop, the welcome
  screen, and the Folder empty state.
- Settings stays in the app menu and existing settings shortcut/dialog flow. Do not add a
  replacement button in this iteration.

The sidebar itself remains toggleable through the existing `sidebarOpen` state and keyboard/menu
bindings.

## Layout

Desktop layout:

- Remove the permanent icon rail.
- Keep one left sidebar with approximately the current drawer width.
- The sidebar header contains a compact tab switcher for Recents, Folder, and Outline.
- Sidebar content below the switcher reuses the existing `RecentsList`, `FolderTree`, and
  `OutlineList` content.
- The main document region starts immediately after the sidebar border.

Collapsed layout:

- When the sidebar is closed, the entire sidebar width becomes zero as it does today.
- The app should not leave behind a rail-only mode.
- Reopening restores the last selected sidebar mode.

Empty states:

- Folder mode with no open folder keeps the "No folder open" state and should include the Open
  Folder action.
- Recents with no entries keeps the existing empty state.
- Outline with no headings keeps the current no-headings/no-document messaging.

## Interaction

Mode switching:

- The tab control is a keyboard-accessible radiogroup or tablist.
- Arrow keys move focus between Recents, Folder, and Outline.
- Activating a tab updates the existing `sidebarMode` store state and persists it through the
  existing app-state save path.

Sidebar visibility:

- Existing menu and keyboard handlers continue to call `toggleSidebar`.
- Closing the sidebar hides all three modes.
- Reopening shows the last active mode.

Global actions:

- Do not keep Open File, Open Folder, Quick Open, and Settings as a permanent vertical rail.
- Prefer command palette and contextual controls over persistent sidebar chrome.
- The first implementation should not add new top-level action buttons. If user testing shows an
  action became too hidden, add one compact action menu in existing document chrome as a follow-up.

## Visual Style

- One surface, one border: the sidebar has a single right border against the document area.
- The tab control should be compact and calm, matching the app's existing 13px UI scale.
- Active tab state should use muted backgrounds and text color, not a primary-color filled pill.
- Use Lucide icons already used by the app, sized at 16px or smaller.
- Avoid nested cards in the sidebar. Lists should remain flat with subtle hover/active states.

## Components

Expected implementation changes:

- `Sidebar.tsx`
  - Remove the rail container and rail-specific indicator helpers.
  - Replace `RailModeIcon` with a horizontal `SidebarModeTabs` component.
  - Remove rail-only global action buttons.
  - Keep existing mode rendering and content components.

- `Sidebar.test.tsx`
  - Update assertions from "rail radiogroup with three options" to the new header mode control.
  - Preserve keyboard navigation tests for mode switching.

- Store
  - Reuse `sidebarOpen`, `sidebarMode`, `toggleSidebar`, and `setSidebarMode`.
  - No new state is required for the selected design.

Out of scope for the first implementation:

- Adding a compact app actions menu in `TabBar` or document chrome.
- Moving Outline to the right side of the document.
- Adding popover side panels.

## Rejected Alternatives

Polished current system:

- Lowest implementation risk.
- Still keeps the core two-surface problem.

Collapsed navigator with popover panels:

- Strong document focus.
- Poorer for sustained folder browsing and harder to implement well with the current file tree.

Document toolbar with right outline:

- Good reading model for outline-heavy documents.
- Larger behavioral departure and likely too much for this iteration.

## Testing

Run the project verification relevant to this change:

- `pnpm run --filter desktop test -- -t Sidebar`
- `pnpm run test`
- `pnpm run typecheck`
- `pnpm run lint`
- `pnpm run fmt:check`

Manual verification:

- Sidebar open and closed states render without a leftover rail.
- Recents, Folder, and Outline modes switch with mouse and keyboard.
- Folder empty state still exposes a clear Open Folder path.
- Existing menu/keyboard sidebar toggle still works.
- Light and dark themes preserve readable active, hover, and focus states.
