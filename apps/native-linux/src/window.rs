//! The main window: sidebar, tab bar, breadcrumb, readers (with an optional side-by-side pane),
//! window actions and keyboard shortcuts, live reload and session persistence.

use crate::dialogs;
use crate::doc::{self, Doc};
use crate::files::{self, ErrorKind};
use crate::media;
use crate::prefs::{
    self, CodeFont, ContentFont, InterfaceScale, Prefs, ReadingWidth, SidebarMode, ThemeMode,
};
use crate::reader::{Reader, ReaderHost};
use crate::render::Style;
use crate::sidebar::Sidebar;
use crate::theme::{self, Palette};
use crate::welcome::Welcome;
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, RefCell};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pane {
    Primary,
    Secondary,
}

pub struct Tab {
    path: PathBuf,
    page: adw::TabPage,
    stack: gtk::Stack,
    reader: Reader,
    error: Cell<Option<ErrorKind>>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    content_hash: Cell<u64>,
    generation: Cell<u64>,
    reload_timer: RefCell<Option<glib::SourceId>>,
    anchor: RefCell<Option<String>>,
}

struct SecondaryPane {
    path: PathBuf,
    reader: Reader,
    generation: Cell<u64>,
}

pub struct WinInner {
    pub window: adw::ApplicationWindow,
    prefs: RefCell<Prefs>,
    prefs_path: PathBuf,
    css: gtk::CssProvider,
    split: adw::OverlaySplitView,
    sidebar: Rc<Sidebar>,
    tab_view: adw::TabView,
    tabs: RefCell<Vec<Rc<Tab>>>,
    breadcrumb: gtk::Box,
    main_stack: gtk::Stack,
    welcome: Welcome,
    primary_pane: gtk::Box,
    primary_header: gtk::Box,
    primary_name: gtk::Label,
    split_line: gtk::Box,
    secondary_box: gtk::Box,
    secondary_name: gtk::Label,
    secondary_stack: gtk::Stack,
    secondary: RefCell<Option<Rc<SecondaryPane>>>,
    active_pane: Cell<Pane>,
    start_controls: gtk::WindowControls,
    folder_monitors: RefCell<Vec<gio::FileMonitor>>,
    folder_rescan: RefCell<Option<glib::SourceId>>,
    folder_generation: Cell<u64>,
    menu_page: RefCell<Option<adw::TabPage>>,
    save_timer: RefCell<Option<glib::SourceId>>,
    recent_menu: gio::Menu,
    zoom_menu_label: gtk::Button,
    toasts: adw::ToastOverlay,
    cheat: gtk::Box,
    cheat_timer: RefCell<Option<glib::SourceId>>,
    restoring: Cell<bool>,
    host: RefCell<Option<Rc<HostRef>>>,
}

#[derive(Clone)]
pub struct Win(pub Rc<WinInner>);

/// The window as seen by its readers; held weakly so readers never keep the window alive.
struct HostRef(Weak<WinInner>);

impl ReaderHost for HostRef {
    fn open_document(&self, path: PathBuf, anchor: Option<String>) {
        if let Some(inner) = self.0.upgrade() {
            Win(inner).open_path(&path, anchor, true);
        }
    }

    fn heading_changed(&self, reader: &Reader, index: Option<usize>) {
        if let Some(inner) = self.0.upgrade() {
            let win = Win(inner);
            if win.active_reader().is_some_and(|r| r.widget() == reader.widget()) {
                win.0.sidebar.set_outline_active(index);
            }
        }
    }

    fn zoom(&self, step: i32) {
        if let Some(inner) = self.0.upgrade() {
            Win(inner).zoom(step);
        }
    }
}

fn content_hash(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn parse(text: &str, path: &Path) -> Doc {
    if files::is_html(path) { doc::parse_html(text, path) } else { doc::parse_markdown(text, path) }
}

enum Loaded {
    Unchanged,
    Doc(u64, Doc),
}

/// Read and parse on a worker thread.
async fn load(path: PathBuf, previous: u64) -> Result<Loaded, ErrorKind> {
    gio::spawn_blocking(move || {
        let text = files::read_document(&path)?;
        let hash = content_hash(&text);
        if hash == previous {
            return Ok(Loaded::Unchanged);
        }
        Ok(Loaded::Doc(hash, parse(&text, &path)))
    })
    .await
    .unwrap_or(Err(ErrorKind::ReadError))
}

fn flat_icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let b = gtk::Button::from_icon_name(icon);
    b.add_css_class("flat");
    b.add_css_class("mdow-icon-button");
    b.set_tooltip_text(Some(tooltip));
    b.set_valign(gtk::Align::Center);
    b
}

fn icon_toggle(icon: &str, tooltip: &str, action: &str) -> gtk::ToggleButton {
    let b = gtk::ToggleButton::new();
    b.set_icon_name(icon);
    b.add_css_class("flat");
    b.add_css_class("mdow-icon-button");
    b.set_tooltip_text(Some(tooltip));
    b.set_valign(gtk::Align::Center);
    b.set_action_name(Some(action));
    b
}

fn pane_header(name: &gtk::Label, badge: &str, close: Option<&gtk::Button>) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.add_css_class("mdow-pane-header");
    let mark = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    mark.add_css_class("mdow-pane-mark");
    mark.set_valign(gtk::Align::Center);
    header.append(&mark);
    let icon = gtk::Image::from_icon_name("text-x-generic-symbolic");
    icon.set_pixel_size(10);
    icon.add_css_class("mdow-dim");
    header.append(&icon);
    name.set_xalign(0.0);
    name.set_hexpand(true);
    name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    header.append(name);
    let b = gtk::Label::new(Some(badge));
    b.add_css_class("mdow-pane-badge");
    header.append(&b);
    if let Some(close) = close {
        header.append(close);
    }
    header
}

fn error_view(path: &Path, kind: ErrorKind, tab_path: &str) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 16);
    root.set_valign(gtk::Align::Center);
    root.set_halign(gtk::Align::Center);
    root.set_vexpand(true);
    root.set_margin_start(24);
    root.set_margin_end(24);
    let icon_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    icon_box.add_css_class("mdow-error-icon");
    icon_box.set_halign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name(kind.icon());
    icon.set_hexpand(true);
    icon_box.append(&icon);
    root.append(&icon_box);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let title = gtk::Label::new(Some(kind.title()));
    title.add_css_class("mdow-error-title");
    text.append(&title);
    let body = gtk::Label::new(Some(kind.body()));
    body.add_css_class("mdow-error-body");
    body.set_wrap(true);
    body.set_max_width_chars(44);
    body.set_justify(gtk::Justification::Center);
    text.append(&body);
    root.append(&text);
    let shown = files::truncate_middle(&path.to_string_lossy(), 48);
    let p = gtk::Label::new(Some(&shown));
    p.add_css_class("mdow-error-path");
    p.set_tooltip_text(Some(&path.to_string_lossy()));
    root.append(&p);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::Center);
    let retry = gtk::Button::with_label("Try again");
    retry.add_css_class("mdow-small");
    retry.add_css_class("mdow-outline");
    retry.set_action_name(Some("win.reload-path"));
    retry.set_action_target_value(Some(&tab_path.to_variant()));
    buttons.append(&retry);
    if kind != ErrorKind::PermissionDenied {
        let show = gtk::Button::new();
        show.add_css_class("flat");
        show.add_css_class("mdow-small");
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let image = gtk::Image::from_icon_name("folder-open-symbolic");
        image.set_pixel_size(14);
        content.append(&image);
        content.append(&gtk::Label::new(Some("Show in folder")));
        show.set_child(Some(&content));
        show.set_action_name(Some("win.show-in-folder"));
        show.set_action_target_value(Some(&tab_path.to_variant()));
        buttons.append(&show);
    }
    let close = gtk::Button::with_label("Close tab");
    close.add_css_class("flat");
    close.add_css_class("mdow-small");
    close.set_action_name(Some("win.close-path"));
    close.set_action_target_value(Some(&tab_path.to_variant()));
    buttons.append(&close);
    root.append(&buttons);
    root
}

