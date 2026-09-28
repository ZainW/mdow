//! Large-document pipeline: off-thread opens, progressive previews, reload diffing, scroll
//! anchors, heading jumps and the virtualized outline.

use super::*;
use crate::{anchor::ScrollAnchor, prefs::SidebarMode, tabs::TabLoad};
use gpui::{ListOffset, TestAppContext, VisualTestContext, px};
use std::{fs, path::PathBuf, sync::Arc, time::Duration};

fn blank_window(cx: &mut TestAppContext) -> gpui::WindowHandle<MdowApp> {
    cx.update(|cx| {
        cx.open_window(Default::default(), |window, cx| {
            cx.new(|cx| {
                MdowApp::boot_with_watcher(
                    Prefs::default(),
                    StateStore::in_memory(),
                    SessionRole::Owner,
                    Err(anyhow::anyhow!("no watcher in tests")),
                    window,
                    cx,
                )
            })
        })
        .unwrap()
    })
}

fn redraw(visual: &mut VisualTestContext) {
    visual.update(|window, cx| window.draw(cx).clear());
}

fn paragraphs(count: usize, label: &str) -> String {
    (0..count)
        .map(|index| {
            format!("{label} paragraph {index} has enough words to wrap in the column.\n\n")
        })
        .collect()
}

fn pane(app: &MdowApp, path: &Path) -> Entity<ReaderPane> {
    app.reader_panes.get(path).unwrap().clone()
}

fn active_path(app: &MdowApp) -> PathBuf {
    app.model.tabs.active().unwrap().path().to_owned()
}

#[gpui::test]
fn large_files_open_a_tab_before_their_parse_finishes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.md");
    let source = format!("# Large\n\n{}", paragraphs(6_000, "Body"));
    assert!(source.len() as u64 >= ASYNC_PARSE_MIN_BYTES);
    fs::write(&path, &source).unwrap();
    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);

    window
        .update(&mut visual, |app, _, cx| app.open_path(&path, cx))
        .unwrap();
    // The tab is there at once, as a placeholder: nothing has been parsed on this thread.
    window
        .update(&mut visual, |app, _, _| {
            let tab = app.model.tabs.active().expect("tab opens immediately");
            assert!(matches!(tab.load, TabLoad::Loading { .. }));
            assert!(tab.document.blocks.is_empty());
            assert_eq!(tab.document.title, "large.md");
            assert!(app.load_tasks.contains_key(tab.path()));
        })
        .unwrap();
    redraw(&mut visual);
    // A fast parse never flashes the loading line.
    assert!(visual.debug_bounds("reader-loading-document").is_none());

    visual.run_until_parked();
    window
        .update(&mut visual, |app, _, _| {
            let tab = app.model.tabs.active().unwrap();
            assert_eq!(tab.load, TabLoad::Ready);
            assert_eq!(tab.document.title, "Large");
            assert_eq!(tab.document.blocks.len(), 6_001);
            assert!(app.load_tasks.is_empty());
        })
        .unwrap();
    redraw(&mut visual);
    assert!(visual.debug_bounds("reader-block-0").is_some());
}

#[gpui::test]
fn small_files_still_open_synchronously(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("small.md");
    fs::write(&path, "# Small\n\nText.\n").unwrap();
    let window = blank_window(cx);
    window
        .update(cx, |app, _, cx| {
            app.open_path(&path, cx);
            let tab = app.model.tabs.active().unwrap();
            assert_eq!(tab.load, TabLoad::Ready);
            assert_eq!(tab.document.title, "Small");
            assert!(app.load_tasks.is_empty());
        })
        .unwrap();
}

#[gpui::test]
fn the_loading_line_waits_until_a_parse_is_slow(cx: &mut TestAppContext) {
    // Test frames keep earlier debug bounds, so each case gets its own window.
    let show = |cx: &mut TestAppContext, since: Instant| {
        let window = blank_window(cx);
        let mut visual = VisualTestContext::from_window(*window, cx);
        window
            .update(&mut visual, |app, _, cx| {
                app.model
                    .tabs
                    .open_loading(Path::new("/tmp/pending-large.md"), since);
                cx.notify();
            })
            .unwrap();
        redraw(&mut visual);
        visual.debug_bounds("reader-loading-document").is_some()
    };
    assert!(!show(cx, Instant::now()), "a fresh parse shows nothing yet");
    assert!(show(cx, Instant::now() - Duration::from_secs(1)));
}

