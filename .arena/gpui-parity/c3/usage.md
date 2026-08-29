# Adopt gpui-component 0.5.1 on gpui 0.2.2

Pin `gpui-component = "=0.5.1"` from crates.io. Leave `gpui = "=0.2.2"`. Do not take longbridge git HEAD. HEAD's getting-started page already switched to `gpui` plus `gpui_platform` from the Zed repo, which is a different GPUI than the crate Mdow locked.

Call `gpui_component::init(cx)` as the first line of `Application::run`. Overwrite the crate's default palette before the first frame, using Mdow's Electron HSLA values. Markdown stays on `pulldown-cmark` → `DocumentBlock` → the existing reader. HTML documents go through `TextView::html`. Find, command palette, settings, and the three sidebar modes use Input, List, Dialog, and the Theme global.

The public API the rest of the app learns is `boot`, `overlay`, `open`, and `theme_bridge`. Callers do not construct `ThemeColor`, implement `ListDelegate`, or call `TextView::markdown`.

## What you depend on

```toml
gpui = "=0.2.2"
gpui-component = "=0.5.1"
notify = "=8.2.0"
```

Leave the `webview` and `tree-sitter-languages` features off. Skip `gpui-component-assets`. Mdow already ships Inter, Geist Mono, and Lucide SVGs.

`gpui-component 0.5.1` declares `gpui = "^0.2.2"` and `notify = "^7.0.0"`. Cargo keeps Mdow's notify 8.2.0 for `FileWatcher` and adds a second notify 7.x for the crate. Two copies in the lockfile. One watcher implementation.

## Boot

```rust
Application::new()
    .with_assets(MdowAssets::new(asset_root))
    .run(move |cx: &mut App| {
        boot::install(cx);
        cx.text_system().add_fonts(fonts).expect("register Mdow fonts");
        // existing keybindings and menus
        cx.open_window(window_options, |window, cx| {
            cx.new(|cx| {
                let mut app = MdowApp::new(window, cx);
                if let Some(path) = launch_path.as_deref() {
                    app.open_path(path, cx);
                }
                app
            })
        })
        .expect("open Mdow window");
        cx.activate(true);
    });
```

`boot::install` calls `gpui_component::init`, then `theme_bridge::install` with the compact light tokens. That avoids one frame of Longbridge teal on a warm Mdow window. `MdowApp::new` resolves `ThemePreference` against `window.appearance()` and calls `theme_bridge::sync` again.

`MdowApp::render` still owns the shell. It appends `Root::render_dialog_layer` so settings and the palette can open. Existing `#[gpui::test]` windows that construct `MdowApp` directly keep working once they call `boot::install_for_test(cx)` first.

## Call site 1. Open Markdown or HTML

```rust
pub fn open_path(&mut self, path: &Path, cx: &mut Context<Self>) {
    match open::classify(path) {
        open::OpenKind::Markdown => {
            if let Err(error) = self.model.open_markdown(path) {
                self.open_error = Some(error.into_view());
            } else {
                self.watch_active_document();
                self.clear_reader_transient_state();
            }
        }
        open::OpenKind::Html => {
            if let Err(error) = self.model.open_html(path) {
                self.open_error = Some(error.into_view());
            } else {
                self.watch_active_document();
                self.clear_reader_transient_state();
            }
        }
        open::OpenKind::Folder => {
            self.model.open_workspace(path).ok();
        }
        open::OpenKind::Unsupported => {
            self.open_error = Some(UserFacingError::unsupported(path));
        }
    }
    cx.notify();
}
```

`open_markdown` is today's `load_source` + `parse_document` + `prepare_document`. The 132 parser, highlight, tab, workspace, token, and reader-selector tests stay on that path.

`open_html` reads UTF-8, runs `html::prepare`, and stores `OpenDocument::Html`. The reader matches on the active tab and calls `html_reader::render` only in that arm. `TextView::markdown` is never used.