impl Win {
    pub fn new(app: &adw::Application, prefs: Prefs, prefs_path: PathBuf) -> Win {
        let window = adw::ApplicationWindow::new(app);
        window.add_css_class("mdow");
        window.set_title(Some("Mdow"));
        window.set_icon_name(Some(crate::APP_ID));
        window.set_size_request(600, 400);
        window.set_default_size(prefs.window.width.max(600), prefs.window.height.max(400));
        if prefs.window.maximized {
            window.maximize();
        }

        let css = gtk::CssProvider::new();
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
            );
        }

        let sidebar = Sidebar::new();
        sidebar.connect();

        // Tabs.
        let tab_view = adw::TabView::new();
        tab_view.set_vexpand(true);
        tab_view.set_hexpand(true);
        let tab_bar = adw::TabBar::new();
        tab_bar.set_view(Some(&tab_view));
        tab_bar.set_autohide(false);
        tab_bar.set_expand_tabs(false);
        tab_bar.set_hexpand(true);

        let start_controls = gtk::WindowControls::new(gtk::PackType::Start);
        let end_controls = gtk::WindowControls::new(gtk::PackType::End);
        let sidebar_toggle =
            icon_toggle("sidebar-show-symbolic", "Toggle sidebar (Ctrl+B)", "win.toggle-sidebar");
        sidebar_toggle.add_css_class("mdow-quiet");
        let split_toggle =
            icon_toggle("view-dual-symbolic", "Open side-by-side view", "win.split-view");

        // Primary menu, with a zoom row as in GNOME's own apps.
        let recent_menu = gio::Menu::new();
        let menu = gio::Menu::new();
        let files_section = gio::Menu::new();
        files_section.append(Some("Open File…"), Some("win.open-file"));
        files_section.append(Some("Open Folder…"), Some("win.open-folder"));
        files_section.append_submenu(Some("Open Recent"), &recent_menu);
        menu.append_section(None, &files_section);
        let nav = gio::Menu::new();
        nav.append(Some("Command Palette"), Some("win.palette"));
        nav.append(Some("Find…"), Some("win.find"));
        menu.append_section(None, &nav);
        let zoom_section = gio::Menu::new();
        let zoom_item = gio::MenuItem::new(None, None);
        zoom_item.set_attribute_value("custom", Some(&"zoom".to_variant()));
        zoom_section.append_item(&zoom_item);
        menu.append_section(None, &zoom_section);
        let view = gio::Menu::new();
        view.append(Some("Show Sidebar"), Some("win.toggle-sidebar"));
        view.append(Some("Wide Mode"), Some("win.wide-mode"));
        view.append(Some("Split View"), Some("win.split-view"));
        menu.append_section(None, &view);
        let app_section = gio::Menu::new();
        app_section.append(Some("Settings"), Some("win.settings"));
        app_section.append(Some("Keyboard Shortcuts"), Some("win.shortcuts"));
        app_section.append(Some("About Mdow"), Some("win.about"));
        menu.append_section(None, &app_section);
        let close_section = gio::Menu::new();
        close_section.append(Some("Close Tab"), Some("win.close-tab"));
        close_section.append(Some("Quit"), Some("app.quit"));
        menu.append_section(None, &close_section);
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.add_css_class("flat");
        menu_button.add_css_class("mdow-icon-button");
        menu_button.set_tooltip_text(Some("Main menu"));
        menu_button.set_valign(gtk::Align::Center);
        menu_button.set_menu_model(Some(&menu));
        let zoom_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        zoom_row.add_css_class("linked");
        zoom_row.set_halign(gtk::Align::Center);
        zoom_row.set_margin_start(12);
        zoom_row.set_margin_end(12);
        let zoom_out = gtk::Button::from_icon_name("zoom-out-symbolic");
        zoom_out.set_action_name(Some("win.zoom-out"));
        zoom_out.set_tooltip_text(Some("Zoom out"));
        let zoom_menu_label = gtk::Button::with_label("100%");
        zoom_menu_label.set_action_name(Some("win.zoom-reset"));
        zoom_menu_label.set_tooltip_text(Some("Reset zoom"));
        zoom_menu_label.set_hexpand(true);
        let zoom_in = gtk::Button::from_icon_name("zoom-in-symbolic");
        zoom_in.set_action_name(Some("win.zoom-in"));
        zoom_in.set_tooltip_text(Some("Zoom in"));
        for b in [&zoom_out, &zoom_menu_label, &zoom_in] {
            b.add_css_class("flat");
            zoom_row.append(b);
        }
        if let Some(popover) = menu_button.popover().and_downcast::<gtk::PopoverMenu>() {
            popover.add_child(&zoom_row, "zoom");
        }

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        actions.add_css_class("mdow-tab-actions");
        actions.set_valign(gtk::Align::Fill);
        actions.append(&split_toggle);
        actions.append(&menu_button);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        header.add_css_class("mdow-tabbar");
        start_controls.set_margin_start(6);
        header.append(&start_controls);
        sidebar_toggle.set_margin_start(6);
        header.append(&sidebar_toggle);
        header.append(&tab_bar);
        header.append(&actions);
        end_controls.set_margin_end(6);
        header.append(&end_controls);
        let handle = gtk::WindowHandle::new();
        handle.set_child(Some(&header));

        let breadcrumb = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        breadcrumb.add_css_class("mdow-breadcrumb");
        breadcrumb.set_visible(false);

        // Panes.
        let primary_name = gtk::Label::new(None);
        let primary_header = pane_header(&primary_name, "Left", None);
        primary_header.set_visible(false);
        let primary_pane = gtk::Box::new(gtk::Orientation::Vertical, 0);
        primary_pane.add_css_class("mdow-pane");
        primary_pane.set_hexpand(true);
        primary_pane.append(&primary_header);
        primary_pane.append(&tab_view);
        let split_line = gtk::Box::new(gtk::Orientation::Vertical, 0);
        split_line.add_css_class("mdow-split-line");
        split_line.set_visible(false);
        let secondary_name = gtk::Label::new(None);
        let close_split = flat_icon_button("view-dual-symbolic", "Close split view");
        close_split.set_action_name(Some("win.close-split"));
        let secondary_header = pane_header(&secondary_name, "Right", Some(&close_split));
        let secondary_stack = gtk::Stack::new();
        secondary_stack.set_vexpand(true);
        let empty = gtk::Box::new(gtk::Orientation::Vertical, 8);
        empty.set_valign(gtk::Align::Center);
        empty.set_halign(gtk::Align::Center);
        let empty_icon = gtk::Image::from_icon_name("view-dual-symbolic");
        empty_icon.set_pixel_size(20);
        empty_icon.add_css_class("mdow-dim");
        empty.append(&empty_icon);
        let empty_title = gtk::Label::new(Some("No document in this pane"));
        empty_title.add_css_class("mdow-error-title");
        empty.append(&empty_title);
        let empty_hint = gtk::Label::new(Some(
            "Select this pane, then open a document or choose a tab from the tab bar.",
        ));
        empty_hint.add_css_class("mdow-dim");
        empty_hint.set_wrap(true);
        empty_hint.set_max_width_chars(36);
        empty_hint.set_justify(gtk::Justification::Center);
        empty.append(&empty_hint);
        secondary_stack.add_named(&empty, Some("empty"));
        let secondary_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        secondary_box.add_css_class("mdow-pane");
        secondary_box.set_hexpand(true);
        secondary_box.append(&secondary_header);
        secondary_box.append(&secondary_stack);
        secondary_box.set_visible(false);
        let docs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        docs.set_homogeneous(false);
        docs.append(&primary_pane);
        docs.append(&split_line);
        docs.append(&secondary_box);

        let welcome = Welcome::new();
        let main_stack = gtk::Stack::new();
        main_stack.set_vexpand(true);
        main_stack.add_named(&welcome.root, Some("welcome"));
        main_stack.add_named(&docs, Some("docs"));
        main_stack.set_visible_child_name("welcome");

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&handle);
        content.append(&breadcrumb);
        content.append(&main_stack);

        let split = adw::OverlaySplitView::new();
        split.set_sidebar(Some(&sidebar.root));
        split.set_content(Some(&content));
        let width = theme::sidebar_width(prefs.interface_scale);
        split.set_min_sidebar_width(width);
        split.set_max_sidebar_width(width);
        split.set_show_sidebar(prefs.sidebar_open);
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            720.0,
            adw::LengthUnit::Px,
        ));
        breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        let cheat = dialogs::cheat_sheet();
        cheat.set_visible(false);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&split));
        overlay.add_overlay(&cheat);
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&overlay));
        window.set_content(Some(&toasts));

        let inner = Rc::new(WinInner {
            window,
            prefs: RefCell::new(prefs),
            prefs_path,
            css,
            split,
            sidebar,
            tab_view,
            tabs: RefCell::new(Vec::new()),
            breadcrumb,
            main_stack,
            welcome,
            primary_pane,
            primary_header,
            primary_name,
            split_line,
            secondary_box,
            secondary_name,
            secondary_stack,
            secondary: RefCell::new(None),
            active_pane: Cell::new(Pane::Primary),
            start_controls,
            folder_monitors: RefCell::new(Vec::new()),
            folder_rescan: RefCell::new(None),
            folder_generation: Cell::new(0),
            menu_page: RefCell::new(None),
            save_timer: RefCell::new(None),
            recent_menu,
            zoom_menu_label,
            toasts,
            cheat,
            cheat_timer: RefCell::new(None),
            restoring: Cell::new(false),
            host: RefCell::new(None),
        });
        inner.host.replace(Some(Rc::new(HostRef(Rc::downgrade(&inner)))));
        let win = Win(inner);
        win.setup_actions(&sidebar_toggle, &split_toggle);
        win.setup_signals();
        win.apply_visuals();
        win.refresh_recents();
        let mode = win.0.prefs.borrow().sidebar_mode;
        win.0.sidebar.set_mode(mode);
        win.update_controls();
        win.update_zoom_label();
        win
    }

    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.0.window
    }

    fn weak(&self) -> Weak<WinInner> {
        Rc::downgrade(&self.0)
    }

    fn reader_host(&self) -> Weak<dyn ReaderHost> {
        let host = self.0.host.borrow().clone().expect("host is set at construction");
        let weak: Weak<dyn ReaderHost> = Rc::downgrade(&(host as Rc<dyn ReaderHost>));
        weak
    }

    // ── Style ────────────────────────────────────────────────────────────────

    fn palette(&self) -> Rc<Palette> {
        Rc::new(Palette::for_dark(adw::StyleManager::default().is_dark()))
    }

    fn style(&self) -> Rc<Style> {
        let prefs = self.0.prefs.borrow();
        Rc::new(Style {
            palette: self.palette(),
            fs: theme::reader_font_size(prefs.zoom_level),
            code_family: prefs.code_font.pango_family(),
            scale_factor: self.0.window.scale_factor().max(1),
        })
    }

    fn column(&self) -> Option<i32> {
        let prefs = self.0.prefs.borrow();
        (!prefs.wide_mode).then(|| prefs.reading_width.px())
    }

    fn new_reader(&self) -> Reader {
        let reader = Reader::new(self.style(), self.column());
        reader.set_host(self.reader_host());
        reader.set_zoom(self.0.prefs.borrow().zoom_level, false);
        reader
    }

    /// Theme, fonts, scale, zoom or width changed: restyle everything.
    fn apply_visuals(&self) {
        let scheme = match self.0.prefs.borrow().theme {
            ThemeMode::System => adw::ColorScheme::Default,
            ThemeMode::Light => adw::ColorScheme::ForceLight,
            ThemeMode::Dark => adw::ColorScheme::ForceDark,
        };
        let manager = adw::StyleManager::default();
        if manager.color_scheme() != scheme {
            // Changing the scheme re-enters through `notify::dark`.
            manager.set_color_scheme(scheme);
        }
        let palette = self.palette();
        let css = theme::stylesheet(&palette, &self.0.prefs.borrow());
        self.0.css.load_from_string(&css);
        let width = theme::sidebar_width(self.0.prefs.borrow().interface_scale);
        self.0.split.set_min_sidebar_width(width);
        self.0.split.set_max_sidebar_width(width);
        let scale = self.0.window.scale_factor().max(1);
        self.0.welcome.set_logo(media::logo(48.0, scale).as_ref());
        let style = self.style();
        let column = self.column();
        for reader in self.all_readers() {
            reader.set_style(style.clone(), column);
        }
    }

    fn all_readers(&self) -> Vec<Reader> {
        let mut readers: Vec<Reader> =
            self.0.tabs.borrow().iter().map(|t| t.reader.clone()).collect();
        if let Some(s) = self.0.secondary.borrow().as_ref() {
            readers.push(s.reader.clone());
        }
        readers
    }

    // ── Prefs ────────────────────────────────────────────────────────────────

    fn update_prefs(&self, f: impl FnOnce(&mut Prefs)) {
        f(&mut self.0.prefs.borrow_mut());
        self.schedule_save();
    }

    fn schedule_save(&self) {
        if self.0.restoring.get() {
            return;
        }
        if let Some(id) = self.0.save_timer.take() {
            id.remove();
        }
        let weak = self.weak();
        let id = glib::timeout_add_local_once(Duration::from_millis(400), move || {
            if let Some(inner) = weak.upgrade() {
                inner.save_timer.take();
                Win(inner).save_now();
            }
        });
        self.0.save_timer.replace(Some(id));
    }

    pub fn save_now(&self) {
        if let Some(id) = self.0.save_timer.take() {
            id.remove();
        }
        let tabs: Vec<String> = (0..self.0.tab_view.n_pages())
            .filter_map(|i| self.tab_for_page(&self.0.tab_view.nth_page(i)))
            .map(|t| t.path.to_string_lossy().into_owned())
            .collect();
        let active = self.active_tab().map(|t| t.path.to_string_lossy().into_owned());
        let secondary =
            self.0.secondary.borrow().as_ref().map(|s| s.path.to_string_lossy().into_owned());
        let window = &self.0.window;
        let mut prefs = self.0.prefs.borrow_mut();
        prefs.session_tabs = tabs;
        prefs.session_active_tab_path = active;
        prefs.session_split_view = self.0.secondary_box.is_visible();
        prefs.session_secondary_pane_path = secondary;
        prefs.sidebar_open = self.0.split.shows_sidebar();
        prefs.window.maximized = window.is_maximized();
        if !window.is_maximized() && !window.is_fullscreen() {
            let (w, h) = window.default_size();
            if w > 0 && h > 0 {
                prefs.window.width = w;
                prefs.window.height = h;
            }
        }
        if let Err(err) = prefs.save_to(&self.0.prefs_path) {
            eprintln!("mdow: could not save state: {err}");
        }
    }

    fn refresh_recents(&self) {
        let recents = self.0.prefs.borrow_mut().existing_recents();
        let active = self.active_path();
        self.0.sidebar.set_recents(&recents, active.as_deref());
        self.0.welcome.set_recents(&recents);
        self.0.recent_menu.remove_all();
        if recents.is_empty() {
            let item = gio::MenuItem::new(Some("No Recent Files"), Some("win.none"));
            self.0.recent_menu.append_item(&item);
        }
        for path in recents.iter().take(10) {
            let item = gio::MenuItem::new(Some(&files::display_name(Path::new(path))), None);
            item.set_action_and_target_value(Some("win.open-path"), Some(&path.to_variant()));
            self.0.recent_menu.append_item(&item);
        }
    }

    // ── Tabs ─────────────────────────────────────────────────────────────────

    fn tab_for_page(&self, page: &adw::TabPage) -> Option<Rc<Tab>> {
        self.0.tabs.borrow().iter().find(|t| t.page == *page).cloned()
    }

    fn tab_for_path(&self, path: &Path) -> Option<Rc<Tab>> {
        self.0.tabs.borrow().iter().find(|t| t.path == path).cloned()
    }

    fn active_tab(&self) -> Option<Rc<Tab>> {
        self.0.tab_view.selected_page().and_then(|p| self.tab_for_page(&p))
    }

    fn active_path(&self) -> Option<PathBuf> {
        if self.split_on()
            && self.0.active_pane.get() == Pane::Secondary
            && let Some(s) = self.0.secondary.borrow().as_ref()
        {
            return Some(s.path.clone());
        }
        self.active_tab().map(|t| t.path.clone())
    }

    /// The reader the outline, find and zoom indicator act on.
    fn active_reader(&self) -> Option<Reader> {
        if self.split_on() && self.0.active_pane.get() == Pane::Secondary {
            return self.0.secondary.borrow().as_ref().map(|s| s.reader.clone());
        }
        self.active_tab().filter(|t| t.error.get().is_none()).map(|t| t.reader.clone())
    }

    fn split_on(&self) -> bool {
        self.0.secondary_box.is_visible()
    }

    /// Open a document (or a folder). Documents already open are focused, not duplicated.
    pub fn open_path(&self, path: &Path, anchor: Option<String>, activate: bool) {
        let path = if path.is_absolute() {
            files::normalize(path)
        } else {
            std::env::current_dir()
                .map(|d| files::normalize(&d.join(path)))
                .unwrap_or(path.to_path_buf())
        };
        if path.is_dir() {
            self.open_folder(&path, true);
            return;
        }
        if !files::is_document(&path) {
            let toast = adw::Toast::new(&format!(
                "{} is not a markdown or HTML document",
                files::display_name(&path)
            ));
            self.0.toasts.add_toast(toast);
            return;
        }
        if self.split_on() && self.0.active_pane.get() == Pane::Secondary && activate {
            self.ensure_tab(&path, None, false);
            self.set_secondary(Some(&path), anchor);
            return;
        }
        let tab = self.ensure_tab(&path, anchor.clone(), activate);
        if let Some(anchor) = anchor
            && tab.reader.has_document()
        {
            tab.reader.scroll_to_anchor(&anchor);
        }
    }

    fn ensure_tab(&self, path: &Path, anchor: Option<String>, activate: bool) -> Rc<Tab> {
        if let Some(tab) = self.tab_for_path(path) {
            if activate {
                self.0.tab_view.set_selected_page(&tab.page);
            }
            // Re-opened from recents or the file manager: refresh if the file changed.
            self.load_tab(&tab, true);
            return tab;
        }
        let reader = self.new_reader();
        let stack = gtk::Stack::new();
        stack.add_named(reader.widget(), Some("reader"));
        stack.set_visible_child_name("reader");
        let position = self
            .0
            .tab_view
            .selected_page()
            .map(|p| self.0.tab_view.page_position(&p) + 1)
            .unwrap_or(self.0.tab_view.n_pages());
        let page = self.0.tab_view.insert(&stack, position);
        page.set_title(&files::display_name(path));
        page.set_tooltip(&glib::markup_escape_text(&path.to_string_lossy()));
        page.set_icon(Some(&gio::ThemedIcon::new("text-x-generic-symbolic")));
        page.set_loading(true);
        let tab = Rc::new(Tab {
            path: path.to_path_buf(),
            page: page.clone(),
            stack,
            reader,
            error: Cell::new(None),
            monitor: RefCell::new(None),
            content_hash: Cell::new(0),
            generation: Cell::new(0),
            reload_timer: RefCell::new(None),
            anchor: RefCell::new(anchor),
        });
        self.0.tabs.borrow_mut().push(tab.clone());
        self.watch_tab(&tab);
        if activate || self.0.tab_view.n_pages() == 1 {
            self.0.tab_view.set_selected_page(&page);
        }
        self.0.main_stack.set_visible_child_name("docs");
        self.0.sidebar.set_opening(true);
        self.load_tab(&tab, false);
        self.schedule_save();
        tab
    }

    fn load_tab(&self, tab: &Rc<Tab>, keep_position: bool) {
        let generation = tab.generation.get() + 1;
        tab.generation.set(generation);
        let previous =
            if keep_position && tab.error.get().is_none() { tab.content_hash.get() } else { 0 };
        let weak_win = self.weak();
        let weak_tab = Rc::downgrade(tab);
        let path = tab.path.clone();
        glib::spawn_future_local(async move {
            let result = load(path.clone(), previous).await;
            let (Some(inner), Some(tab)) = (weak_win.upgrade(), weak_tab.upgrade()) else { return };
            let win = Win(inner);
            if tab.generation.get() != generation {
                return;
            }
            tab.page.set_loading(false);
            win.0.sidebar.set_opening(false);
            match result {
                Ok(Loaded::Unchanged) => {}
                Ok(Loaded::Doc(hash, doc)) => {
                    let had_doc = tab.reader.has_document() && tab.error.get().is_none();
                    tab.content_hash.set(hash);
                    tab.error.set(None);
                    tab.page.set_icon(Some(&gio::ThemedIcon::new("text-x-generic-symbolic")));
                    tab.reader.set_document(doc, &path, keep_position && had_doc);
                    tab.stack.set_visible_child_name("reader");
                    if let Some(anchor) = tab.anchor.take() {
                        tab.reader.scroll_to_anchor(&anchor);
                    }
                    win.update_prefs(|p| p.push_recent(&path.to_string_lossy()));
                    win.refresh_recents();
                    if win.active_tab().is_some_and(|t| Rc::ptr_eq(&t, &tab)) {
                        win.refresh_active();
                    }
                    win.reload_secondary_if(&path);
                }
                Err(kind) => win.show_error(&tab, kind),
            }
        });
    }

    fn show_error(&self, tab: &Rc<Tab>, kind: ErrorKind) {
        tab.error.set(Some(kind));
        tab.content_hash.set(0);
        tab.page.set_icon(Some(&gio::ThemedIcon::new("dialog-error-symbolic")));
        if let Some(old) = tab.stack.child_by_name("error") {
            tab.stack.remove(&old);
        }
        let view = error_view(&tab.path, kind, &tab.path.to_string_lossy());
        tab.stack.add_named(&view, Some("error"));
        tab.stack.set_visible_child_name("error");
        if self.active_tab().is_some_and(|t| Rc::ptr_eq(&t, tab)) {
            self.refresh_active();
        }
    }

    fn watch_tab(&self, tab: &Rc<Tab>) {
        let file = gio::File::for_path(&tab.path);
        let Ok(monitor) =
            file.monitor_file(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        else {
            return;
        };
        let weak_win = self.weak();
        let weak_tab = Rc::downgrade(tab);
        monitor.connect_changed(move |_, _, _, event| {
            let relevant = matches!(
                event,
                gio::FileMonitorEvent::ChangesDoneHint
                    | gio::FileMonitorEvent::Changed
                    | gio::FileMonitorEvent::Created
                    | gio::FileMonitorEvent::Deleted
                    | gio::FileMonitorEvent::MovedIn
                    | gio::FileMonitorEvent::MovedOut
                    | gio::FileMonitorEvent::Renamed
            );
            if !relevant {
                return;
            }
            let (Some(inner), Some(tab)) = (weak_win.upgrade(), weak_tab.upgrade()) else { return };
            Win(inner).schedule_reload(&tab);
        });
        tab.monitor.replace(Some(monitor));
    }

    /// Debounced: editors write files in several steps (truncate, write, rename).
    fn schedule_reload(&self, tab: &Rc<Tab>) {
        if let Some(id) = tab.reload_timer.take() {
            id.remove();
        }
        let weak_win = self.weak();
        let weak_tab = Rc::downgrade(tab);
        let id = glib::timeout_add_local_once(Duration::from_millis(150), move || {
            let (Some(inner), Some(tab)) = (weak_win.upgrade(), weak_tab.upgrade()) else { return };
            tab.reload_timer.take();
            let win = Win(inner);
            if tab.path.exists() {
                win.load_tab(&tab, true);
            } else {
                // A save by rename briefly leaves no file; only then call it deleted.
                let weak_win = win.weak();
                let weak_tab = Rc::downgrade(&tab);
                glib::timeout_add_local_once(Duration::from_millis(350), move || {
                    let (Some(inner), Some(tab)) = (weak_win.upgrade(), weak_tab.upgrade()) else {
                        return;
                    };
                    let win = Win(inner);
                    if tab.path.exists() {
                        win.load_tab(&tab, true);
                    } else {
                        win.show_error(&tab, ErrorKind::Deleted);
                    }
                });
            }
        });
        tab.reload_timer.replace(Some(id));
    }

    fn close_tab(&self, tab: &Rc<Tab>) {
        self.0.tab_view.close_page(&tab.page);
    }

    fn cycle_tab(&self, step: i32) {
        let n = self.0.tab_view.n_pages();
        if n == 0 {
            return;
        }
        let current =
            self.0.tab_view.selected_page().map(|p| self.0.tab_view.page_position(&p)).unwrap_or(0);
        let next = (current + step).rem_euclid(n);
        self.0.tab_view.set_selected_page(&self.0.tab_view.nth_page(next));
    }

    /// The selected tab (or pane) changed: sync the chrome to it.
    fn refresh_active(&self) {
        let tab = self.active_tab();
        let has_tabs = self.0.tab_view.n_pages() > 0;
        self.0.main_stack.set_visible_child_name(if has_tabs { "docs" } else { "welcome" });
        let active_path = self.active_path();
        let title =
            active_path.as_deref().map(files::display_name).unwrap_or_else(|| "Mdow".to_string());
        self.0.window.set_title(Some(&title));
        self.0.sidebar.set_recents_active(active_path.as_deref());
        self.0.sidebar.reveal_in_tree(active_path.as_deref());
        match self.active_reader() {
            Some(reader) => {
                self.0.sidebar.set_outline(&reader.headings(), true, reader.active_heading());
            }
            None => self.0.sidebar.set_outline(&[], active_path.is_some(), None),
        }
        self.update_breadcrumb(tab.as_ref());
        self.update_panes();
    }

    fn update_breadcrumb(&self, tab: Option<&Rc<Tab>>) {
        let bar = &self.0.breadcrumb;
        while let Some(child) = bar.first_child() {
            bar.remove(&child);
        }
        let Some(tab) = tab.filter(|_| !self.split_on()) else {
            bar.set_visible(false);
            return;
        };
        bar.set_visible(true);
        let nav = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        nav.set_hexpand(true);
        nav.set_overflow(gtk::Overflow::Hidden);
        let root = self.0.sidebar.folder_root();
        for (name, dir) in files::breadcrumb_segments(&tab.path, root.as_deref()) {
            let b = gtk::Button::with_label(&name);
            b.add_css_class("flat");
            b.add_css_class("mdow-crumb");
            b.set_tooltip_text(Some(&format!("Show in Folder: {}", dir.display())));
            b.set_action_name(Some("win.show-in-folder"));
            b.set_action_target_value(Some(&dir.to_string_lossy().to_variant()));
            if let Some(label) = b.child().and_downcast::<gtk::Label>() {
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            }
            nav.append(&b);
            let chevron = gtk::Image::from_icon_name("pan-end-symbolic");
            chevron.add_css_class("mdow-crumb-sep");
            nav.append(&chevron);
        }
        let filename = files::display_name(&tab.path);
        let title = tab
            .reader
            .title()
            .filter(|_| tab.error.get().is_none())
            .unwrap_or_else(|| filename.clone());
        let file = gtk::Button::with_label(&title);
        file.add_css_class("flat");
        file.add_css_class("mdow-crumb");
        file.add_css_class("mdow-crumb-file");
        file.set_tooltip_text(Some("Show in Folder"));
        file.set_action_name(Some("win.show-in-folder"));
        file.set_action_target_value(Some(&tab.path.to_string_lossy().to_variant()));
        if let Some(label) = file.child().and_downcast::<gtk::Label>() {
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        }
        nav.append(&file);
        if title != filename {
            let secondary = gtk::Label::new(Some(&filename));
            secondary.add_css_class("mdow-crumb-secondary");
            secondary.set_ellipsize(gtk::pango::EllipsizeMode::End);
            secondary.set_margin_start(4);
            nav.append(&secondary);
        }
        bar.append(&nav);
        let wide = self.0.prefs.borrow().wide_mode;
        let toggle = flat_icon_button(
            if wide { "mdow-narrow-symbolic" } else { "mdow-wide-symbolic" },
            if wide { "Constrained width" } else { "Full width" },
        );
        toggle.set_action_name(Some("win.wide-mode"));
        bar.append(&toggle);
    }

    // ── Split view ───────────────────────────────────────────────────────────

    fn update_panes(&self) {
        let split = self.split_on();
        self.0.primary_header.set_visible(split);
        self.0.split_line.set_visible(split);
        if split {
            let primary = self.active_tab().map(|t| files::display_name(&t.path));
            self.0.primary_name.set_text(primary.as_deref().unwrap_or("No document"));
            let secondary =
                self.0.secondary.borrow().as_ref().map(|s| files::display_name(&s.path));
            self.0.secondary_name.set_text(secondary.as_deref().unwrap_or("No document"));
        }
        let active = self.0.active_pane.get();
        for (pane, widget) in
            [(Pane::Primary, &self.0.primary_pane), (Pane::Secondary, &self.0.secondary_box)]
        {
            if split && pane == active {
                widget.add_css_class("mdow-pane-active");
            } else {
                widget.remove_css_class("mdow-pane-active");
            }
        }
    }

    fn set_split(&self, on: bool) {
        if on == self.split_on() {
            return;
        }
        if on {
            self.0.secondary_box.set_visible(true);
            if self.0.secondary.borrow().is_none() {
                // Like the desktop: the next tab after the active one, else the previous.
                let n = self.0.tab_view.n_pages();
                let current =
                    self.0.tab_view.selected_page().map(|p| self.0.tab_view.page_position(&p));
                let pick = current.and_then(|c| {
                    if c + 1 < n {
                        Some(c + 1)
                    } else if c > 0 {
                        Some(c - 1)
                    } else {
                        None
                    }
                });
                let path = pick
                    .and_then(|i| self.tab_for_page(&self.0.tab_view.nth_page(i)))
                    .map(|t| t.path.clone());
                self.set_secondary(path.as_deref(), None);
            }
        } else {
            self.0.secondary_box.set_visible(false);
            self.0.active_pane.set(Pane::Primary);
            if let Some(s) = self.0.secondary.take() {
                self.0.secondary_stack.remove(s.reader.widget());
            }
        }
        if let Some(action) = self.0.window.lookup_action("split-view") {
            action.change_state(&on.to_variant());
        }
        self.refresh_active();
        self.schedule_save();
    }

    fn set_secondary(&self, path: Option<&Path>, anchor: Option<String>) {
        if !self.split_on() {
            self.0.secondary_box.set_visible(true);
            if let Some(action) = self.0.window.lookup_action("split-view") {
                action.change_state(&true.to_variant());
            }
        }
        if let Some(old) = self.0.secondary.take() {
            self.0.secondary_stack.remove(old.reader.widget());
        }
        match path {
            Some(path) => {
                let reader = self.new_reader();
                self.0.secondary_stack.add_named(reader.widget(), Some("reader"));
                self.0.secondary_stack.set_visible_child_name("reader");
                let pane = Rc::new(SecondaryPane {
                    path: path.to_path_buf(),
                    reader,
                    generation: Cell::new(0),
                });
                self.0.secondary.replace(Some(pane.clone()));
                self.load_secondary(&pane, false, anchor);
            }
            None => self.0.secondary_stack.set_visible_child_name("empty"),
        }
        self.refresh_active();
        self.schedule_save();
    }

    fn load_secondary(&self, pane: &Rc<SecondaryPane>, keep: bool, anchor: Option<String>) {
        let generation = pane.generation.get() + 1;
        pane.generation.set(generation);
        let weak_win = self.weak();
        let weak_pane = Rc::downgrade(pane);
        let path = pane.path.clone();
        glib::spawn_future_local(async move {
            let result = load(path.clone(), 0).await;
            let (Some(inner), Some(pane)) = (weak_win.upgrade(), weak_pane.upgrade()) else {
                return;
            };
            if pane.generation.get() != generation {
                return;
            }
            let win = Win(inner);
            if let Ok(Loaded::Doc(_, doc)) = result {
                pane.reader.set_document(doc, &path, keep);
                if let Some(anchor) = anchor {
                    pane.reader.scroll_to_anchor(&anchor);
                }
                if win.0.active_pane.get() == Pane::Secondary {
                    win.refresh_active();
                }
            }
        });
    }

    fn reload_secondary_if(&self, path: &Path) {
        let pane = self.0.secondary.borrow().clone();
        if let Some(pane) = pane
            && pane.path == path
            && pane.reader.has_document()
        {
            self.load_secondary(&pane, true, None);
        }
    }

    fn set_active_pane(&self, pane: Pane) {
        if self.0.active_pane.replace(pane) != pane {
            self.refresh_active();
        }
    }

    // ── Folder ───────────────────────────────────────────────────────────────

    pub fn open_folder(&self, path: &Path, reveal: bool) {
        let generation = self.0.folder_generation.get() + 1;
        self.0.folder_generation.set(generation);
        let weak = self.weak();
        let root = path.to_path_buf();
        glib::spawn_future_local(async move {
            let scan_root = root.clone();
            let scan = gio::spawn_blocking(move || files::scan_folder(&scan_root))
                .await
                .unwrap_or_default();
            let Some(inner) = weak.upgrade() else { return };
            let win = Win(inner);
            if win.0.folder_generation.get() != generation {
                return;
            }
            let dirs = scan.dirs.clone();
            win.0.sidebar.set_folder(Some((root.clone(), scan)));
            win.watch_folder(&root, &dirs);
            win.update_prefs(|p| p.last_folder = Some(root.to_string_lossy().into_owned()));
            if reveal {
                win.set_sidebar_mode(SidebarMode::Folder);
                win.0.split.set_show_sidebar(true);
            }
            win.refresh_active();
        });
    }

    fn watch_folder(&self, root: &Path, dirs: &[PathBuf]) {
        let mut monitors = Vec::new();
        for dir in dirs.iter().take(4000) {
            let Ok(monitor) = gio::File::for_path(dir)
                .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
            else {
                continue;
            };
            let weak = self.weak();
            let root = root.to_path_buf();
            monitor.connect_changed(move |_, file, _, event| {
                if matches!(
                    event,
                    gio::FileMonitorEvent::Changed | gio::FileMonitorEvent::AttributeChanged
                ) {
                    return;
                }
                let interesting = file.path().is_some_and(|p| p.is_dir() || files::is_document(&p))
                    || matches!(
                        event,
                        gio::FileMonitorEvent::Deleted | gio::FileMonitorEvent::MovedOut
                    );
                if !interesting {
                    return;
                }
                let Some(inner) = weak.upgrade() else { return };
                let win = Win(inner);
                if let Some(id) = win.0.folder_rescan.take() {
                    id.remove();
                }
                let weak = win.weak();
                let root = root.clone();
                let id = glib::timeout_add_local_once(Duration::from_millis(400), move || {
                    if let Some(inner) = weak.upgrade() {
                        inner.folder_rescan.take();
                        Win(inner).open_folder(&root, false);
                    }
                });
                win.0.folder_rescan.replace(Some(id));
            });
            monitors.push(monitor);
        }
        self.0.folder_monitors.replace(monitors);
    }

    fn set_sidebar_mode(&self, mode: SidebarMode) {
        self.update_prefs(|p| p.sidebar_mode = mode);
        self.0.sidebar.set_mode(mode);
        if let Some(action) = self.0.window.lookup_action("sidebar-mode") {
            action.change_state(&mode.id().to_variant());
        }
    }

    // ── Zoom ─────────────────────────────────────────────────────────────────

    fn zoom(&self, step: i32) {
        let current = self.0.prefs.borrow().zoom_level;
        let next = match step {
            0 => 100,
            s => prefs::clamp_zoom((current as i32 + s * prefs::ZOOM_STEP as i32).max(0) as u32),
        };
        if next == current && step != 0 {
            return;
        }
        self.update_prefs(|p| p.zoom_level = next);
        self.apply_visuals();
        let active = self.active_reader();
        for reader in self.all_readers() {
            let announce = active.as_ref().is_some_and(|a| a.widget() == reader.widget());
            reader.set_zoom(next, announce);
        }
        self.update_zoom_label();
    }

    fn update_zoom_label(&self) {
        self.0.zoom_menu_label.set_label(&format!("{}%", self.0.prefs.borrow().zoom_level));
    }

    // ── Dialogs ──────────────────────────────────────────────────────────────

    pub fn open_file_dialog(&self) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Markdown and HTML"));
        for ext in files::MD_EXTENSIONS.iter().chain(files::HTML_EXTENSIONS.iter()) {
            filter.add_suffix(ext);
        }
        filter.add_mime_type("text/markdown");
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder()
            .title("Open File")
            .modal(true)
            .filters(&filters)
            .default_filter(&filter)
            .build();
        if let Some(dir) = self
            .active_path()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .or_else(|| self.0.sidebar.folder_root())
        {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let weak = self.weak();
        dialog.open_multiple(Some(&self.0.window), gio::Cancellable::NONE, move |result| {
            let (Some(inner), Ok(files)) = (weak.upgrade(), result) else { return };
            let win = Win(inner);
            let n = files.n_items();
            for i in 0..n {
                if let Some(path) = files.item(i).and_downcast::<gio::File>().and_then(|f| f.path())
                {
                    win.open_path(&path, None, i + 1 == n);
                }
            }
        });
    }

    pub fn open_folder_dialog(&self) {
        let dialog = gtk::FileDialog::builder().title("Open Folder").modal(true).build();
        if let Some(dir) = self.0.sidebar.folder_root() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let weak = self.weak();
        dialog.select_folder(Some(&self.0.window), gio::Cancellable::NONE, move |result| {
            let (Some(inner), Ok(folder)) = (weak.upgrade(), result) else { return };
            if let Some(path) = folder.path() {
                Win(inner).open_folder(&path, true);
            }
        });
    }

    fn show_in_folder(&self, path: &Path) {
        let file = gio::File::for_path(path);
        gtk::FileLauncher::new(Some(&file)).open_containing_folder(
            Some(&self.0.window),
            gio::Cancellable::NONE,
            |_| {},
        );
    }

    fn copy_path(&self, path: &str) {
        self.0.window.clipboard().set_text(path);
        let toast = adw::Toast::new("Path copied");
        toast.set_timeout(2);
        self.0.toasts.add_toast(toast);
    }

    fn palette_files(&self) -> Vec<(String, String)> {
        let folder_files = self.0.sidebar.folder_files();
        let mut seen: std::collections::HashSet<PathBuf> = folder_files.iter().cloned().collect();
        let mut items: Vec<(String, String)> = folder_files
            .iter()
            .map(|p| (p.to_string_lossy().into_owned(), files::display_name(p)))
            .collect();
        for recent in self.0.prefs.borrow().recents.iter() {
            let p = PathBuf::from(recent);
            if seen.insert(p.clone()) {
                items.push((recent.clone(), files::display_name(&p)));
            }
        }
        items
    }

    // ── Window chrome ────────────────────────────────────────────────────────

    fn update_controls(&self) {
        // The sidebar header carries the start window buttons while it shows.
        let sidebar_visible = self.0.split.shows_sidebar() && !self.0.split.is_collapsed();
        self.0.start_controls.set_visible(!sidebar_visible);
    }

    fn setup_signals(&self) {
        let weak = self.weak();
        self.0.tab_view.connect_selected_page_notify(move |_| {
            if let Some(inner) = weak.upgrade() {
                let win = Win(inner);
                if win.split_on() {
                    win.0.active_pane.set(Pane::Primary);
                }
                win.refresh_active();
                if let Some(reader) = win.active_reader() {
                    reader.set_zoom(win.0.prefs.borrow().zoom_level, false);
                    reader.focus();
                }
                win.schedule_save();
            }
        });
        let weak = self.weak();
        self.0.tab_view.connect_close_page(move |_, page| {
            if let Some(inner) = weak.upgrade() {
                let win = Win(inner);
                let removed = {
                    let mut tabs = win.0.tabs.borrow_mut();
                    tabs.iter().position(|t| t.page == *page).map(|i| tabs.remove(i))
                };
                if let Some(tab) = removed {
                    if let Some(monitor) = tab.monitor.take() {
                        monitor.cancel();
                    }
                    if let Some(id) = tab.reload_timer.take() {
                        id.remove();
                    }
                }
                let weak = win.weak();
                glib::idle_add_local_once(move || {
                    if let Some(inner) = weak.upgrade() {
                        let win = Win(inner);
                        win.refresh_active();
                        win.schedule_save();
                    }
                });
            }
            glib::Propagation::Proceed
        });
        let weak = self.weak();
        self.0.tab_view.connect_page_reordered(move |_, _, _| {
            if let Some(inner) = weak.upgrade() {
                Win(inner).schedule_save();
            }
        });

        // Tab context menu.
        let menu = gio::Menu::new();
        let close = gio::Menu::new();
        close.append(Some("Close"), Some("win.tab-close"));
        close.append(Some("Close Others"), Some("win.tab-close-others"));
        close.append(Some("Close to the Right"), Some("win.tab-close-right"));
        close.append(Some("Close All"), Some("win.tab-close-all"));
        menu.append_section(None, &close);
        let panes = gio::Menu::new();
        panes.append(Some("Open in Left Pane"), Some("win.tab-left-pane"));
        panes.append(Some("Open in Right Pane"), Some("win.tab-right-pane"));
        menu.append_section(None, &panes);
        let path = gio::Menu::new();
        path.append(Some("Copy Path"), Some("win.tab-copy-path"));
        path.append(Some("Show in Folder"), Some("win.tab-show-in-folder"));
        menu.append_section(None, &path);
        self.0.tab_view.set_menu_model(Some(&menu));
        let weak = self.weak();
        self.0.tab_view.connect_setup_menu(move |_, page| {
            if let Some(inner) = weak.upgrade() {
                if page.is_some() {
                    inner.menu_page.replace(page.cloned());
                }
                let win = Win(inner);
                let n = win.0.tab_view.n_pages();
                let pos = page.map(|p| win.0.tab_view.page_position(p)).unwrap_or(0);
                for (name, enabled) in
                    [("tab-close-others", n > 1), ("tab-close-right", pos + 1 < n)]
                {
                    if let Some(action) =
                        win.0.window.lookup_action(name).and_downcast::<gio::SimpleAction>()
                    {
                        action.set_enabled(enabled);
                    }
                }
            }
        });

        let weak = self.weak();
        self.0.split.connect_show_sidebar_notify(move |split| {
            if let Some(inner) = weak.upgrade() {
                let win = Win(inner);
                win.update_controls();
                if let Some(action) = win.0.window.lookup_action("toggle-sidebar") {
                    action.change_state(&split.shows_sidebar().to_variant());
                }
                win.schedule_save();
            }
        });
        let weak = self.weak();
        self.0.split.connect_collapsed_notify(move |_| {
            if let Some(inner) = weak.upgrade() {
                Win(inner).update_controls();
            }
        });

        let weak = self.weak();
        adw::StyleManager::default().connect_dark_notify(move |_| {
            if let Some(inner) = weak.upgrade() {
                Win(inner).apply_visuals();
            }
        });
        let weak = self.weak();
        self.0.window.connect_scale_factor_notify(move |_| {
            if let Some(inner) = weak.upgrade() {
                Win(inner).apply_visuals();
            }
        });

        // Panes become active on any click inside them.
        for (pane, widget) in [
            (Pane::Primary, self.0.primary_pane.clone()),
            (Pane::Secondary, self.0.secondary_box.clone()),
        ] {
            let click = gtk::GestureClick::new();
            click.set_propagation_phase(gtk::PropagationPhase::Capture);
            click.set_button(0);
            let weak = self.weak();
            click.connect_pressed(move |_, _, _, _| {
                if let Some(inner) = weak.upgrade() {
                    let win = Win(inner);
                    if win.split_on() {
                        win.set_active_pane(pane);
                    }
                }
            });
            widget.add_controller(click);
        }

        // Drop files or a folder anywhere in the window.
        let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let weak = self.weak();
        drop.connect_drop(move |_, value, _, _| {
            let (Some(inner), Ok(list)) = (weak.upgrade(), value.get::<gdk::FileList>()) else {
                return false;
            };
            let win = Win(inner);
            win.0.welcome.set_drag_over(false);
            let paths: Vec<PathBuf> = list.files().iter().filter_map(|f| f.path()).collect();
            let docs: Vec<&PathBuf> = paths.iter().filter(|p| files::is_document(p)).collect();
            for (i, path) in docs.iter().enumerate() {
                win.open_path(path, None, i + 1 == docs.len());
            }
            if docs.is_empty() && paths.len() == 1 && paths[0].is_dir() {
                win.open_folder(&paths[0], true);
            }
            true
        });
        let weak = self.weak();
        drop.connect_enter(move |_, _, _| {
            if let Some(inner) = weak.upgrade() {
                inner.welcome.set_drag_over(true);
            }
            gdk::DragAction::COPY
        });
        let weak = self.weak();
        drop.connect_leave(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.welcome.set_drag_over(false);
            }
        });
        self.0.window.add_controller(drop);

        // Esc closes find; holding Ctrl shows the shortcut cheat sheet.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        keys.connect_key_pressed(move |_, key, _, state| {
            let Some(inner) = weak.upgrade() else { return glib::Propagation::Proceed };
            let win = Win(inner);
            let is_ctrl = matches!(key, gdk::Key::Control_L | gdk::Key::Control_R);
            let others = state
                & (gdk::ModifierType::SHIFT_MASK
                    | gdk::ModifierType::ALT_MASK
                    | gdk::ModifierType::SUPER_MASK);
            if is_ctrl && others.is_empty() {
                win.arm_cheat_sheet();
            } else {
                win.hide_cheat_sheet();
            }
            if key == gdk::Key::Escape
                && let Some(reader) = win.active_reader()
                && reader.find_is_open()
            {
                reader.close_find();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        let weak = self.weak();
        keys.connect_key_released(move |_, key, _, _| {
            if matches!(key, gdk::Key::Control_L | gdk::Key::Control_R)
                && let Some(inner) = weak.upgrade()
            {
                Win(inner).hide_cheat_sheet();
            }
        });
        self.0.window.add_controller(keys);
        // Ctrl held for a Ctrl+wheel zoom or a Ctrl+click is not a request for the cheat sheet.
        let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        wheel.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        wheel.connect_scroll(move |_, _, _| {
            if let Some(inner) = weak.upgrade() {
                Win(inner).hide_cheat_sheet();
            }
            glib::Propagation::Proceed
        });
        self.0.window.add_controller(wheel);
        let click = gtk::GestureClick::new();
        click.set_button(0);
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        click.connect_pressed(move |_, _, _, _| {
            if let Some(inner) = weak.upgrade() {
                Win(inner).hide_cheat_sheet();
            }
        });
        self.0.window.add_controller(click);
        let weak = self.weak();
        self.0.window.connect_is_active_notify(move |w| {
            if !w.is_active()
                && let Some(inner) = weak.upgrade()
            {
                Win(inner).hide_cheat_sheet();
            }
        });

        let weak = self.weak();
        self.0.window.connect_close_request(move |_| {
            if let Some(inner) = weak.upgrade() {
                Win(inner).save_now();
            }
            glib::Propagation::Proceed
        });
    }

    fn arm_cheat_sheet(&self) {
        if self.0.cheat_timer.borrow().is_some() || self.0.cheat.is_visible() {
            return;
        }
        let weak = self.weak();
        let id = glib::timeout_add_local_once(Duration::from_millis(700), move || {
            if let Some(inner) = weak.upgrade() {
                inner.cheat_timer.take();
                inner.cheat.set_visible(true);
            }
        });
        self.0.cheat_timer.replace(Some(id));
    }

    fn hide_cheat_sheet(&self) {
        if let Some(id) = self.0.cheat_timer.take() {
            id.remove();
        }
        self.0.cheat.set_visible(false);
    }

    // ── Actions ──────────────────────────────────────────────────────────────

    fn add_action(&self, name: &str, f: impl Fn(&Win) + 'static) -> gio::SimpleAction {
        let action = gio::SimpleAction::new(name, None);
        let weak = self.weak();
        action.connect_activate(move |_, _| {
            if let Some(inner) = weak.upgrade() {
                f(&Win(inner));
            }
        });
        self.0.window.add_action(&action);
        action
    }

    fn add_string_action(&self, name: &str, f: impl Fn(&Win, &str) + 'static) {
        let action = gio::SimpleAction::new(name, Some(glib::VariantTy::STRING));
        let weak = self.weak();
        action.connect_activate(move |_, param| {
            if let (Some(inner), Some(value)) = (weak.upgrade(), param.and_then(|p| p.str())) {
                f(&Win(inner), value);
            }
        });
        self.0.window.add_action(&action);
    }

    fn add_choice_action(&self, name: &str, initial: &str, f: impl Fn(&Win, &str) + 'static) {
        let action = gio::SimpleAction::new_stateful(
            name,
            Some(glib::VariantTy::STRING),
            &initial.to_variant(),
        );
        let weak = self.weak();
        action.connect_change_state(move |action, value| {
            let (Some(inner), Some(value)) =
                (weak.upgrade(), value.and_then(|v| v.str().map(str::to_string)))
            else {
                return;
            };
            action.set_state(&value.to_variant());
            f(&Win(inner), &value);
        });
        self.0.window.add_action(&action);
    }

    fn add_toggle_action(&self, name: &str, initial: bool, f: impl Fn(&Win, bool) + 'static) {
        let action = gio::SimpleAction::new_stateful(name, None, &initial.to_variant());
        let weak = self.weak();
        action.connect_change_state(move |action, value| {
            let (Some(inner), Some(on)) = (weak.upgrade(), value.and_then(|v| v.get::<bool>()))
            else {
                return;
            };
            let changed = action.state().and_then(|s| s.get::<bool>()) != Some(on);
            action.set_state(&on.to_variant());
            if changed {
                f(&Win(inner), on);
            }
        });
        self.0.window.add_action(&action);
    }

    fn menu_tab(&self) -> Option<Rc<Tab>> {
        let page = self.0.menu_page.borrow().clone().or_else(|| self.0.tab_view.selected_page());
        page.and_then(|p| self.tab_for_page(&p))
    }

    fn setup_actions(
        &self,
        _sidebar_toggle: &gtk::ToggleButton,
        _split_toggle: &gtk::ToggleButton,
    ) {
        self.add_action("open-file", |w| w.open_file_dialog());
        self.add_action("open-folder", |w| w.open_folder_dialog());
        self.add_action("none", |_| {}).set_enabled(false);
        self.add_string_action("open-path", |w, p| w.open_path(Path::new(p), None, true));
        self.add_string_action("reload-path", |w, p| {
            if let Some(tab) = w.tab_for_path(Path::new(p)) {
                tab.page.set_loading(true);
                w.load_tab(&tab, false);
            }
        });
        self.add_string_action("close-path", |w, p| {
            if let Some(tab) = w.tab_for_path(Path::new(p)) {
                w.close_tab(&tab);
            }
        });
        self.add_string_action("copy-path", |w, p| w.copy_path(p));
        self.add_string_action("show-in-folder", |w, p| w.show_in_folder(Path::new(p)));
        self.add_string_action("remove-recent", |w, p| {
            w.update_prefs(|prefs| prefs.remove_recent(p));
            w.refresh_recents();
        });
        self.add_action("close-tab", |w| match w.active_tab() {
            Some(tab) => w.close_tab(&tab),
            None => w.0.window.close(),
        });
        self.add_action("next-tab", |w| w.cycle_tab(1));
        self.add_action("prev-tab", |w| w.cycle_tab(-1));
        let select = gio::SimpleAction::new("select-tab", Some(glib::VariantTy::INT32));
        let weak = self.weak();
        select.connect_activate(move |_, param| {
            let (Some(inner), Some(n)) = (weak.upgrade(), param.and_then(|p| p.get::<i32>()))
            else {
                return;
            };
            let view = &inner.tab_view;
            let count = view.n_pages();
            if count == 0 {
                return;
            }
            let index = if n >= 9 { count - 1 } else { (n - 1).min(count - 1) };
            view.set_selected_page(&view.nth_page(index));
        });
        self.0.window.add_action(&select);

        let sidebar_open = self.0.prefs.borrow().sidebar_open;
        self.add_toggle_action("toggle-sidebar", sidebar_open, |w, on| {
            w.0.split.set_show_sidebar(on);
            w.update_prefs(|p| p.sidebar_open = on);
        });
        let mode = self.0.prefs.borrow().sidebar_mode;
        self.add_choice_action("sidebar-mode", mode.id(), |w, id| {
            if let Some(mode) = SidebarMode::from_id(id) {
                w.update_prefs(|p| p.sidebar_mode = mode);
                w.0.sidebar.set_mode(mode);
            }
        });
        let goto = gio::SimpleAction::new("goto-heading", Some(glib::VariantTy::UINT32));
        let weak = self.weak();
        goto.connect_activate(move |_, param| {
            let (Some(inner), Some(i)) = (weak.upgrade(), param.and_then(|p| p.get::<u32>()))
            else {
                return;
            };
            if let Some(reader) = Win(inner).active_reader() {
                reader.scroll_to_heading(i as usize);
            }
        });
        self.0.window.add_action(&goto);

        self.add_action("find", |w| {
            if let Some(reader) = w.active_reader() {
                reader.open_find();
            }
        });
        self.add_action("find-next", |w| {
            if let Some(reader) = w.active_reader() {
                reader.find_step(1);
            }
        });
        self.add_action("find-prev", |w| {
            if let Some(reader) = w.active_reader() {
                reader.find_step(-1);
            }
        });
        self.add_action("palette", |w| dialogs::present_palette(&w.0.window, w.palette_files()));
        self.add_action("zoom-in", |w| w.zoom(1));
        self.add_action("zoom-out", |w| w.zoom(-1));
        self.add_action("zoom-reset", |w| w.zoom(0));
        let wide = self.0.prefs.borrow().wide_mode;
        self.add_toggle_action("wide-mode", wide, |w, on| {
            w.update_prefs(|p| p.wide_mode = on);
            w.apply_visuals();
            w.refresh_active();
        });
        self.add_toggle_action("split-view", false, |w, on| w.set_split(on));
        self.add_action("close-split", |w| w.set_split(false));

        let prefs = self.0.prefs.borrow().clone();
        self.add_choice_action("theme", prefs.theme.id(), |w, id| {
            if let Some(v) = ThemeMode::from_id(id) {
                w.update_prefs(|p| p.theme = v);
                w.apply_visuals();
            }
        });
        self.add_choice_action("content-font", prefs.content_font.id(), |w, id| {
            if let Some(v) = ContentFont::from_id(id) {
                w.update_prefs(|p| p.content_font = v);
                w.apply_visuals();
            }
        });
        self.add_choice_action("code-font", prefs.code_font.id(), |w, id| {
            if let Some(v) = CodeFont::from_id(id) {
                w.update_prefs(|p| p.code_font = v);
                w.apply_visuals();
            }
        });
        self.add_choice_action("interface-scale", prefs.interface_scale.id(), |w, id| {
            if let Some(v) = InterfaceScale::from_id(id) {
                w.update_prefs(|p| p.interface_scale = v);
                w.apply_visuals();
            }
        });
        self.add_choice_action("reading-width", prefs.reading_width.id(), |w, id| {
            if let Some(v) = ReadingWidth::from_id(id) {
                w.update_prefs(|p| p.reading_width = v);
                w.apply_visuals();
            }
        });
        self.add_action("reset-appearance", |w| {
            w.update_prefs(|p| p.reset_appearance());
            let prefs = w.0.prefs.borrow().clone();
            for (name, value) in [
                ("theme", prefs.theme.id()),
                ("content-font", prefs.content_font.id()),
                ("code-font", prefs.code_font.id()),
                ("interface-scale", prefs.interface_scale.id()),
                ("reading-width", prefs.reading_width.id()),
            ] {
                if let Some(action) =
                    w.0.window.lookup_action(name).and_downcast::<gio::SimpleAction>()
                {
                    action.set_state(&value.to_variant());
                }
            }
            w.apply_visuals();
        });
        self.add_action("settings", |w| {
            let prefs = w.0.prefs.borrow().clone();
            dialogs::present_settings(&w.0.window, &prefs);
        });
        self.add_action("shortcuts", |w| dialogs::present_shortcuts(&w.0.window));
        self.add_action("about", |w| dialogs::present_about(&w.0.window));
        self.add_action("fullscreen", |w| {
            if w.0.window.is_fullscreen() {
                w.0.window.unfullscreen();
            } else {
                w.0.window.fullscreen();
            }
        });

        // Tab context menu.
        self.add_action("tab-close", |w| {
            if let Some(tab) = w.menu_tab() {
                w.close_tab(&tab);
            }
        });
        self.add_action("tab-close-others", |w| {
            let Some(keep) = w.menu_tab() else { return };
            let others: Vec<Rc<Tab>> =
                w.0.tabs.borrow().iter().filter(|t| !Rc::ptr_eq(t, &keep)).cloned().collect();
            for tab in others {
                w.close_tab(&tab);
            }
        });
        self.add_action("tab-close-right", |w| {
            let Some(tab) = w.menu_tab() else { return };
            let view = &w.0.tab_view;
            let pos = view.page_position(&tab.page);
            let right: Vec<adw::TabPage> =
                ((pos + 1)..view.n_pages()).map(|i| view.nth_page(i)).collect();
            for page in right {
                view.close_page(&page);
            }
        });
        self.add_action("tab-close-all", |w| {
            let tabs: Vec<Rc<Tab>> = w.0.tabs.borrow().clone();
            for tab in tabs {
                w.close_tab(&tab);
            }
        });
        self.add_action("tab-left-pane", |w| {
            if let Some(tab) = w.menu_tab() {
                w.0.tab_view.set_selected_page(&tab.page);
                w.set_active_pane(Pane::Primary);
            }
        });
        self.add_action("tab-right-pane", |w| {
            if let Some(tab) = w.menu_tab() {
                let path = tab.path.clone();
                w.set_secondary(Some(&path), None);
            }
        });
        self.add_action("tab-copy-path", |w| {
            if let Some(tab) = w.menu_tab() {
                w.copy_path(&tab.path.to_string_lossy());
            }
        });
        self.add_action("tab-show-in-folder", |w| {
            if let Some(tab) = w.menu_tab() {
                w.show_in_folder(&tab.path);
            }
        });
    }

    /// Debug aid for screenshots: scroll the active reader to an anchor or row number.
    pub fn debug_scroll(&self, target: &str) {
        if let Some(reader) = self.active_reader() {
            match target.parse::<usize>() {
                Ok(row) => reader.scroll_to_row(row),
                Err(_) => {
                    reader.scroll_to_anchor(target);
                }
            }
        }
    }

    /// Debug aid for screenshots: search the active reader, or show the cheat sheet.
    pub fn debug_find(&self, query: &str) {
        if let Some(reader) = self.active_reader() {
            reader.debug_find(query);
        }
    }

    pub fn debug_cheat_sheet(&self) {
        self.0.cheat.set_visible(true);
    }

    /// Reopen the last session: tabs, the active tab, the folder and the split pane.
    pub fn restore_session(&self) {
        let prefs = self.0.prefs.borrow().clone();
        self.0.restoring.set(true);
        if let Some(folder) = prefs.last_folder.as_deref().map(PathBuf::from).filter(|p| p.is_dir())
        {
            self.open_folder(&folder, false);
        }
        for path in prefs.session_tabs.iter().map(PathBuf::from) {
            if path.exists() && files::is_document(&path) {
                let active = prefs.session_active_tab_path.as_deref()
                    == Some(path.to_string_lossy().as_ref());
                self.ensure_tab(&path, None, active);
            }
        }
        if prefs.session_split_view && self.0.tab_view.n_pages() > 0 {
            let secondary =
                prefs.session_secondary_pane_path.map(PathBuf::from).filter(|p| p.exists());
            self.set_secondary(secondary.as_deref(), None);
        }
        self.0.restoring.set(false);
        self.refresh_active();
    }
}

