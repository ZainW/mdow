# Usage — GPUI Electron-parity chrome (candidate c1)

The caller here is `MdowApp` (the thin shell) and `main.rs`. Everything below is what
those two files get to see. Four new modules carry the weight:

| Module     | One-line contract                                                                                                       |
| ---------- | ----------------------------------------------------------------------------------------------------------------------- |
| `prefs`    | Typed preference values. Illegal combinations don't exist. Pure; no IO.                                                 |
| `session`  | Snapshot of "what was open" — tabs, active tab, folder, recents, window bounds.                                         |
| `persist`  | The only code that touches disk or strings. `load()` never fails; `save()` is atomic.                                   |
| `overlay`  | One slot. At most one of find / palette / settings / shortcuts exists, ever.                                            |
| `ui/field` | A one-line text input entity (`EntityInputHandler`, from gpui 0.2.2 `examples/input.rs`), reusable by find and palette. |

The quickstart, as a consumer reads it:

```rust
// Load once at launch. Never fails — corrupt or missing state degrades field-by-field
// to defaults, and the Electron-era `wideMode`/`readingWidth` pair is merged into one
// typed value during the parse.
let store = StateStore::open_default();
let Restored { prefs, session } = store.load();

// Prefs are plain enums and newtypes. There is no string in sight and no way to hold
// "wide mode on" and "reading width standard" as two divergent facts.
assert_eq!(prefs.theme_mode, ThemeMode::System);
let width: Option<Pixels> = prefs.reader_width.max_width(); // None = full-bleed

// Overlays: one slot. Opening the palette while find is up replaces find. Escape or a
// second toggle closes. Focus returns to where it was.
app.toggle_overlay(OverlayKind::Palette, window, cx);

// Every preference mutation goes through one funnel that also persists:
app.apply_pref(PrefEdit::Theme(ThemeMode::Dark), cx);
```

---

## Call site 1 — `main.rs`: launch, restore, window bounds

```rust
fn main() -> anyhow::Result<()> {
    // ... asset discovery unchanged ...

    // Boundary: one read, before any window exists. Window bounds come from the
    // session so the window opens where it closed.
    let store = StateStore::open_default();
    let Restored { prefs, session } = store.load();

    Application::new().with_assets(assets).run(move |cx: &mut App| {
        cx.bind_keys([
            // existing bindings, plus:
            KeyBinding::new("cmd-f", ToggleFind, None),
            KeyBinding::new("cmd-k", TogglePalette, None),
            KeyBinding::new("cmd-,", ToggleSettings, None),
            KeyBinding::new("cmd-/", ToggleShortcuts, None),
            KeyBinding::new("escape", Dismiss, None),
            KeyBinding::new("cmd-=", ZoomIn, None),
            KeyBinding::new("cmd--", ZoomOut, None),
            KeyBinding::new("cmd-0", ZoomReset, None),
            // Field editing keys are scoped to the Field key context, so they never
            // shadow reader shortcuts:
            KeyBinding::new("left", field::MoveLeft, Some("Field")),
            KeyBinding::new("cmd-a", field::SelectAll, Some("Field")),
            // ... rest of the Field keymap ...
        ]);

        let bounds = session
            .window
            .map(WindowBounds::from_saved)
            .unwrap_or_else(|| WindowBounds::Windowed(Bounds::centered(None, size(px(1120.0), px(760.0)), cx)));

        cx.open_window(options_with(bounds), |window, cx| {
            cx.new(|cx| {
                // MdowApp swallows the whole restored state. Restore is tolerant:
                // moved/deleted session tabs are skipped silently, recents keep them.
                let mut app = MdowApp::new(prefs, store, window, cx);
                app.restore_session(session, cx);
                if let Some(path) = launch_path.as_deref() {
                    app.open_path(path, cx); // CLI arg wins over restored active tab
                }
                app
            })
        });
    });
    Ok(())
}
```

What the caller never sees: JSON keys, the `wideMode` migration, atomic temp-file
renames, clamping of out-of-range zoom values.

## Call site 2 — `app.rs`: cmd-F, typing, enter, escape

