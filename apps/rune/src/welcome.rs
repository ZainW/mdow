//! The empty tab: the logo, Open and Open Folder, and the most recent documents as cards.

use crate::app::{App, Cmd};
use crate::files;
use crate::icons;
use crate::theme::{self, Palette};
use rune::{El, Tx, col, icon, row, text};

fn action(
    key: &str,
    glyph: &'static rune::Icon,
    title: &str,
    keys: &str,
    cmd: Cmd,
    p: &Palette,
) -> El {
    row()
        .key(key.to_string())
        .w(220.0)
        .h(40.0)
        .gap(10.0)
        .center_y()
        .px(14.0)
        .radius(9.0)
        .fill(p.elevated)
        .border(1.0, p.line)
        .hover(p.hover)
        .role("button")
        .label(title.to_string())
        .focusable(true)
        .child(icon(glyph, 16.0).color(p.accent))
        .child(text(title.to_string()).font(theme::ui(13.0, 500.0)).color(p.text).grow(1.0))
        .child(text(keys.to_string()).font(theme::ui(11.5, 500.0)).color(p.tertiary))
        .on_press_tx(move |s: &mut App, tx: &mut Tx| s.run(cmd.clone(), tx))
}

pub fn view(s: &App, p: &Palette, _k: &rune_kit::Kit) -> El {
    let mut recent = col().w(452.0).gap(6.0);
    let items: Vec<_> = s.prefs.recents.iter().filter(|p| p.exists()).take(6).cloned().collect();
    if !items.is_empty() {
        recent = recent.child(
            text("RECENT")
                .font(rune::TextStyle { tracking: 0.5, ..theme::ui(10.5, 600.0) })
                .color(p.tertiary)
                .pb(2.0),
        );
        let mut grid = row().w_full().flex_wrap().gap(8.0);
        for (i, path) in items.into_iter().enumerate() {
            let dir = path.is_dir();
            let name = files::display_name(&path);
            let detail = files::truncate_middle(&files::display_dir(&path), 34);
            grid = grid.child(
                col()
                    .key(format!("welcome-recent-{i}"))
                    .w(222.0)
                    .gap(3.0)
                    .px(12.0)
                    .py(10.0)
                    .radius(9.0)
                    .fill(p.elevated)
                    .border(1.0, p.line)
                    .hover(p.hover)
                    .role("button")
                    .label(name.clone())
                    .help(path.display().to_string())
                    .child(
                        row()
                            .gap(7.0)
                            .center_y()
                            .child(
                                icon(if dir { &icons::FOLDER } else { &icons::DOC }, 14.0)
                                    .color(p.tertiary),
                            )
                            .child(
                                text(name)
                                    .font(theme::ui(13.0, 500.0))
                                    .color(p.text)
                                    .truncate()
                                    .min_w(0.0)
                                    .grow(1.0),
                            ),
                    )
                    .child(text(detail).font(theme::ui(11.0, 400.0)).color(p.tertiary).truncate())
                    .on_press_tx(move |s: &mut App, tx: &mut Tx| {
                        s.open_paths(std::slice::from_ref(&path), tx)
                    })
                    .transition(rune::Transition::opacity()),
            );
        }
        recent = recent.child(grid);
    }
    col()
        .key("welcome")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .center()
        .gap(22.0)
        .child(
            col()
                .items_center()
                .gap(10.0)
                .child(icon(&icons::LOGO_MARK, 56.0).color(p.accent))
                .child(text("Mdow").font(theme::ui(22.0, 650.0)).color(p.text))
                .child(
                    text("A quiet place to read markdown.")
                        .font(theme::ui(13.0, 400.0))
                        .color(p.secondary),
                ),
        )
        .child(
            row()
                .gap(12.0)
                .child(action(
                    "welcome-open",
                    &icons::DOC,
                    "Open…",
                    crate::platform::label("⌘O"),
                    Cmd::OpenDialog,
                    p,
                ))
                .child(action(
                    "welcome-folder",
                    &icons::FOLDER,
                    "Open Folder…",
                    crate::platform::label("⇧⌘O"),
                    Cmd::OpenFolderDialog,
                    p,
                )),
        )
        .child(recent)
        .child(
            text("Or drop a file or folder onto the window.")
                .font(theme::ui(11.5, 400.0))
                .color(p.tertiary),
        )
}
