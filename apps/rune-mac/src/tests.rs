//! The window driven through Rune's headless runtime: open, navigate, find, tabs, split.

use crate::app::{App, Cmd, OpenIn};
use rune::{Env, Runtime};
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    // One folder per test: tests run in parallel and rewrite their files.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mdow-rune-tests-{}-{n}", std::process::id()));
    std::fs::create_dir_all(dir.join("guide")).unwrap();
    std::fs::write(dir.join("README.md"), "# Readme\n\nSee [the guide](guide/intro.md#setup) and [below](#usage).\n\n## Usage\n\nUse the thing. The thing works.\n").unwrap();
    std::fs::write(dir.join("guide/intro.md"), "# Intro\n\ntext\n\n## Setup\n\nsteps\n").unwrap();
    dir
}

fn runtime(paths: Vec<PathBuf>) -> Runtime<App> {
    let env = Env { system_dark: false, scale: 2.0 };
    let mut rt =
        Runtime::new(App::new("test".into(), paths), crate::ui::view, (1100.0, 800.0), env);
    rt.frame(0.0);
    rt
}

fn current(rt: &Runtime<App>) -> Option<PathBuf> {
    rt.state.tab().pane().path().map(Path::to_path_buf)
}

#[test]
fn links_open_documents_in_place_and_back_returns() {
    let dir = fixture();
    let mut rt = runtime(vec![dir.join("README.md")]);
    let pane = rt.state.tab().pane().id;
    rt.update(|s, tx| s.open_link(pane, "guide/intro.md#setup", tx));
    rt.frame(1.0 / 60.0);
    assert_eq!(current(&rt), Some(crate::files::normalize(&dir.join("guide/intro.md"))));
    assert!(rt.state.can_go(false));
    rt.update(|s, tx| s.run(Cmd::Back, tx));
    rt.frame(1.0 / 60.0);
    assert_eq!(current(&rt), Some(crate::files::normalize(&dir.join("README.md"))));
    assert!(rt.state.can_go(true));
}

#[test]
fn find_counts_and_steps_through_matches() {
    let dir = fixture();
    let mut rt = runtime(vec![dir.join("README.md")]);
    rt.update(|s, tx| {
        s.run(Cmd::Find, tx);
        s.find.as_mut().unwrap().query.replace_text("thing");
        s.find_changed(tx);
    });
    rt.frame(1.0 / 60.0);
    let f = rt.state.find.as_ref().unwrap();
    assert_eq!(f.matches.len(), 2);
    assert_eq!(f.current, Some(0));
    rt.update(|s, tx| s.find_step(true, tx));
    assert_eq!(rt.state.find.as_ref().unwrap().current, Some(1));
    rt.update(|s, tx| s.find_step(true, tx));
    assert_eq!(rt.state.find.as_ref().unwrap().current, Some(0), "wraps around");
}

#[test]
fn tabs_open_close_and_switch() {
    let dir = fixture();
    let mut rt = runtime(vec![dir.join("README.md")]);
    rt.update(|s, _| s.open_path(&dir.join("guide/intro.md"), OpenIn::NewTab));
    assert_eq!(rt.state.tabs.len(), 2);
    assert_eq!(rt.state.active, 1);
    rt.update(|s, tx| s.run(Cmd::SelectTab(0), tx));
    assert_eq!(rt.state.active, 0);
    rt.update(|s, tx| s.run(Cmd::CloseTab, tx));
    assert_eq!(rt.state.tabs.len(), 1);
    assert_eq!(current(&rt), Some(crate::files::normalize(&dir.join("guide/intro.md"))));
    rt.frame(1.0 / 60.0);
}

#[test]
fn split_view_shows_the_document_twice() {
    let dir = fixture();
    let mut rt = runtime(vec![dir.join("README.md")]);
    rt.update(|s, tx| s.run(Cmd::ToggleSplit, tx));
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.tab().panes.len(), 2);
    let ids: Vec<u64> = rt.state.tab().panes.iter().map(|p| p.id).collect();
    for id in ids {
        assert!(rt.find(&crate::reader::scroll_key(id)).is_some(), "pane {id} is drawn");
    }
    rt.update(|s, tx| s.run(Cmd::ToggleSplit, tx));
    assert_eq!(rt.state.tab().panes.len(), 1);
}

#[test]
fn opening_a_folder_lists_it_and_opens_its_readme() {
    let dir = fixture();
    let mut rt = runtime(Vec::new());
    rt.update(|s, _| s.set_folder(&dir));
    rt.frame(1.0 / 60.0);
    assert!(rt.state.tab().scan.as_ref().is_some_and(|s| !s.tree.is_empty()));
    assert_eq!(current(&rt), Some(crate::files::normalize(&dir.join("README.md"))));
}

#[test]
fn select_all_copies_rows_that_were_never_on_screen() {
    let dir = fixture();
    let long: String = (0..400).map(|i| format!("Paragraph {i}.\n\n")).collect();
    std::fs::write(dir.join("long.md"), long).unwrap();
    let mut rt = runtime(vec![dir.join("long.md")]);
    rt.frame(1.0 / 60.0);
    assert!(rt.update_if(|s, tx| s.select_all(tx)));
    let pane = rt.state.tab().pane().id;
    assert_eq!(rt.state.select_all, Some(pane));
    let doc = rt.state.tab().pane().doc.as_ref().unwrap();
    let text =
        crate::reader::plain_text(doc, &rt.state.read_style(), &crate::theme::Palette::get(false));
    assert!(text.starts_with("Paragraph 0."));
    assert!(text.ends_with("Paragraph 399."), "the whole document, not only mounted rows");
}

/// How long a frame takes while a reply streams into a long chat in the companion.
/// `cargo test --release bench_companion_frame -- --ignored --nocapture`
#[test]
#[ignore]
fn bench_companion_frame() {
    let dir = fixture();
    let mut rt = runtime(vec![dir.join("README.md")]);
    for turns in [1, 20, 100] {
        rt.update_if(|s, _| {
            s.companion = Default::default();
            s.companion.bench_transcript(turns);
            true
        });
        rt.frame(1.0 / 60.0);
        let mut times = Vec::new();
        for _ in 0..60 {
            rt.update_if(|s, _| {
                s.companion.bench_word();
                true
            });
            let began = std::time::Instant::now();
            rt.frame(1.0 / 60.0);
            times.push(began.elapsed());
        }
        times.sort();
        eprintln!(
            "{turns:>3} turns: median {:?}, worst {:?} a frame",
            times[times.len() / 2],
            times[times.len() - 1]
        );
    }
}