#[gpui::test]
fn a_failed_background_read_closes_the_placeholder_and_reports(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vanishing.md");
    fs::write(&path, paragraphs(6_000, "Gone")).unwrap();
    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    window
        .update(&mut visual, |app, _, cx| app.open_path(&path, cx))
        .unwrap();
    fs::remove_file(&path).unwrap();
    visual.run_until_parked();
    window
        .update(&mut visual, |app, _, _| {
            assert!(app.model.tabs.is_empty());
            assert_eq!(
                app.open_error.as_ref().map(|error| error.title.as_str()),
                Some("File not found")
            );
        })
        .unwrap();
}

#[gpui::test]
fn a_preview_swaps_to_the_full_document_without_moving_the_reader(cx: &mut TestAppContext) {
    let source = format!("# Huge\n\n{}", paragraphs(20_000, "Huge"));
    let head = slice_document_head(&source).expect("large enough to preview");
    let path = PathBuf::from("/tmp/huge-preview.md");
    let preview = PreparedDocument::preview(parse_document(path.clone(), head.to_owned()));
    let full = prepare_document(parse_document(path.clone(), source.clone()));
    let preview_blocks = preview.blocks.len();
    assert!(
        preview.headings.is_empty(),
        "outline waits for the full document"
    );
    assert_eq!(preview.blocks[..], full.blocks[..preview_blocks]);

    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let generation = window
        .update(&mut visual, |app, _, _| {
            let generation = app.model.tabs.open_loading(&path, Instant::now());
            assert!(app.model.tabs.apply_loaded(preview, generation));
            generation
        })
        .unwrap();
    redraw(&mut visual);
    let list = window
        .update(&mut visual, |app, _, cx| {
            let tab = app.model.tabs.active().unwrap();
            assert_eq!(tab.load, TabLoad::Preview);
            pane(app, tab.path()).read(cx).list_state()
        })
        .unwrap();
    // Every preview block plus the "Loading the rest of the document…" row.
    assert_eq!(list.item_count(), preview_blocks + 1);
    list.scroll_to(ListOffset {
        item_ix: preview_blocks - 1,
        offset_in_item: px(0.0),
    });
    redraw(&mut visual);
    assert!(visual.debug_bounds("reader-loading-rest").is_some());
    list.scroll_to(ListOffset {
        item_ix: 40,
        offset_in_item: px(7.0),
    });
    redraw(&mut visual);
    let before = list.logical_scroll_top();

    window
        .update(&mut visual, |app, _, cx| {
            assert!(app.model.tabs.apply_loaded(full, generation));
            cx.notify();
        })
        .unwrap();
    redraw(&mut visual);
    let after = list.logical_scroll_top();
    assert_eq!(
        (after.item_ix, after.offset_in_item),
        (before.item_ix, before.offset_in_item)
    );
    assert_eq!(list.item_count(), 20_001);
    assert!(visual.debug_bounds("reader-loading-rest").is_none());
    window
        .update(&mut visual, |app, _, _| {
            let tab = app.model.tabs.active().unwrap();
            assert_eq!(tab.load, TabLoad::Ready);
            assert_eq!(tab.document.headings.len(), 1);
        })
        .unwrap();
}

