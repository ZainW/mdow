# Usage — the reader engine

Candidate C4. The load-bearing surface is the document itself: one `Reader` entity per open
document that owns parsing, incremental reload, shaping, selection, find, and copy. Product
chrome (tabs, sidebar, prefs, overlays) is typed but thin, and stays outside the reader.

## Quickstart

```rust
use mdow_gpui::reader::{Reader, ReaderEvent, ReaderStyle, Step};

// One reader per open document. It owns its own scroll, selection, and find state.
let reader = cx.new(|cx| Reader::open(path, source, DocKind::Markdown, cx));

// The app decides what a link *means*; the reader decides what a link *is*.
cx.subscribe(&reader, |this, _, event, cx| match event {
    ReaderEvent::ActivateLink(target) => this.follow(target, cx),
    ReaderEvent::OutlineChanged => cx.notify(),
    ReaderEvent::MatchesChanged => cx.notify(),
})
.detach();

// Style is derived from preferences + theme, never stored twice. Pushing the same
// style twice is a no-op.
reader.update(cx, |reader, cx| {
    reader.set_style(ReaderStyle::resolve(&prefs, theme), cx);
});
```

Mounting it is one child:

```rust
div().flex_grow().min_h_0().child(reader.clone())
```

Everything the Electron reader loop does inside that surface — wrapping inline text, syntax
highlighting, code copy, link hover/focus/activation, the custom scrollbar, cross-block text
selection, copy, find-in-document with `N of M` and highlight-and-reveal — is behind that one
child. The app never sees a block, an atom, a text run, or a glyph rect.

### What the app still owns

| App owns                                               | Reader owns                                            |
| ------------------------------------------------------ | ------------------------------------------------------ |
| Which document is active, tab order, close policy      | Document content and everything derived from it        |
| What a link activation _does_ (new tab / `open::that`) | What a link _is_ (markdown / web / local file / inert) |
| Preferences, theme resolution, persistence             | Style application and invalidation                     |
| Find bar chrome (input, buttons, keybindings)          | Query matching, match ordering, highlight, reveal      |
| Workspace tree, recents, command palette               | Outline entries and heading reveal                     |

---

## Call site 1 — `MdowApp::render` mounts the active reader

The whole reader surface is one child. Note what is _absent_ compared to today: no
`copied_code`, no `hovered_link`, no `focused_link`, no `reader_scroll_handles`, no
`reader_link_focus_handles`, no `reader_scrollbar_drag`, no `ReaderLinkState`, no
per-frame focus-handle reconciliation.

```rust
impl Render for MdowApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.resolve(window.appearance());
        let layout = ShellLayout::for_width(
            f32::from(window.viewport_size().width),
            self.chrome.sidebar.open,
            self.prefs.reading_width,
        );

        let content = match self.tabs.active() {
            None => self.welcome(theme),
            Some(tab) => {
                // Idempotent: only bumps the run cache when the style actually changed.
                let style = ReaderStyle::resolve(&self.prefs, theme);
                tab.reader.update(cx, |reader, cx| reader.set_style(style, cx));
                tab.reader.clone().into_any_element()
            }
        };

        shell(theme, layout)
            .child(self.chrome.tab_bar(theme, cx))
            .child(self.chrome.breadcrumb(theme, cx))
            .child(content)
            // Find is a non-modal layer over the reader; a modal is exclusive by construction.
            .children(self.chrome.find_bar(self.tabs.active(), theme, cx))
            .children(self.chrome.modal(theme, cx))
    }
}
```

Escape has exactly one meaning, and it lives in one place:

```rust
fn dismiss(&mut self, _: &Dismiss, _: &mut Window, cx: &mut Context<Self>) {
    // Closes the modal if one is open, else the find bar, else nothing. Idempotent.
    if self.chrome.dismiss() {
        if let Some(tab) = self.tabs.active() {
            tab.reader.update(cx, |reader, cx| reader.set_query(None, cx));
        }
        cx.notify();
    }
}
```

---

## Call site 2 — the watcher reloads a file under the user's cursor

This is the call site that justifies the whole data structure. The user has a 4,000-line
document open, is scrolled two-thirds down, has `deprecated` in the find bar showing
`17 of 43`, and has a paragraph selected. An external editor saves the file with a one-word
change in the middle.

```rust
fn on_file_changed(&mut self, path: &Path, cx: &mut Context<Self>) {
    let Some(tab) = self.tabs.get(path) else { return };
    match load_source(path) {
        Ok(loaded) => tab.reader.update(cx, |reader, cx| {
            // Diffs old and new source at stable block boundaries, reparses only the
            // changed span, and splices. Blocks whose bytes are unchanged keep their
            // BlockId, so scroll offset, selection anchors, and find matches outside
            // the spliced span all survive. Matches inside it are recomputed.
            reader.reload(loaded.source, cx);
        }),
        Err(error) => tab.set_reload_error(error.body()),
    }
}
```

`reload` is the whole API. There is no "invalidate the highlight cache," no "re-collect the
link focus targets," no "rebuild the outline," no "re-run the search." Those are all derived
from the compiled document, so splicing the document updates them by construction.

Reloading with byte-identical source is a no-op that touches nothing.

---

## Call site 3 — find-in-document and copy

The find bar is app chrome. It owns a text field and keybindings; it owns no matching logic.

```rust
fn open_find(&mut self, _: &Find, window: &mut Window, cx: &mut Context<Self>) {
    let seed = self.tabs.active().and_then(|tab| tab.reader.read(cx).selection_text());
    self.chrome.open_find(seed, window, cx);
    cx.notify();
}

fn on_find_query_changed(&mut self, query: SharedString, cx: &mut Context<Self>) {
    let Some(tab) = self.tabs.active() else { return };
    tab.reader.update(cx, |reader, cx| {
        // Debounced internally by 120ms, matching Electron. Recomputes hits, keeps the
        // current match if it survives, and reveals it. Emits MatchesChanged.
        reader.set_query(Some(query), cx);
    });
}

fn step_match(&mut self, direction: Step, cx: &mut Context<Self>) {
    if let Some(tab) = self.tabs.active() {
        tab.reader.update(cx, |reader, cx| reader.step_match(direction, cx));
    }
}
```

The bar renders `N of M` straight off the reader — no second count to keep in sync:

```rust
let label = match reader.match_status() {
    None => SharedString::default(),                       // empty query
    Some(status) if status.total == 0 => "No results".into(),
    Some(status) => format!("{} of {}", status.index + 1, status.total).into(),
};
```

Copy is the same coordinate space as find, so it is three lines:

```rust
fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
    if let Some(text) = self.tabs.active().and_then(|t| t.reader.read(cx).selection_text()) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}

fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
    if let Some(tab) = self.tabs.active() {
        tab.reader.update(cx, |reader, cx| reader.select_all(cx));
    }
}
```

Selecting from the middle of a heading, through a list, across a code block, into the
following paragraph, and pressing `cmd-c` yields the visible text with block breaks
preserved. That works because selection spans, find hits, and copy all address the same
`(atom, byte range)` coordinates.

### Jumping from the outline sidebar

```rust
fn reveal_heading(&mut self, heading: HeadingId, cx: &mut Context<Self>) {
    if let Some(tab) = self.tabs.active() {
        tab.reader.update(cx, |reader, cx| {
            reader.reveal(RevealTarget::Heading(heading), cx);
        });
    }
}
```

The sidebar's outline list is `reader.outline()` — the same `Vec<Outline>` the compile pass
already produced, not a second traversal.