```rust
impl MdowApp {
    fn toggle_find(&mut self, _: &ToggleFind, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_overlay(OverlayKind::Find, window, cx);
    }

    /// The only overlay construction site. `OverlayHost` guarantees exclusivity and
    /// focus restore; this method only decides which entity to build and which
    /// domain events to route.
    fn toggle_overlay(&mut self, kind: OverlayKind, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlays.kind() == Some(kind) {
            self.overlays.close(window);
            cx.notify();
            return;
        }
        let opened = match kind {
            OverlayKind::Find => {
                let doc = self.model.tabs.active().map(|tab| tab.document.clone());
                let find = cx.new(|cx| FindOverlay::new(doc, window, cx));
                let events = cx.subscribe_in(&find, window, Self::on_find_event);
                OpenOverlay::find(find, events)
            }
            OverlayKind::Palette => {
                let palette =
                    cx.new(|cx| PaletteOverlay::new(self.model.recents.clone(), window, cx));
                let events = cx.subscribe_in(&palette, window, Self::on_palette_event);
                OpenOverlay::palette(palette, events)
            }
            // Settings and Shortcuts analogous.
            _ => unimplemented!(),
        };
        self.overlays.open(opened, window);
        cx.notify();
    }

    // FindOverlay owns the query Field, the match list, and the cursor. The app only
    // hears about the things it must act on: "scroll here" and "close me".
    fn on_find_event(&mut self, _: &Entity<FindOverlay>, event: &FindEvent,
                     window: &mut Window, cx: &mut Context<Self>) {
        match event {
            FindEvent::ActiveHit(hit) => self.scroll_reader_to_block(hit.block, cx),
            FindEvent::Dismissed => { self.overlays.close(window); cx.notify(); }
        }
    }
}

// In render: highlights are *derived* from the overlay, not synced into app state.
// Closing find removes them because the state they came from is gone.
let find_matches = self.overlays.find().map(|find| find.read(cx).matches());
surface.child(render_document(document, reader_style, self.theme, self.copied_code,
                              &link_state, find_matches, &scroll_handle, cx))
```

One more obligation on the app: when the active document changes (tab switch, reload,
open), it calls the single funnel `self.active_document_changed(cx)`, which retargets
an open find overlay and persists the session. That funnel already exists in spirit —
it replaces today's scattered `clear_reader_transient_state()` call sites.

## Call site 3 — palette runs "Theme: Dark", pref persists, render follows

```rust
fn on_palette_event(&mut self, _: &Entity<PaletteOverlay>, event: &PaletteEvent,
                    window: &mut Window, cx: &mut Context<Self>) {
    match event {
        PaletteEvent::Invoked(action) => {
            self.overlays.close(window);
            match action {
                PaletteAction::Run(id) => self.run_command(*id, window, cx),
                PaletteAction::Open(path) => self.open_path(path, cx),
            }
        }
        PaletteEvent::Dismissed => { self.overlays.close(window); cx.notify(); }
    }
}

fn run_command(&mut self, id: CommandId, window: &mut Window, cx: &mut Context<Self>) {
    match id {
        CommandId::ThemeDark => self.apply_pref(PrefEdit::Theme(ThemeMode::Dark), cx),
        CommandId::ToggleWideMode => self.apply_pref(PrefEdit::ToggleFull, cx),
        CommandId::SidebarOutline => self.apply_pref(PrefEdit::Sidebar(SidebarMode::Outline), cx),
        CommandId::OpenFile => self.open_file_prompt(cx),
        // ... every palette command routes to an existing method or a PrefEdit ...
    }
}

/// One funnel for every preference change, from the settings panel, the palette,
/// menu items, or keybindings. Mutation and persistence cannot separate:
/// `StoredPrefs::apply` writes through to disk when the value actually changed.
fn apply_pref(&mut self, edit: PrefEdit, cx: &mut Context<Self>) {
    if self.prefs.apply(edit, &self.session_snapshot()) {
        self.overlays.refresh_settings(self.prefs.get(), cx);
        cx.notify();
    }
}

// Render start — theme is derived every frame from (mode, appearance). There is no
// stored "current theme" to fall out of sync:
self.theme = Theme::resolve(self.prefs.get().theme_mode, window.appearance());
let layout = ShellLayout::for_width(
    f32::from(window.viewport_size().width),
    self.sidebar_open,
    self.prefs.get().reader_width,
    self.prefs.get().interface_scale,
);
```

Note what `ToggleWideMode` does now: `PrefEdit::ToggleFull` flips
`ReaderWidth::Column(w)` ⇄ `ReaderWidth::Full { returns_to: w }`. Toggling twice from
Comfortable lands back on Comfortable — the memory lives in the type, not in a second
boolean that someone has to keep synchronized.

## Sidebar modes (small but load-bearing)

```rust
// render_sidebar switches on one enum; each arm reads a different *derived* source.
match self.prefs.get().sidebar_mode {
    SidebarMode::Recents => sidebar_recents(&self.model.recents, ...),
    SidebarMode::Folder  => sidebar_folder(self.model.workspace.as_ref(), ...),   // existing tree
    SidebarMode::Outline => sidebar_outline(active_document.map(|d| &d.headings), ...), // derived, never stored
}
```

Outline rows call the same `scroll_reader_to_block` the find overlay uses.
