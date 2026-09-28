use crate::{
    app::{MdowApp, UserFacingError},
    document::{is_html_document, is_supported_markdown},
    session::Recents,
    theme::{Metrics, Theme},
    ui::{
        chrome::{RecentRowStyle, recent_row},
        primitives::{action_button, border_width, brand_logo, icon, tabular_nums},
    },
};
use gpui::{AnyElement, Context, Div, FontWeight, div, prelude::*, px};
use std::path::Path;

/// Width of the welcome and error columns.
const COLUMN_WIDTH: f32 = 400.0;
const WELCOME_RECENTS: usize = 5;

/// What an external drag would open, counted once when the drag enters the window.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DropSummary {
    pub markdown: usize,
    pub html: usize,
    pub folders: usize,
}

impl DropSummary {
    pub fn from_paths<P: AsRef<Path>>(paths: impl IntoIterator<Item = P>) -> Self {
        let mut summary = Self::default();
        for path in paths {
            let path = path.as_ref();
            if path.is_dir() {
                summary.folders += 1;
            } else if is_supported_markdown(path) {
                summary.markdown += 1;
            } else if is_html_document(path) {
                summary.html += 1;
            }
        }
        summary
    }

    pub fn is_empty(self) -> bool {
        self.markdown + self.html + self.folders == 0
    }

    /// "3 Markdown files · 1 folder", or a plain refusal when nothing is openable.
    pub fn label(self) -> String {
        if self.is_empty() {
            return "Nothing Mdow can open".into();
        }
        let mut parts = Vec::new();
        let mut push = |count: usize, singular: &str, plural: &str| {
            if count > 0 {
                parts.push(format!(
                    "{count} {}",
                    if count == 1 { singular } else { plural }
                ));
            }
        };
        push(self.markdown, "Markdown file", "Markdown files");
        push(self.html, "HTML file", "HTML files");
        push(self.folders, "folder", "folders");
        parts.join(" · ")
    }
}

pub fn welcome(theme: Theme, recents: &Recents, cx: &Context<MdowApp>) -> AnyElement {
    let mut column = div()
        .debug_selector(|| "welcome".into())
        .flex()
        .flex_col()
        .w(px(theme.ui.space(COLUMN_WIDTH)))
        .max_w_full()
        .font_family(Metrics::FONT_SANS)
        .child(
            border_width(
                div()
                    .size(px(theme.ui.space(44.0)))
                    .flex_none()
                    .rounded(px(11.0))
                    .overflow_hidden()
                    .border_color(theme.border),
                0.5,
            )
            .child(brand_logo(theme.ui.space(44.0))),
        )
        .child(
            div()
                .mt(px(theme.ui.space(16.0)))
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(theme.ui.text(22.0)))
                .line_height(px(theme.ui.space(28.0)))
                .text_color(theme.foreground)
                .child("Mdow"),
        )
        .child(
            div()
                .mt(px(theme.ui.space(4.0)))
                .text_size(px(theme.ui.text(14.0)))
                .line_height(px(theme.ui.space(21.0)))
                .text_color(theme.muted_foreground)
                .child("Open a Markdown file or folder to start reading."),
        )
        .child(
            div()
                .mt(px(theme.ui.space(20.0)))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(theme.ui.space(8.0)))
                .child(action_button(
                    "welcome-open-file",
                    "Open File",
                    "icons/file.svg",
                    "⌘O",
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| this.open_file_prompt(cx)),
                ))
                .child(action_button(
                    "welcome-open-folder",
                    "Open Folder",
                    "icons/folder-open.svg",
                    "⇧⌘O",
                    false,
                    theme,
                    cx.listener(|this, _, _, cx| this.open_folder_prompt(cx)),
                )),
        );

    if !recents.is_empty() {
        let mut rows = div()
            .flex()
            .flex_col()
            .pt(px(theme.ui.space(4.0)))
            .border_t_1()
            .border_color(theme.border_subtle);
        for (index, path) in recents.iter().take(WELCOME_RECENTS).enumerate() {
            rows = rows.child(recent_row(
                ("welcome-recent", index),
                path,
                false,
                RecentRowStyle::Welcome,
                theme,
                cx,
            ));
        }
        column = column
            .child(
                div()
                    .mt(px(theme.ui.space(28.0)))
                    .mb(px(theme.ui.space(6.0)))
                    .px(px(theme.ui.space(10.0)))
                    .flex()
                    .items_center()
                    .font_weight(FontWeight::MEDIUM)
                    .text_size(px(theme.ui.text(11.0)))
                    .text_color(theme.muted_foreground)
                    .child(div().flex_grow().child("Recent"))
                    .child("⌘K to search all"),
            )
            .child(rows);
    }

    column = column.child(
        div()
            .mt(px(theme.ui.space(18.0)))
            .px(px(theme.ui.space(10.0)))
            .text_size(px(theme.ui.text(12.0)))
            .text_color(theme.muted_foreground)
            .child("Or drop files and folders anywhere in this window."),
    );

    centered(column)
}

