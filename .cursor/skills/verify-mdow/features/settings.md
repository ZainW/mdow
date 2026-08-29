# Settings

Settings changes how markdown reads: theme, fonts, interface scale, reading width, and whether the app checks for updates.

## Sub-features

- `settings-open` opens the Settings dialog from the sidebar, keyboard, or palette.
- `settings-theme` selects System, Light, or Dark and updates the chrome.
- `settings-preview` keeps the "Tune how markdown reads." preview in view while options change.
- `settings-close` dismisses the dialog without quitting the app.

## How to get to it (user POV)

- Choose `Settings` in the sidebar footer.
- Press `⌘,` (macOS) or `Ctrl+,`.
- Run `Settings` from the command palette.
- Use the app menu Settings command.

## Driving it with verify-mdow

Preconditions:

- A verify-mdow instance is live (welcome or reader).
- The sidebar is open. If it is not, `press --key Meta+b` once and confirm the Sidebar region is visible.
- `verify-mdow doctor` reports the isolated profile.

- **Sidebar entry.** Choose Settings. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role button --name "Settings"`. A dialog named `Settings` appears with description `Tune how markdown reads.`
- **Keyboard entry.** Close the dialog (`press --key Escape`) and press the settings shortcut. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Meta+,`. The same dialog appears.
- **Palette entry.** Close the dialog, open the palette, and run Settings. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs press --key Meta+k`, `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs fill --placeholder "Search files and commands..." --value "Settings"`, and `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role option --name "Settings"`. The Settings dialog appears again.
- **Theme.** Choose Dark. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs click --role button --name "Dark"`. The Dark option is pressed. The preview and window chrome follow the dark theme. Then choose Light the same way and confirm Light is pressed.
- **Proof.** Capture Settings with Dark pressed. Run `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs screenshot --path .verify-mdow/evidence/settings/dark.png` and `node .cursor/skills/verify-mdow/scripts/verify-mdow.mjs snapshot --aria --path .verify-mdow/evidence/settings/dark.aria.txt`. Both show the Settings dialog, the description, and Dark pressed.

## Gotchas

- Theme `System` follows the OS. A System proof must say what the OS theme was, or it is not reproducible.
- Interface scale `Comfortable` and reading width `Comfortable` share a label. Scope clicks to the group (`Interface scale` vs `Reading width`) if the click hits the wrong control.
- Closing Settings must leave the document (or welcome) as it was. If the dialog close also cleared the tab, the run is invalid.
- Native app-menu Settings is not clickable here. Use the sidebar, `Meta+,`, or the palette.