pub fn set_accels(app: &adw::Application) {
    let accels: &[(&str, &[&str])] = &[
        ("win.open-file", &["<Control>o"]),
        ("win.open-folder", &["<Control><Shift>o"]),
        ("win.close-tab", &["<Control>w"]),
        ("win.palette", &["<Control>k", "<Control><Shift>p"]),
        ("win.find", &["<Control>f"]),
        ("win.find-next", &["<Control>g", "F3"]),
        ("win.find-prev", &["<Control><Shift>g", "<Shift>F3"]),
        ("win.toggle-sidebar", &["<Control>b"]),
        ("win.zoom-in", &["<Control>equal", "<Control>plus", "<Control>KP_Add"]),
        ("win.zoom-out", &["<Control>minus", "<Control>KP_Subtract"]),
        ("win.zoom-reset", &["<Control>0", "<Control>KP_0"]),
        ("win.shortcuts", &["<Control>slash", "<Control>question"]),
        ("win.settings", &["<Control>comma"]),
        ("win.next-tab", &["<Control><Alt>Right"]),
        ("win.prev-tab", &["<Control><Alt>Left"]),
        ("win.wide-mode", &["<Control><Shift>w"]),
        ("win.split-view", &["<Control>backslash"]),
        ("win.fullscreen", &["F11"]),
        ("app.quit", &["<Control>q"]),
    ];
    for (action, keys) in accels {
        app.set_accels_for_action(action, keys);
    }
    for n in 1..=9 {
        app.set_accels_for_action(&format!("win.select-tab({n})"), &[&format!("<Control>{n}")]);
    }
}