fn centered(column: Div) -> AnyElement {
    div()
        .flex()
        .flex_grow()
        .min_w_0()
        .min_h_0()
        .items_center()
        .justify_center()
        .px(px(32.0))
        .pb(px(40.0))
        .child(column)
        .into_any_element()
}

/// Open failures share the welcome column: icon tile, title, body, path, then actions.
pub fn error_state(theme: Theme, error: &UserFacingError, cx: &Context<MdowApp>) -> AnyElement {
    let path = error.path.to_string_lossy().into_owned();
    let column = div()
        .debug_selector(|| "error-state".into())
        .flex()
        .flex_col()
        .w(px(theme.ui.space(COLUMN_WIDTH)))
        .max_w_full()
        .font_family(Metrics::FONT_SANS)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(theme.ui.space(44.0)))
                .rounded(px(11.0))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.muted)
                .child(icon(
                    "icons/alert-circle.svg",
                    theme.destructive,
                    theme.ui.space(20.0),
                )),
        )
        .child(
            div()
                .mt(px(theme.ui.space(16.0)))
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(theme.ui.text(18.0)))
                .line_height(px(theme.ui.space(24.0)))
                .text_color(theme.foreground)
                .child(error.title.clone()),
        )
        .child(
            div()
                .mt(px(theme.ui.space(4.0)))
                .text_size(px(theme.ui.text(14.0)))
                .line_height(px(theme.ui.space(21.0)))
                .text_color(theme.muted_foreground)
                .child(error.body.clone()),
        )
        .when(!path.is_empty(), |column| {
            column.child(
                div()
                    .mt(px(theme.ui.space(12.0)))
                    .px(px(theme.ui.space(10.0)))
                    .py(px(theme.ui.space(7.0)))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface_well)
                    .truncate()
                    .font_family(Metrics::FONT_MONO)
                    .text_size(px(theme.ui.text(11.0)))
                    .text_color(theme.muted_foreground)
                    .child(path),
            )
        })
        .child(
            div()
                .mt(px(theme.ui.space(20.0)))
                .flex()
                .flex_wrap()
                .gap(px(theme.ui.space(8.0)))
                .child(action_button(
                    "error-open-file",
                    "Open File",
                    "icons/file.svg",
                    "⌘O",
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| this.open_file_prompt(cx)),
                ))
                .child(action_button(
                    "error-open-folder",
                    "Open Folder",
                    "icons/folder-open.svg",
                    "⇧⌘O",
                    false,
                    theme,
                    cx.listener(|this, _, _, cx| this.open_folder_prompt(cx)),
                )),
        );
    centered(column)
}

/// Full-window drop target shown only while external paths are dragged over the window.
pub fn drop_overlay(theme: Theme, summary: DropSummary) -> AnyElement {
    let openable = !summary.is_empty();
    let tint = if openable {
        theme.primary
    } else {
        theme.muted_foreground
    };
    let target = border_width(
        div()
            .absolute()
            .top(px(48.0))
            .left(px(12.0))
            .right(px(12.0))
            .bottom(px(12.0))
            .rounded(px(12.0))
            .border_dashed()
            .border_color(tint.opacity(0.7))
            .bg(tint.opacity(0.07)),
        1.5,
    );
    div()
        .debug_selector(|| "drop-overlay".into())
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .bg(theme.background.opacity(0.46))
        .child(
            target
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(theme.ui.space(10.0)))
                .font_family(Metrics::FONT_SANS)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(theme.ui.space(44.0)))
                        .rounded(px(10.0))
                        .bg(tint.opacity(0.16))
                        .child(icon("icons/file.svg", tint, theme.ui.space(20.0))),
                )
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_size(px(theme.ui.text(14.0)))
                        .text_color(theme.foreground)
                        .child("Drop to open"),
                )
                .child(tabular_nums(
                    div()
                        .debug_selector(|| "drop-overlay-summary".into())
                        .text_size(px(theme.ui.text(12.0)))
                        .text_color(theme.muted_foreground)
                        .child(summary.label()),
                )),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn drop_summary_counts_only_what_mdow_can_open() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("notes");
        fs::create_dir(&folder).unwrap();
        let paths = vec![
            temp.path().join("a.md"),
            temp.path().join("b.markdown"),
            temp.path().join("c.MDX"),
            temp.path().join("page.html"),
            temp.path().join("image.png"),
            folder,
        ];

        let summary = DropSummary::from_paths(&paths);

        assert_eq!(
            summary,
            DropSummary {
                markdown: 3,
                html: 1,
                folders: 1,
            }
        );
        assert_eq!(summary.label(), "3 Markdown files · 1 HTML file · 1 folder");
        assert_eq!(
            DropSummary {
                markdown: 1,
                html: 0,
                folders: 2,
            }
            .label(),
            "1 Markdown file · 2 folders"
        );
        assert_eq!(
            DropSummary::from_paths([temp.path().join("image.png")]).label(),
            "Nothing Mdow can open"
        );
    }
}