#[gpui::test]
fn reloads_splice_changed_blocks_and_keep_measured_heights(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("reload.md");
    let body = paragraphs(300, "Kept");
    fs::write(&path, format!("# Reload\n\n{body}")).unwrap();
    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    window
        .update(&mut visual, |app, _, cx| app.open_path(&path, cx))
        .unwrap();
    redraw(&mut visual);
    let path = window
        .read_with(&visual, |app, _| active_path(app))
        .unwrap();
    let list = window
        .update(&mut visual, |app, _, cx| {
            pane(app, &path).read(cx).list_state()
        })
        .unwrap();
    list.scroll_to(ListOffset {
        item_ix: 120,
        offset_in_item: px(10.0),
    });
    redraw(&mut visual);
    let measured = list.max_offset_for_scrollbar().height;
    assert!(measured > px(0.0));

    // Three paragraphs land between paragraphs 10 and 11: no other block's margins change.
    let (early, late) = body.split_at(body.find("Kept paragraph 11 ").unwrap());
    fs::write(
        &path,
        format!("# Reload\n\n{early}Inserted one.\n\nInserted two.\n\nInserted three.\n\n{late}"),
    )
    .unwrap();
    window
        .update(&mut visual, |app, window, cx| {
            app.model.reload_path(&path).unwrap();
            let tab = app.model.tabs.active().unwrap();
            let (document, load) = (tab.document.clone(), tab.load);
            // Sync the pane directly so nothing is re-measured before we look.
            let style = app.prefs.get().reader_style();
            let theme = app.theme;
            pane(app, &path).update(cx, |pane, cx| pane.sync(document, load, style, theme, cx));
            let _ = window;
        })
        .unwrap();
    // Unchanged blocks kept their measured heights (a reset would drop them all to zero), and
    // the reader still looks at the same paragraph, now three blocks further down.
    assert_eq!(list.max_offset_for_scrollbar().height, measured);
    let top = list.logical_scroll_top();
    assert_eq!((top.item_ix, top.offset_in_item), (123, px(10.0)));
    assert_eq!(list.item_count(), 304);
    redraw(&mut visual);
    let top = list.logical_scroll_top();
    assert_eq!((top.item_ix, top.offset_in_item), (123, px(10.0)));
}

#[gpui::test]
fn a_reader_at_the_top_stays_there_when_content_is_inserted_above(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("top.md");
    let body = paragraphs(50, "Body");
    fs::write(&path, format!("# Top\n\n{body}")).unwrap();
    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    window
        .update(&mut visual, |app, _, cx| app.open_path(&path, cx))
        .unwrap();
    redraw(&mut visual);
    fs::write(&path, format!("New first line.\n\n# Top\n\n{body}")).unwrap();
    window
        .update(&mut visual, |app, _, cx| {
            app.handle_watch_messages(vec![WatchMessage::Reload(path.clone())], cx)
        })
        .unwrap();
    redraw(&mut visual);
    let path = window
        .read_with(&visual, |app, _| active_path(app))
        .unwrap();
    let top = window
        .update(&mut visual, |app, _, cx| {
            pane(app, &path).read(cx).list_state().logical_scroll_top()
        })
        .unwrap();
    assert_eq!((top.item_ix, top.offset_in_item), (0, px(0.0)));
}

#[gpui::test]
fn large_reloads_parse_off_thread_and_keep_the_reading_position(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large-reload.md");
    let body = paragraphs(6_000, "Body");
    fs::write(&path, format!("# Large\n\n{body}")).unwrap();
    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    window
        .update(&mut visual, |app, _, cx| app.open_path(&path, cx))
        .unwrap();
    visual.run_until_parked();
    redraw(&mut visual);
    let path = window
        .read_with(&visual, |app, _| active_path(app))
        .unwrap();
    let list = window
        .update(&mut visual, |app, _, cx| {
            pane(app, &path).read(cx).list_state()
        })
        .unwrap();
    list.scroll_to(ListOffset {
        item_ix: 3_000,
        offset_in_item: px(4.0),
    });
    redraw(&mut visual);

    fs::write(&path, format!("# Large\n\nA new opening line.\n\n{body}")).unwrap();
    let before = window
        .update(&mut visual, |app, _, cx| {
            let before = app.model.tabs.active().unwrap().document.clone();
            app.handle_watch_messages(vec![WatchMessage::Reload(path.clone())], cx);
            // Still showing the last version while the new one parses.
            let tab = app.model.tabs.active().unwrap();
            assert!(Arc::ptr_eq(&tab.document, &before));
            assert!(app.load_tasks.contains_key(&path));
            before
        })
        .unwrap();
    visual.run_until_parked();
    redraw(&mut visual);
    window
        .update(&mut visual, |app, _, _| {
            let tab = app.model.tabs.active().unwrap();
            assert!(!Arc::ptr_eq(&tab.document, &before));
            assert_eq!(tab.document.blocks.len(), 6_002);
        })
        .unwrap();
    let top = list.logical_scroll_top();
    assert_eq!((top.item_ix, top.offset_in_item), (3_001, px(4.0)));
}