## Call site 2. Find, then settings, never both

```rust
fn find_in_document(&mut self, _: &FindInDocument, window: &mut Window, cx: &mut Context<Self>) {
    self.overlay.show(OverlayKind::Find, window, cx);
}

fn open_settings(&mut self, _: &OpenSettings, window: &mut Window, cx: &mut Context<Self>) {
    self.overlay.show(OverlayKind::Settings, window, cx);
}

fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    theme_bridge::sync(&self.prefs, window.appearance(), cx);
    let chrome = self.chrome.render(window, cx);
    div()
        .id("mdow-root")
        .size_full()
        .child(chrome)
        .children(self.overlay.render_find_bar(window, cx))
        .children(gpui_component::Root::render_dialog_layer(window, cx))
}
```

`Overlay` is one enum. Showing settings closes find. Showing the palette closes settings. Electron's four independent booleans cannot be represented.

Find is a compact bar that hosts `Input::new(&self.chrome.find_input)`. Settings and the command palette call `window.open_dialog`. The dialog body is Mdow form copy. The crate supplies focus trap, dim, and Escape.

## Call site 3. Sidebar modes through List

```rust
fn set_sidebar_mode(&mut self, mode: SidebarMode, cx: &mut Context<Self>) {
    self.chrome.sidebar.set_mode(mode, &self.model, cx);
    self.prefs.sidebar_mode = mode;
    cx.notify();
}

// inside Chrome::render, only the active list is in the tree
match self.sidebar.mode() {
    SidebarMode::Folder => List::new(&self.sidebar.folder),
    SidebarMode::Recents => List::new(&self.sidebar.recents),
    SidebarMode::Outline => List::new(&self.sidebar.outline),
}
```

Each mode has its own `Entity<ListState<_>>`. Delegates stay private in `chrome/lists.rs`. Row height is `Metrics` compact (same 24 px slot the current tree uses). Indent is padding inside `render_item`, not extra height. Confirm on a file opens it. Confirm on a directory toggles expand and rebuilds visible rows.

## Theme rule

Mdow tokens stay the named colors tests already assert.

```rust
let tokens = MdowTokens::resolve(self.prefs.theme, window.appearance());
assert_eq!(tokens.colors.background, hsla(0.08672199, 0.39970066, 0.97152986, 1.0));
theme_bridge::sync(&self.prefs, window.appearance(), cx);
// Input, List, Dialog now paint those same HSLAs via cx.theme()
```

`theme_bridge` copies the 13 Electron colors onto the matching `ThemeColor` fields, then derives hover, active, and unused chart slots from those values so a default Longbridge accent cannot leak. Fonts on the Theme global are Inter Variable, Geist Mono, and the compact 13 px chrome size, scaled by `InterfaceScale`.

## HTML rule

```rust
html_reader::render(&prepared_html, &tokens, window, cx)
// expands to:
TextView::html("html-document", prepared_html.display_source.clone(), window, cx)
    .selectable(true)
    .scrollable(true)
    .style(html_reader::style(&tokens))
```

`TextView` in 0.5.1 is a basic-tag reader. No CSS, no script. That is the product. Electron's sandboxed iframe is the thing we are not rebuilding.

`html::prepare` rewrites relative `src` and `href` against the document parent, strips `script` / `iframe` / `object` / `embed`, and drops `on*` attributes before the string reaches `TextView`. Local images become absolute paths. http(s) links stay. Local `.md` links in HTML are a known weak point. 0.5.1's `TextView` has no `on_link` callback.

## What you do not call

- `TextView::markdown`. That renderer is not the 132-test contract.
- `Theme::sync_system_appearance` from app code. Preference resolution belongs to `theme_bridge::sync`.
- `gpui-component` `TabBar` for document tabs. The measured 36 / 28 / 4 tab rail stays custom GPUI.
- The crate `webview` feature.
