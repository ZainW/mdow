# Command-reduced application core

`AppCore` is the only owner of product-chrome truth. GPUI renders borrowed query views and sends
`AppCommand` values. It never edits tabs, overlays, recents, session state, sidebar state, or
settings directly.

The shell also owns every operation that can touch the outside world. `dispatch` returns effects;
the shell executes them and dispatches completion commands. `PreparedDocument` and
`WorkspaceTree` cross back into the core as validated domain values, so persistence schemas,
filesystem errors, and GPUI framework objects do not leak into the reducer.

```rust
use mdow_gpui::core::{
    AppCommand, AppCore, AppDefaults, AppEffect, DocumentContentView, OpenRequest,
};

let mut core = AppCore::new(AppDefaults::mdow());
let effects = core.dispatch(AppCommand::Boot);
shell.submit(effects);

let view = core.view();
render_shell(view.chrome());
if let Some(document) = view.active_document() {
    match document.content() {
        // The existing DocumentBlock-based renderer remains unchanged.
        DocumentContentView::Markdown(prepared) => render_document(prepared, view.reader()),
        DocumentContentView::Html(html) => render_native_html(html, view.reader()),
    }
}
```

There are no `set_tabs`, `set_search_open`, `set_theme`, or `save_session` methods. A caller can
only dispatch an intent/result or ask for a read-only projection.

## Call site 1: menu and click actions

GPUI actions and controls translate directly into commands. Opening a picker does not call GPUI
from the reducer; it returns a dialog effect.

```rust
fn open_file(&mut self, _: &OpenFile, _: &mut Window, cx: &mut Context<Self>) {
    self.run(self.core.dispatch(AppCommand::OpenRequested(
        OpenRequest::FilePicker,
    )), cx);
}

fn activate_tab(&mut self, path: DocumentId, cx: &mut Context<Self>) {
    self.run(self.core.dispatch(AppCommand::ActivateTab(path)), cx);
}

fn show_settings(&mut self, cx: &mut Context<Self>) {
    self.run(self.core.dispatch(AppCommand::ShowOverlay(
        OverlayKind::Settings,
    )), cx);
}
```

`OverlayKind` is an enum and the state contains one `OverlayState`, so opening settings replaces
search or the command palette instead of producing an illegal dual-open state.

## Call site 2: native field input

The GPUI `EntityInputHandler` may retain caret, selection, and IME composition. Its committed text
is not application truth: each edit dispatches a command, and the field re-synchronizes from the
query view after every transition.

```rust
fn search_text_changed(&mut self, text: String, cx: &mut Context<Self>) {
    self.run(
        self.core
            .dispatch(AppCommand::OverlayTextChanged(text)),
        cx,
    );
}

fn render_search(&self) -> impl IntoElement {
    let OverlayView::Search(search) = self.core.view().overlay() else {
        return div().into_any_element();
    };

    self.search_input.sync_text(search.query());
    render_search_field(&self.search_input, search.matches(), search.selected_match())
}
```

The same command serves the command-palette field. The active overlay determines how the reducer
interprets the text and computes derived matches.

## Call site 3: effect completion and live reload

The shell serializes effects, performs I/O, and feeds typed results back. One open completion is
folded atomically: tabs, active selection, recents, errors, session snapshot, and desired watch set
cannot observe different versions of the operation.

```rust
fn execute_effect(&mut self, effect: AppEffect, cx: &mut Context<Self>) {
    match effect {
        AppEffect::ShowOpenDialog { request, kind } => {
            self.spawn_dialog(request, kind, cx);
        }
        AppEffect::ResolveOpenTargets { request, paths } => {
            self.spawn_open_load(request, paths, cx);
        }
        AppEffect::Persist {
            revision,
            snapshot,
        } => {
            self.persistence.enqueue_latest(revision, snapshot);
        }
        AppEffect::ReconcileWatches {
            revision,
            documents,
        } => {
            self.spawn_watch_reconciliation(revision, documents, cx);
        }
        AppEffect::OpenExternal(target) => {
            open::that(target.into_path_or_url());
        }
        other => self.execute_other_effect(other, cx),
    }
}

fn on_open_loaded(&mut self, request: RequestId, result: OpenBatchResult, cx: &mut Context<Self>) {
    self.run(
        self.core
            .dispatch(AppCommand::OpenTargetsResolved { request, result }),
        cx,
    );
}

fn on_watch_message(&mut self, document: DocumentId, cx: &mut Context<Self>) {
    self.run(
        self.core
            .dispatch(AppCommand::DocumentChanged(document)),
        cx,
    );
}
```

Every completion carries the reducer-issued `RequestId`. Stale dialog, folder scan, document load,
and reload completions are ignored by the reducer rather than racing a newer user intent.