#[gpui::test]
fn relaunch_restores_the_saved_reading_position(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.md");
    let body = paragraphs(200, "Saved");
    let source = format!("# Session\n\n{body}");
    fs::write(&path, &source).unwrap();
    let path = path.canonicalize().unwrap();
    let document = prepare_document(parse_document(path.clone(), source));
    let anchor = ScrollAnchor::capture(80, 12.0, &document.layout().signatures).unwrap();
    // The file gained two blocks above the saved position since the last launch.
    fs::write(&path, format!("# Session\n\nNew.\n\nAlso new.\n\n{body}")).unwrap();
    let session = Session::from_parts(
        vec![path.clone()],
        Some(path.clone()),
        None,
        Recents::default(),
        None,
    )
    .with_anchors(HashMap::from([(path.clone(), anchor)]));

    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    window
        .update(&mut visual, |app, _, cx| app.restore_session(session, cx))
        .unwrap();
    visual.run_until_parked();
    redraw(&mut visual);
    let (top, saved) = window
        .update(&mut visual, |app, _, cx| {
            let top = pane(app, &path).read(cx).list_state().logical_scroll_top();
            (top, app.session_snapshot().anchors.get(&path).copied())
        })
        .unwrap();
    assert_eq!((top.item_ix, top.offset_in_item), (82, px(12.0)));
    let saved = saved.expect("the reading position is part of the session");
    assert_eq!((saved.block, saved.signature), (82, anchor.signature));
}

#[gpui::test]
fn outline_jumps_land_exactly_on_nested_headings(cx: &mut TestAppContext) {
    let source = format!(
        "# Intro\n\n{}- first item\n\n  ## Nested target\n\n  More item text.\n\n{}> [!NOTE]\n> ### Callout target\n\n{}## Plain target\n\n{}",
        paragraphs(120, "Lead"),
        paragraphs(60, "Middle"),
        paragraphs(60, "Late"),
        paragraphs(60, "Tail"),
    );
    let document = parse_document(PathBuf::from("/tmp/jumps.md"), source);
    let nested_path = document.heading_path(1).unwrap();
    let callout_path = document.heading_path(2).unwrap();
    let plain_path = document.heading_path(3).unwrap();
    assert_eq!(nested_path.len(), 2, "nested inside a list item");
    assert_eq!(callout_path.len(), 2, "nested inside a callout");
    assert_eq!(plain_path.len(), 1);
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |window, cx| {
            cx.new(|cx| {
                let mut app = MdowApp::new(window, cx);
                app.model.tabs.open(document);
                app
            })
        })
        .unwrap()
    });
    let mut visual = VisualTestContext::from_window(*window, cx);
    redraw(&mut visual);
    for (heading, path) in [(1, nested_path), (2, callout_path), (3, plain_path)] {
        window
            .update(&mut visual, |app, _, cx| app.jump_to_heading(heading, cx))
            .unwrap();
        for _ in 0..6 {
            redraw(&mut visual);
        }
        let suffix = path
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("-");
        let selector: &'static str = Box::leak(format!("reader-block-{suffix}").into_boxed_str());
        let heading_top = visual
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} is painted"))
            .top();
        let viewport_top = visual.debug_bounds("reader-scroll").unwrap().top();
        let gap = f32::from(heading_top - viewport_top);
        assert!(
            (gap - crate::ui::reader::HEADING_JUMP_MARGIN).abs() <= 1.0,
            "heading {heading} landed {gap}px below the viewport top"
        );
        assert_eq!(
            window
                .update(&mut visual, |app, _, cx| app.active_outline_heading(cx))
                .unwrap(),
            Some(heading)
        );
    }
    // Contents links use the same exact jump.
    window
        .update(&mut visual, |app, _, cx| {
            app.activate_link(Path::new("/tmp/jumps.md"), "#nested-target", cx)
        })
        .unwrap();
    for _ in 0..6 {
        redraw(&mut visual);
    }
    assert_eq!(
        window
            .update(&mut visual, |app, _, cx| app.active_outline_heading(cx))
            .unwrap(),
        Some(1)
    );
}

