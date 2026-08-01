use gpui::{
    App, AppContext, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions,
    point, px, size,
};
use mdow_gpui::{
    actions::{CloseTab, OpenFile, OpenFolder, Quit, ToggleSidebar, ToggleWideMode},
    app::MdowApp,
    assets::MdowAssets,
};
use std::{borrow::Cow, path::PathBuf};

fn main() {
    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    Application::new()
        .with_assets(MdowAssets::new(asset_root.clone()))
        .run(move |cx: &mut App| {
            cx.text_system()
                .add_fonts(vec![
                    Cow::Owned(
                        std::fs::read(asset_root.join("fonts/InterVariable.ttf"))
                            .expect("read bundled Inter font"),
                    ),
                    Cow::Owned(
                        std::fs::read(asset_root.join("fonts/GeistMono-Variable.ttf"))
                            .expect("read bundled Geist Mono font"),
                    ),
                ])
                .expect("register bundled Mdow fonts");

            cx.bind_keys([
                KeyBinding::new("cmd-o", OpenFile, None),
                KeyBinding::new("cmd-shift-o", OpenFolder, None),
                KeyBinding::new("cmd-b", ToggleSidebar, None),
                KeyBinding::new("cmd-w", CloseTab, None),
                KeyBinding::new("cmd-shift-w", ToggleWideMode, None),
                KeyBinding::new("cmd-q", Quit, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());

            let bounds = Bounds::centered(None, size(px(1120.0), px(760.0)), cx);
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("Mdow".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(14.0), px(14.0))),
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| MdowApp::new(window, cx)),
            )
            .expect("open Mdow window");
            cx.activate(true);
        });
}