#[gpui::test]
fn the_outline_renders_only_its_visible_rows(cx: &mut TestAppContext) {
    let source = (0..3_000)
        .map(|index| format!("## Heading {index}\n\nText.\n\n"))
        .collect::<String>();
    let document = parse_document(PathBuf::from("/tmp/outline.md"), source);
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |window, cx| {
            cx.new(|cx| {
                let mut app = MdowApp::new(window, cx);
                app.model.tabs.open(document);
                app.set_sidebar_mode(SidebarMode::Outline, cx);
                app
            })
        })
        .unwrap()
    });
    let mut visual = VisualTestContext::from_window(*window, cx);
    redraw(&mut visual);
    assert!(visual.debug_bounds("outline-row-0").is_some());
    let painted = (0..3_000)
        .filter(|index| {
            let selector: &'static str = Box::leak(format!("outline-row-{index}").into_boxed_str());
            visual.debug_bounds(selector).is_some()
        })
        .count();
    assert!(painted < 80, "outline painted {painted} of 3000 rows");

    // Jumping deep into the document scrolls the active row into the virtualized list.
    window
        .update(&mut visual, |app, _, cx| app.jump_to_heading(2_500, cx))
        .unwrap();
    for _ in 0..6 {
        redraw(&mut visual);
    }
    assert!(visual.debug_bounds("outline-row-2500").is_some());
}

#[gpui::test]
fn diagrams_and_math_keep_their_place_across_a_reload(cx: &mut TestAppContext) {
    let section = |index: usize| {
        format!(
            "Paragraph {index} before a diagram.\n\n```mermaid\ngraph TD\n  A{index} --> B{index}\n```\n\n$$\nx_{index}^2 + y^2 = z^2\n$$\n\n"
        )
    };
    let body = (0..60).map(section).collect::<String>();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graphics.md");
    fs::write(&path, format!("# Graphics\n\n{body}")).unwrap();
    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    window
        .update(&mut visual, |app, _, cx| app.open_path(&path, cx))
        .unwrap();
    redraw(&mut visual);
    // Diagrams and math rasterize in the background and grow from their placeholder height.
    visual.run_until_parked();
    redraw(&mut visual);
    let path = window
        .read_with(&visual, |app, _| active_path(app))
        .unwrap();
    let list = window
        .update(&mut visual, |app, _, cx| {
            pane(app, &path).read(cx).list_state()
        })
        .unwrap();
    list.scroll_to(ListOffset {
        item_ix: 91,
        offset_in_item: px(20.0),
    });
    redraw(&mut visual);
    visual.run_until_parked();
    redraw(&mut visual);
    let signature = window
        .read_with(&visual, |app, _| {
            app.model
                .tabs
                .active()
                .unwrap()
                .document
                .layout()
                .signatures[91]
        })
        .unwrap();

    fs::write(
        &path,
        format!("# Graphics\n\nA new paragraph.\n\n$$\na + b\n$$\n\n{body}"),
    )
    .unwrap();
    window
        .update(&mut visual, |app, _, cx| {
            app.handle_watch_messages(vec![WatchMessage::Reload(path.clone())], cx)
        })
        .unwrap();
    visual.run_until_parked();
    redraw(&mut visual);
    let top = list.logical_scroll_top();
    assert_eq!((top.item_ix, top.offset_in_item), (93, px(20.0)));
    window
        .read_with(&visual, |app, _| {
            let layout = app.model.tabs.active().unwrap().document.layout();
            assert_eq!(layout.signatures[93], signature);
        })
        .unwrap();
}

#[gpui::test]
fn a_selection_follows_unchanged_blocks_through_a_reload(cx: &mut TestAppContext) {
    use crate::ui::text_surface::{SurfaceId, TextPoint, TextSelection};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("selection.md");
    let body = paragraphs(40, "Kept");
    fs::write(&path, format!("# Selection\n\n{body}")).unwrap();
    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    window
        .update(&mut visual, |app, _, cx| app.open_path(&path, cx))
        .unwrap();
    redraw(&mut visual);
    let path = window
        .read_with(&visual, |app, _| active_path(app))
        .unwrap();
    let pane = window
        .read_with(&visual, |app, _| pane(app, &path))
        .unwrap();
    let selection = TextSelection {
        anchor: TextPoint::new(SurfaceId::new(20, 0), 5),
        head: TextPoint::new(SurfaceId::new(22, 0), 9),
    };
    let expected = visual.update(|_, cx| {
        pane.update(cx, |pane, _| {
            pane.set_selection(Some(selection));
            pane.selected_text()
        })
    });
    assert!(expected.is_some());

    let reload = |visual: &mut VisualTestContext, source: String| {
        fs::write(&path, source).unwrap();
        window
            .update(visual, |app, _, cx| {
                app.handle_watch_messages(vec![WatchMessage::Reload(path.clone())], cx)
            })
            .unwrap();
        redraw(visual);
    };
    // Two blocks inserted above: the selection moves with its text.
    reload(
        &mut visual,
        format!("# Selection\n\nNew one.\n\nNew two.\n\n{body}"),
    );
    let (moved, text) = visual.update(|_, cx| {
        let pane = pane.read(cx);
        (pane.selection(), pane.selected_text())
    });
    assert_eq!(moved.unwrap().anchor.block, 22);
    assert_eq!(moved.unwrap().head.block, 24);
    assert_eq!(text, expected);

    // Editing the block an end sits in clears the selection: its offset no longer names the
    // same text.
    let edited = body.replace("Kept paragraph 19 ", "Edited paragraph 19 ");
    reload(
        &mut visual,
        format!("# Selection\n\nNew one.\n\nNew two.\n\n{edited}"),
    );
    assert_eq!(visual.update(|_, cx| pane.read(cx).selection()), None);
}

#[gpui::test]
fn split_panes_keep_separate_reading_positions_and_load_off_thread(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let left = dir.path().join("left.md");
    let right = dir.path().join("right.md");
    fs::write(&left, format!("# Left\n\n{}", paragraphs(6_000, "Left"))).unwrap();
    fs::write(
        &right,
        format!("# Right\n\n## Deep\n\n{}", paragraphs(300, "Right")),
    )
    .unwrap();
    let window = blank_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    window
        .update(&mut visual, |app, _, cx| {
            app.open_path(&right, cx);
            app.open_path(&left, cx);
            app.toggle_split_view(cx);
        })
        .unwrap();
    visual.run_until_parked();
    redraw(&mut visual);
    let (left, right) = (left.canonicalize().unwrap(), right.canonicalize().unwrap());
    let (left_list, right_list) = window
        .update(&mut visual, |app, _, cx| {
            assert!(app.split.is_enabled());
            assert_eq!(app.model.tabs.get(&left).unwrap().load, TabLoad::Ready);
            (
                pane(app, &left).read(cx).list_state(),
                pane(app, &right).read(cx).list_state(),
            )
        })
        .unwrap();
    left_list.scroll_to(ListOffset {
        item_ix: 3_000,
        offset_in_item: px(5.0),
    });
    redraw(&mut visual);

    // The outline acts on the focused pane only.
    window
        .update(&mut visual, |app, _, cx| {
            let pane_of_right = app.split.pane_of(&right).unwrap();
            app.focus_pane(pane_of_right, cx);
            assert_eq!(active_path(app), right);
            app.jump_to_heading(1, cx);
        })
        .unwrap();
    for _ in 0..4 {
        redraw(&mut visual);
    }
    assert_eq!(left_list.logical_scroll_top().item_ix, 3_000);
    assert_eq!(right_list.logical_scroll_top().item_ix, 1);

    // Both panes' positions are in the session, alongside the split itself.
    let session = window
        .update(&mut visual, |app, _, _| app.session_snapshot())
        .unwrap();
    assert!(session.split.is_some());
    assert_eq!(
        session.anchors.get(&left).map(|anchor| anchor.block),
        Some(3_000)
    );
    assert!(session.anchors.contains_key(&right));
}
