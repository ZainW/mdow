//! A document window: the header with its tabs, the sidebar, the document card with its readers
//! (and an optional side-by-side pane), window actions and keyboard shortcuts, per-pane history,
//! live reload and session persistence.

use crate::companion::panel::Panel as CompanionPanel;
use crate::companion::service::Runtime as CompanionRuntime;
use crate::dialogs;
use crate::doc::{self, Doc};
use crate::files::{self, ErrorKind};
use crate::media;
use crate::omarchy;
use crate::prefs::{
    self, CodeFont, ContentFont, InterfaceScale, LineSpacing, Prefs, ReadingWidth, SidebarMode,
    ThemeMode,
};
use crate::print;
use crate::reader::{self, Columns, Reader, ReaderHost};
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

thread_local! {
    /// Every open window, oldest first. This is what keeps a window's state alive.
    static WINDOWS: RefCell<Vec<Rc<WinInner>>> = const { RefCell::new(Vec::new()) };
    static STYLESHEET: gtk::CssProvider = gtk::CssProvider::new();
    static LOADED_CSS: RefCell<String> = const { RefCell::new(String::new()) };
    static OMARCHY: RefCell<Option<omarchy::Theme>> = const { RefCell::new(None) };
    static OMARCHY_MONITOR: RefCell<Option<gio::FileMonitor>> = const { RefCell::new(None) };
}

/// Reads the Omarchy theme, and follows `omarchy theme set` from then on.
pub fn init_omarchy() {
    OMARCHY.with_borrow_mut(|theme| *theme = omarchy::load());
    let monitor = omarchy::watch(|| {
        OMARCHY.with_borrow_mut(|theme| *theme = omarchy::load());
        // A theme swap shouldn't fade every colour separately.
        let settings = gtk::Settings::default();
        let animations = settings.as_ref().is_some_and(|s| s.is_gtk_enable_animations());
        if let Some(settings) = &settings {
            settings.set_gtk_enable_animations(false);
        }
        Win::restyle_all();
        glib::timeout_add_local_once(Duration::from_millis(300), move || {
            if let Some(settings) = settings {
                settings.set_gtk_enable_animations(animations);
            }
        });
    });
    OMARCHY_MONITOR.with_borrow_mut(|m| *m = monitor);
}

pub fn on_omarchy() -> bool {
    OMARCHY.with_borrow(|theme| theme.is_some())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pane {
    Primary,
    Secondary,
}

/// A spot to come back to: the document, and the row at the top of the view.
#[derive(Clone, Debug)]
struct Place {
    path: PathBuf,
    position: Option<(usize, f64)>,
}

#[derive(Default)]
struct History {
    back: Vec<Place>,
    forward: Vec<Place>,
}

pub struct Tab {
    location: RefCell<PathBuf>,
    history: RefCell<History>,
    restore: Cell<Option<(usize, f64)>>,
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

impl Tab {
    fn path(&self) -> PathBuf {
        self.location.borrow().clone()
    }
}

struct SecondaryPane {
    path: PathBuf,
    reader: Reader,
    generation: Cell<u64>,
}

pub struct WinInner {
    pub window: adw::ApplicationWindow,
    prefs: Rc<RefCell<Prefs>>,
    prefs_path: PathBuf,
    split: adw::OverlaySplitView,
    sidebar: Rc<Sidebar>,
    tab_view: adw::TabView,
    tabs: RefCell<Vec<Rc<Tab>>>,
    card: gtk::Box,
    companion: Rc<CompanionPanel>,
    companion_paned: gtk::Paned,
    menu_button: gtk::MenuButton,
    header_leading: gtk::Box,
    header_spacer: gtk::Box,
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
    secondary_history: RefCell<History>,
    active_pane: Cell<Pane>,
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
    fn open_document(&self, path: PathBuf, anchor: Option<String>, new_tab: bool) {
        if let Some(inner) = self.0.upgrade() {
            let win = Win(inner);
            if new_tab {
                win.open_path(&path, anchor, true);
            } else {
                win.navigate(&path, anchor);
            }
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
    crate::set_accessible_name(&b, tooltip);
    b.set_valign(gtk::Align::Center);
    b
}

fn icon_toggle(icon: &str, tooltip: &str, action: &str) -> gtk::ToggleButton {
    let b = gtk::ToggleButton::new();
    b.set_icon_name(icon);
    b.add_css_class("flat");
    b.add_css_class("mdow-icon-button");
    b.set_tooltip_text(Some(tooltip));
    crate::set_accessible_name(&b, tooltip);
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
    pub fn new(app: &adw::Application, prefs: Rc<RefCell<Prefs>>, prefs_path: PathBuf) -> Win {
        let window = adw::ApplicationWindow::new(app);
        window.add_css_class("mdow");
        window.set_title(Some("Mdow"));
        window.set_icon_name(Some(crate::APP_ID));
        window.set_size_request(600, 400);
        // On Omarchy, Hyprland draws the frame and Super+W closes the window, as for every app.
        let on_omarchy = on_omarchy();
        window.set_decorated(!on_omarchy);
        {
            let prefs = prefs.borrow();
            window.set_default_size(prefs.window.width.max(600), prefs.window.height.max(400));
            if prefs.window.maximized {
                window.maximize();
            }
        }

        let first_window = WINDOWS.with_borrow(|windows| windows.is_empty());
        if first_window && let Some(display) = gdk::Display::default() {
            STYLESHEET.with(|css| {
                gtk::style_context_add_provider_for_display(
                    &display,
                    css,
                    gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
                );
            });
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
        start_controls.set_visible(!on_omarchy);
        end_controls.set_visible(!on_omarchy);
        let sidebar_toggle =
            icon_toggle("sidebar-show-symbolic", "Toggle sidebar (Ctrl+B)", "win.toggle-sidebar");
        sidebar_toggle.add_css_class("mdow-quiet");
        let back = flat_icon_button("go-previous-symbolic", "Back (Ctrl+[)");
        back.set_action_name(Some("win.go-back"));
        let forward = flat_icon_button("go-next-symbolic", "Forward (Ctrl+])");
        forward.set_action_name(Some("win.go-forward"));

        let search = gtk::Button::new();
        search.add_css_class("mdow-search");
        search.set_action_name(Some("win.palette"));
        search.set_tooltip_text(Some("Quick Open (Ctrl+K)"));
        search.set_valign(gtk::Align::Center);
        search.set_hexpand(false);
        let search_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let search_icon = gtk::Image::from_icon_name("system-search-symbolic");
        search_icon.set_pixel_size(12);
        search_content.append(&search_icon);
        let search_label = gtk::Label::new(Some("Search"));
        search_label.set_hexpand(true);
        search_label.set_xalign(0.0);
        search_content.append(&search_label);
        let search_keys = gtk::Label::new(Some("Ctrl K"));
        search_keys.add_css_class("mdow-subtle");
        search_content.append(&search_keys);
        search.set_child(Some(&search_content));

        let recent_menu = gio::Menu::new();
        let menu = primary_menu(&recent_menu);
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("view-more-horizontal-symbolic");
        menu_button.add_css_class("flat");
        menu_button.add_css_class("mdow-icon-button");
        menu_button.set_tooltip_text(Some("More"));
        crate::set_accessible_name(&menu_button, "More");
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
        crate::set_accessible_name(&zoom_out, "Zoom out");
        let zoom_menu_label = gtk::Button::with_label("100%");
        zoom_menu_label.set_action_name(Some("win.zoom-reset"));
        zoom_menu_label.set_tooltip_text(Some("Reset zoom"));
        zoom_menu_label.set_hexpand(true);
        let zoom_in = gtk::Button::from_icon_name("zoom-in-symbolic");
        zoom_in.set_action_name(Some("win.zoom-in"));
        zoom_in.set_tooltip_text(Some("Zoom in"));
        crate::set_accessible_name(&zoom_in, "Zoom in");
        for b in [&zoom_out, &zoom_menu_label, &zoom_in] {
            b.add_css_class("flat");
            zoom_row.append(b);
        }
        if let Some(popover) = menu_button.popover().and_downcast::<gtk::PopoverMenu>() {
            popover.add_child(&zoom_row, "zoom");
            popover.set_has_arrow(false);
        }

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        actions.add_css_class("mdow-tab-actions");
        actions.set_valign(gtk::Align::Fill);
        actions.append(&search);
        let companion_toggle = icon_toggle(
            "mdow-companion-symbolic",
            "Companion (Ctrl+Alt+0)",
            "win.toggle-companion",
        );
        companion_toggle.add_css_class("mdow-quiet");
        actions.append(&companion_toggle);
        actions.append(&menu_button);

        let header_leading = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        header_leading.set_valign(gtk::Align::Center);
        start_controls.set_margin_end(6);
        header_leading.append(&start_controls);
        header_leading.append(&sidebar_toggle);
        header_leading.append(&back);
        header_leading.append(&forward);
        // Keeps the tabs lined up with the document card while the sidebar is open.
        let header_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        header_spacer.add_css_class("mdow-header-spacer");
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        header.add_css_class("mdow-tabbar");
        header.append(&header_leading);
        header.append(&header_spacer);
        header.append(&tab_bar);
        header.append(&actions);
        end_controls.set_margin_start(6);
        header.append(&end_controls);
        let handle = gtk::WindowHandle::new();
        handle.set_child(Some(&header));

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

        // The document's card, inset from the window like a sheet of paper.
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("mdow-card");
        card.set_overflow(gtk::Overflow::Hidden);
        card.append(&main_stack);

        CompanionRuntime::shared().set_prefs(prefs.clone(), prefs_path.clone());
        let companion = CompanionPanel::new(Rc::new(Style {
            palette: Rc::new(Palette::light()),
            fs: theme::READER_FONT_SIZE,
            code_family: prefs.borrow().code_font.pango_family(),
            scale_factor: 1,
            line_height: prefs.borrow().line_spacing.height(),
        }));
        // The trailing panel keeps its width as the window resizes; the card takes the rest.
        let companion_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        companion_paned.add_css_class("mdow-companion-paned");
        companion_paned.set_start_child(Some(&card));
        companion_paned.set_end_child(Some(&companion.root));
        companion_paned.set_resize_start_child(true);
        companion_paned.set_resize_end_child(false);
        companion_paned.set_shrink_start_child(false);
        companion_paned.set_shrink_end_child(false);
        card.set_hexpand(true);

        let split = adw::OverlaySplitView::new();
        split.set_sidebar(Some(&sidebar.root));
        split.set_content(Some(&companion_paned));
        split.set_vexpand(true);
        let width = theme::sidebar_width(prefs.borrow().interface_scale);
        split.set_min_sidebar_width(width);
        split.set_max_sidebar_width(width);
        split.set_show_sidebar(prefs.borrow().sidebar_open);
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            720.0,
            adw::LengthUnit::Px,
        ));
        breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        let cheat = dialogs::cheat_sheet();
        cheat.set_visible(false);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
        body.append(&handle);
        body.append(&split);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&body));
        overlay.add_overlay(&cheat);
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&overlay));
        window.set_content(Some(&toasts));

        let inner = Rc::new(WinInner {
            window,
            prefs,
            prefs_path,
            split,
            sidebar,
            tab_view,
            tabs: RefCell::new(Vec::new()),
            card,
            companion,
            companion_paned,
            menu_button,
            header_leading,
            header_spacer,
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
            secondary_history: RefCell::new(History::default()),
            active_pane: Cell::new(Pane::Primary),
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
        WINDOWS.with_borrow_mut(|windows| windows.push(inner.clone()));
        let win = Win(inner);
        let weak = win.weak();
        win.0.companion.set_open_document(Rc::new(move |path: &Path| {
            if let Some(inner) = weak.upgrade() {
                Win(inner).open_path(path, None, true);
            }
        }));
        win.setup_actions();
        win.setup_signals();
        win.apply_visuals();
        win.refresh_recents();
        let mode = win.0.prefs.borrow().sidebar_mode;
        win.0.sidebar.set_mode(mode);
        win.update_controls();
        win.update_history_actions();
        win.update_document_actions();
        win.update_zoom_label();
        win
    }

    /// Every open window, oldest first.
    pub fn all() -> Vec<Win> {
        WINDOWS.with_borrow(|windows| windows.iter().cloned().map(Win).collect())
    }

    pub fn restyle_all() {
        for win in Win::all() {
            win.apply_visuals();
        }
    }

    /// The oldest window keeps the session, so closing a second window doesn't forget the tabs.
    fn is_primary(&self) -> bool {
        Win::all().first().is_some_and(|w| Rc::ptr_eq(&w.0, &self.0))
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

    /// The Omarchy theme, unless Settings asks for Mdow's own light or dark.
    fn omarchy_theme(&self) -> Option<omarchy::Theme> {
        if self.0.prefs.borrow().theme != ThemeMode::System {
            return None;
        }
        OMARCHY.with_borrow(Clone::clone)
    }

    fn palette(&self) -> Rc<Palette> {
        Rc::new(match self.omarchy_theme() {
            Some(theme) => Palette::from_omarchy(&theme),
            None => Palette::for_dark(adw::StyleManager::default().is_dark()),
        })
    }

    fn style(&self) -> Rc<Style> {
        let prefs = self.0.prefs.borrow();
        Rc::new(Style {
            palette: self.palette(),
            fs: theme::reader_font_size(prefs.zoom_level),
            code_family: prefs.code_font.pango_family(),
            scale_factor: self.0.window.scale_factor().max(1),
            line_height: prefs.line_spacing.height(),
        })
    }

    /// The reading column. Prose is capped at a number of characters in the chosen font and
    /// size, so the line length holds when either changes.
    fn column(&self) -> Option<Columns> {
        let prefs = self.0.prefs.borrow();
        if prefs.wide_mode {
            return None;
        }
        let wide = prefs.reading_width.px();
        let text = reader::text_column(
            &self.0.window.pango_context(),
            dialogs::content_pango_family(prefs.content_font),
            theme::reader_font_size(prefs.zoom_level),
            prefs.reading_width.measure(),
        );
        Some(Columns { text: text.min(wide), wide })
    }

    fn new_reader(&self) -> Reader {
        let reader = Reader::new(self.style(), self.column());
        reader.set_host(self.reader_host());
        reader.set_zoom(self.0.prefs.borrow().zoom_level, false);
        reader
    }

    /// Theme, fonts, scale, zoom or width changed: restyle everything.
    fn apply_visuals(&self) {
        let theme = self.0.prefs.borrow().theme;
        let scheme = match (theme, self.omarchy_theme()) {
            (ThemeMode::System, Some(omarchy)) if omarchy.dark => adw::ColorScheme::ForceDark,
            (ThemeMode::System, Some(_)) => adw::ColorScheme::ForceLight,
            (ThemeMode::System, None) => adw::ColorScheme::Default,
            (ThemeMode::Light, _) => adw::ColorScheme::ForceLight,
            (ThemeMode::Dark, _) => adw::ColorScheme::ForceDark,
        };
        let manager = adw::StyleManager::default();
        if manager.color_scheme() != scheme {
            // Changing the scheme re-enters through `notify::dark`.
            manager.set_color_scheme(scheme);
        }
        let palette = self.palette();
        let css = theme::stylesheet(&palette, &self.0.prefs.borrow());
        // Each window restyles itself, but they share one stylesheet.
        LOADED_CSS.with_borrow_mut(|loaded| {
            if *loaded != css {
                STYLESHEET.with(|provider| provider.load_from_string(&css));
                *loaded = css;
            }
        });
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
        self.0.companion.set_style(style);
        self.update_controls();
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
            .map(|t| t.path().to_string_lossy().into_owned())
            .collect();
        let active = self.active_tab().map(|t| t.path().to_string_lossy().into_owned());
        let secondary =
            self.0.secondary.borrow().as_ref().map(|s| s.path.to_string_lossy().into_owned());
        let window = &self.0.window;
        let is_primary = self.is_primary();
        let mut prefs = self.0.prefs.borrow_mut();
        if is_primary {
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
        self.0.tabs.borrow().iter().find(|t| t.path() == path).cloned()
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
        self.active_tab().map(|t| t.path())
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
        if !self.0.prefs.borrow().open_in_tabs
            && self.tab_for_path(&path).is_none()
            && self.0.tab_view.n_pages() > 0
            && !self.0.restoring.get()
            && let Some(app) = self.0.window.application().and_downcast::<adw::Application>()
        {
            let win = Win::new(&app, self.0.prefs.clone(), self.0.prefs_path.clone());
            win.0.window.present();
            win.open_path(&path, anchor, activate);
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
            location: RefCell::new(path.to_path_buf()),
            history: RefCell::new(History::default()),
            restore: Cell::new(None),
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
        let path = tab.path();
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
                    } else if let Some((row, offset)) = tab.restore.take() {
                        tab.reader.restore_position(row, offset);
                    }
                    win.update_prefs(|p| p.push_recent(&path.to_string_lossy()));
                    win.refresh_recents();
                    if win.active_tab().is_some_and(|t| Rc::ptr_eq(&t, &tab)) {
                        win.refresh_active();
                        if !had_doc && win.0.window.visible_dialog().is_none() {
                            tab.reader.focus();
                        }
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
        let view = error_view(&tab.path(), kind, &tab.path().to_string_lossy());
        tab.stack.add_named(&view, Some("error"));
        tab.stack.set_visible_child_name("error");
        if self.active_tab().is_some_and(|t| Rc::ptr_eq(&t, tab)) {
            self.refresh_active();
        }
    }

    fn watch_tab(&self, tab: &Rc<Tab>) {
        let file = gio::File::for_path(tab.path());
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
            if inner.prefs.borrow().live_reload {
                Win(inner).schedule_reload(&tab);
            }
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
            if tab.path().exists() {
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
                    if tab.path().exists() {
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
        self.update_panes();
        self.update_history_actions();
        self.update_document_actions();
        self.0.companion.document_changed(
            active_path,
            self.0.sidebar.folder_root(),
            self.0.sidebar.folder_files(),
        );
    }

    fn set_companion_shown(&self, shown: bool) {
        let paned = &self.0.companion_paned;
        if shown {
            const COMPANION_WIDTH: i32 = 360;
            let available = paned.width().max(COMPANION_WIDTH * 2);
            let width = (available as f64 * 0.3).clamp(COMPANION_WIDTH as f64, 560.0) as i32;
            paned.set_position(available - width);
        }
        self.0.companion.set_shown(shown);
        self.update_controls();
    }

    // ── History ──────────────────────────────────────────────────────────────

    fn history_of_active<T>(&self, f: impl FnOnce(&mut History) -> T) -> Option<T> {
        if self.split_on() && self.0.active_pane.get() == Pane::Secondary {
            return Some(f(&mut self.0.secondary_history.borrow_mut()));
        }
        self.active_tab().map(|tab| f(&mut tab.history.borrow_mut()))
    }

    fn current_place(&self) -> Option<Place> {
        let path = self.active_path()?;
        Some(Place { path, position: self.active_reader().and_then(|r| r.position()) })
    }

    /// Follow a link in the active pane, as a browser does; Back returns to this spot.
    pub fn navigate(&self, path: &Path, anchor: Option<String>) {
        let path = files::normalize(path);
        if self.active_path().is_none() || !files::is_document(&path) {
            self.open_path(&path, anchor, true);
            return;
        }
        let in_secondary = self.split_on() && self.0.active_pane.get() == Pane::Secondary;
        if !in_secondary && self.tab_for_path(&path).is_some() {
            self.open_path(&path, anchor, true);
            return;
        }
        let Some(place) = self.current_place() else { return };
        self.history_of_active(|history| {
            history.back.push(place);
            history.forward.clear();
        });
        self.show_in_active_pane(&path, anchor, None);
    }

    fn show_in_active_pane(
        &self,
        path: &Path,
        anchor: Option<String>,
        position: Option<(usize, f64)>,
    ) {
        if self.split_on() && self.0.active_pane.get() == Pane::Secondary {
            self.show_in_secondary(path, anchor, position);
        } else if let Some(tab) = self.active_tab() {
            tab.location.replace(path.to_path_buf());
            tab.page.set_title(&files::display_name(path));
            tab.page.set_tooltip(&glib::markup_escape_text(&path.to_string_lossy()));
            tab.page.set_loading(true);
            tab.anchor.replace(anchor);
            tab.restore.set(position);
            tab.content_hash.set(0);
            self.watch_tab(&tab);
            self.load_tab(&tab, false);
            self.schedule_save();
        }
        self.refresh_active();
    }

    fn go_in_history(&self, back: bool) {
        let Some(current) = self.current_place() else { return };
        let target = self.history_of_active(|history| {
            let (from, to) = if back {
                (&mut history.back, &mut history.forward)
            } else {
                (&mut history.forward, &mut history.back)
            };
            let target = from.pop();
            if target.is_some() {
                to.push(current);
            }
            target
        });
        let Some(Some(target)) = target else { return };
        self.show_in_active_pane(&target.path, None, target.position);
    }

    fn update_history_actions(&self) {
        let (can_back, can_forward) = self
            .history_of_active(|h| (!h.back.is_empty(), !h.forward.is_empty()))
            .unwrap_or_default();
        self.set_action_enabled("go-back", can_back);
        self.set_action_enabled("go-forward", can_forward);
    }

    fn set_action_enabled(&self, name: &str, enabled: bool) {
        if let Some(action) = self.0.window.lookup_action(name).and_downcast::<gio::SimpleAction>()
        {
            action.set_enabled(enabled);
        }
    }

    /// The actions that need a document greyed out without one.
    fn update_document_actions(&self) {
        let has_document = self.active_path().is_some();
        for name in [
            "open-in-editor",
            "show-document",
            "open-terminal",
            "copy-document-path",
            "share",
            "reload",
            "print",
            "export-pdf",
            "previous-heading",
            "next-heading",
            "find",
        ] {
            self.set_action_enabled(name, has_document);
        }
    }

    // ── The document, outside Mdow ───────────────────────────────────────────

    fn toast(&self, text: &str) {
        let toast = adw::Toast::new(text);
        toast.set_timeout(3);
        self.0.toasts.add_toast(toast);
    }

    fn open_in_editor(&self) {
        let Some(path) = self.active_path() else { return };
        if on_omarchy() && omarchy::open_in_editor(&path) {
            return;
        }
        let file = gio::File::for_path(&path);
        gtk::FileLauncher::new(Some(&file)).launch(
            Some(&self.0.window),
            gio::Cancellable::NONE,
            |_| {},
        );
    }

    fn open_terminal(&self) {
        let Some(dir) = self.active_path().and_then(|p| p.parent().map(Path::to_path_buf)) else {
            return;
        };
        if !omarchy::open_terminal(&dir) {
            self.toast("Could not open a terminal");
        }
    }

    fn export_pdf_dialog(&self) {
        let Some(path) = self.active_path() else { return };
        let name = path.with_extension("pdf");
        let name = name.file_name().map(|n| n.to_string_lossy().into_owned());
        let dialog = gtk::FileDialog::builder()
            .title("Export as PDF")
            .modal(true)
            .initial_name(name.unwrap_or_else(|| "document.pdf".to_string()))
            .build();
        if let Some(dir) = path.parent() {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let weak = self.weak();
        dialog.save(Some(&self.0.window), gio::Cancellable::NONE, move |result| {
            let (Some(inner), Ok(file)) = (weak.upgrade(), result) else { return };
            let win = Win(inner);
            if let (Some(target), Some(reader)) = (file.path(), win.active_reader()) {
                win.toast(&match print::export_pdf(&win.0.window, &reader, &target) {
                    Ok(()) => format!("Exported {}", files::display_name(&target)),
                    Err(err) => format!("Could not export the PDF: {err}"),
                });
            }
        });
    }

    fn share(&self) {
        let Some(path) = self.active_path() else { return };
        if !omarchy::share_file(&path) {
            self.toast("Sharing needs Omarchy's share menu");
        }
    }

    // ── Split view ───────────────────────────────────────────────────────────

    fn update_panes(&self) {
        let split = self.split_on();
        self.0.primary_header.set_visible(split);
        self.0.split_line.set_visible(split);
        if split {
            let primary = self.active_tab().map(|t| files::display_name(&t.path()));
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
                // The next tab after the active one, else the previous.
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
                    .map(|t| t.path());
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

    /// Link navigation in the right pane: same reader, new document.
    fn show_in_secondary(
        &self,
        path: &Path,
        anchor: Option<String>,
        position: Option<(usize, f64)>,
    ) {
        let pane = self.0.secondary.borrow().as_ref().map(|old| {
            Rc::new(SecondaryPane {
                path: path.to_path_buf(),
                reader: old.reader.clone(),
                generation: Cell::new(old.generation.get()),
            })
        });
        match pane {
            Some(pane) => {
                self.0.secondary.replace(Some(pane.clone()));
                self.load_secondary(&pane, false, anchor, position);
            }
            None => self.set_secondary(Some(path), anchor),
        }
    }

    fn set_secondary(&self, path: Option<&Path>, anchor: Option<String>) {
        self.0.secondary_history.replace(History::default());
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
                self.load_secondary(&pane, false, anchor, None);
            }
            None => self.0.secondary_stack.set_visible_child_name("empty"),
        }
        self.refresh_active();
        self.schedule_save();
    }

    fn load_secondary(
        &self,
        pane: &Rc<SecondaryPane>,
        keep: bool,
        anchor: Option<String>,
        position: Option<(usize, f64)>,
    ) {
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
                } else if let Some((row, offset)) = position {
                    pane.reader.restore_position(row, offset);
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
            self.load_secondary(&pane, true, None, None);
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
            if reveal
                && win.0.tab_view.n_pages() == 0
                && let Some(readme) = win.0.sidebar.folder_files().into_iter().find(|p| {
                    p.parent() == Some(root.as_path())
                        && p.file_stem().is_some_and(|s| s.eq_ignore_ascii_case("readme"))
                })
            {
                win.open_path(&readme, None, true);
            }
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

    /// For Quick Open: the folder's documents, then recents outside the folder.
    fn palette_files(&self) -> (dialogs::NamedPaths, dialogs::NamedPaths) {
        let folder_files = self.0.sidebar.folder_files();
        let mut seen: std::collections::HashSet<PathBuf> = folder_files.iter().cloned().collect();
        let files = folder_files
            .iter()
            .map(|p| (p.to_string_lossy().into_owned(), files::display_name(p)))
            .collect();
        let mut recents = Vec::new();
        for recent in self.0.prefs.borrow().recents.iter() {
            let p = PathBuf::from(recent);
            if seen.insert(p.clone()) {
                recents.push((recent.clone(), files::display_name(&p)));
            }
        }
        (files, recents)
    }

    // ── Window chrome ────────────────────────────────────────────────────────

    /// The card sits flush against an open sidebar, and the tabs line up with the card.
    fn update_controls(&self) {
        let sidebar_visible = self.0.split.shows_sidebar() && !self.0.split.is_collapsed();
        if sidebar_visible {
            self.0.card.add_css_class("mdow-card-flush");
        } else {
            self.0.card.remove_css_class("mdow-card-flush");
        }
        if self.0.companion.is_shown() {
            self.0.card.add_css_class("mdow-card-flush-end");
        } else {
            self.0.card.remove_css_class("mdow-card-flush-end");
        }
        let spacer = if sidebar_visible {
            let scale = self.0.prefs.borrow().interface_scale;
            let (_, leading, _, _) =
                self.0.header_leading.measure(gtk::Orientation::Horizontal, -1);
            // The header's padding is 8px.
            (theme::sidebar_width(scale) as i32 - leading - 8).max(0)
        } else {
            0
        };
        self.0.header_spacer.set_size_request(spacer, -1);
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
            if win.0.companion.handle_shortcut(key, state) {
                return glib::Propagation::Stop;
            }
            let focus = gtk::prelude::RootExt::focus(&win.0.window);
            if win.0.companion.claim_typing(key, state, focus.as_ref()) {
                return glib::Propagation::Stop;
            }
            let typing = focus.as_ref().is_some_and(|f| {
                f.is::<gtk::Text>()
                    || f.is::<gtk::Editable>()
                    || f.downcast_ref::<gtk::TextView>().is_some_and(|view| view.is_editable())
            });
            if !typing
                && win.0.window.visible_dialog().is_none()
                && let Some(reader) = win.active_reader()
            {
                let inside = focus.as_ref().is_some_and(|f| reader.contains_focus(f));
                if reader.handle_key(key, state, inside) {
                    return glib::Propagation::Stop;
                }
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
        let weak = self.weak();
        self.0.window.connect_destroy(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.companion.close();
                WINDOWS.with_borrow_mut(|windows| windows.retain(|w| !Rc::ptr_eq(w, &inner)));
            }
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

    fn setup_actions(&self) {
        self.add_action("open-file", |w| w.open_file_dialog());
        self.add_action("open-folder", |w| w.open_folder_dialog());
        self.add_action("new-window", |w| {
            let Some(app) = w.0.window.application().and_downcast::<adw::Application>() else {
                return;
            };
            Win::new(&app, w.0.prefs.clone(), w.0.prefs_path.clone()).0.window.present();
        });
        self.add_toggle_action("toggle-companion", false, |w, on| w.set_companion_shown(on));
        self.add_action("close-window", |w| w.0.window.close());
        self.add_action("menu", |w| w.0.menu_button.popup());
        self.add_action("go-back", |w| w.go_in_history(true));
        self.add_action("go-forward", |w| w.go_in_history(false));
        self.add_action("previous-heading", |w| {
            if let Some(reader) = w.active_reader() {
                reader.jump_heading(-1);
            }
        });
        self.add_action("next-heading", |w| {
            if let Some(reader) = w.active_reader() {
                reader.jump_heading(1);
            }
        });
        self.add_action("reload", |w| {
            if let Some(tab) = w.active_tab() {
                tab.content_hash.set(0);
                w.load_tab(&tab, true);
            }
        });
        self.add_action("open-in-editor", |w| w.open_in_editor());
        self.add_action("show-document", |w| {
            if let Some(path) = w.active_path() {
                w.show_in_folder(&path);
            }
        });
        self.add_action("open-terminal", |w| w.open_terminal());
        self.add_action("copy-document-path", |w| {
            if let Some(path) = w.active_path() {
                w.copy_path(&path.to_string_lossy());
            }
        });
        self.add_action("share", |w| w.share());
        self.add_action("print", |w| {
            if let Some(reader) = w.active_reader()
                && let Err(err) = print::print(&w.0.window, &reader)
            {
                w.toast(&format!("Could not print: {err}"));
            }
        });
        self.add_action("export-pdf", |w| w.export_pdf_dialog());
        self.add_string_action("export-pdf-to", |w, path| {
            if let Some(reader) = w.active_reader()
                && let Err(err) = print::export_pdf(&w.0.window, &reader, Path::new(path))
            {
                w.toast(&format!("Could not export the PDF: {err}"));
            }
        });
        self.add_action("appearance", |w| {
            if w.omarchy_theme().is_some() && omarchy::open_theme_switcher() {
                return;
            }
            let dark = adw::StyleManager::default().is_dark();
            let next = if dark { ThemeMode::Light } else { ThemeMode::Dark };
            w.update_prefs(|p| p.theme = next);
            if let Some(action) =
                w.0.window.lookup_action("theme").and_downcast::<gio::SimpleAction>()
            {
                action.set_state(&next.id().to_variant());
            }
            Win::restyle_all();
        });
        self.add_action("none", |_| {}).set_enabled(false);
        self.add_string_action("open-path", |w, p| w.open_path(Path::new(p), None, true));
        self.add_string_action("navigate", |w, p| w.navigate(Path::new(p), None));
        self.add_action("copy-code", |w| match w.active_reader().and_then(|r| r.code_in_view()) {
            Some(code) => {
                w.0.window.clipboard().set_text(code.trim_end_matches('\n'));
                w.toast("Code copied");
            }
            None => w.toast("No code block in view"),
        });
        self.add_string_action("follow-link", |w, url| {
            if let Some(reader) = w.active_reader() {
                reader.activate_link(url);
            }
        });
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
        self.add_action("palette", |w| {
            let reader = w.active_reader();
            let headings = reader.as_ref().map(|r| r.headings()).unwrap_or_default();
            let links = reader.and_then(|r| r.with_document(doc::links)).unwrap_or_default();
            let (files, recents) = w.palette_files();
            dialogs::present_palette(&w.0.window, headings, links, files, recents);
        });
        self.add_action("zoom-in", |w| w.zoom(1));
        self.add_action("zoom-out", |w| w.zoom(-1));
        self.add_action("zoom-reset", |w| w.zoom(0));
        let wide = self.0.prefs.borrow().wide_mode;
        self.add_toggle_action("wide-mode", wide, |w, on| {
            w.update_prefs(|p| p.wide_mode = on);
            Win::restyle_all();
            w.refresh_active();
        });
        self.add_toggle_action("split-view", false, |w, on| w.set_split(on));
        self.add_action("close-split", |w| w.set_split(false));

        let prefs = self.0.prefs.borrow().clone();
        self.add_choice_action("theme", prefs.theme.id(), |w, id| {
            if let Some(v) = ThemeMode::from_id(id) {
                w.update_prefs(|p| p.theme = v);
                Win::restyle_all();
            }
        });
        self.add_choice_action("content-font", prefs.content_font.id(), |w, id| {
            if let Some(v) = ContentFont::from_id(id) {
                w.update_prefs(|p| p.content_font = v);
                Win::restyle_all();
            }
        });
        self.add_choice_action("code-font", prefs.code_font.id(), |w, id| {
            if let Some(v) = CodeFont::from_id(id) {
                w.update_prefs(|p| p.code_font = v);
                Win::restyle_all();
            }
        });
        self.add_choice_action("interface-scale", prefs.interface_scale.id(), |w, id| {
            if let Some(v) = InterfaceScale::from_id(id) {
                w.update_prefs(|p| p.interface_scale = v);
                Win::restyle_all();
            }
        });
        self.add_choice_action("reading-width", prefs.reading_width.id(), |w, id| {
            if let Some(v) = ReadingWidth::from_id(id) {
                w.update_prefs(|p| p.reading_width = v);
                Win::restyle_all();
            }
        });
        self.add_choice_action("line-spacing", prefs.line_spacing.id(), |w, id| {
            if let Some(v) = LineSpacing::from_id(id) {
                w.update_prefs(|p| p.line_spacing = v);
                Win::restyle_all();
            }
        });
        self.add_toggle_action("live-reload", prefs.live_reload, |w, on| {
            w.update_prefs(|p| p.live_reload = on);
        });
        self.add_toggle_action("open-in-tabs", prefs.open_in_tabs, |w, on| {
            w.update_prefs(|p| p.open_in_tabs = on);
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
                ("line-spacing", prefs.line_spacing.id()),
            ] {
                if let Some(action) =
                    w.0.window.lookup_action(name).and_downcast::<gio::SimpleAction>()
                {
                    action.set_state(&value.to_variant());
                }
            }
            Win::restyle_all();
        });
        self.add_action("settings", |w| {
            let prefs = w.0.prefs.borrow().clone();
            dialogs::present_settings(&w.0.window, &prefs, on_omarchy());
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
                let path = tab.path();
                w.set_secondary(Some(&path), None);
            }
        });
        self.add_action("tab-copy-path", |w| {
            if let Some(tab) = w.menu_tab() {
                w.copy_path(&tab.path().to_string_lossy());
            }
        });
        self.add_action("tab-show-in-folder", |w| {
            if let Some(tab) = w.menu_tab() {
                w.show_in_folder(&tab.path());
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

/// The ⋯ menu. There's no menu bar on Hyprland, so this holds what Mac has in File and View.
fn primary_menu(recent_menu: &gio::Menu) -> gio::Menu {
    fn section(items: &[(&str, &str)]) -> gio::Menu {
        let menu = gio::Menu::new();
        for (label, action) in items {
            menu.append(Some(label), Some(action));
        }
        menu
    }
    let menu = gio::Menu::new();
    let files = section(&[
        ("New Window", "win.new-window"),
        ("Open File…", "win.open-file"),
        ("Open Folder…", "win.open-folder"),
    ]);
    files.append_submenu(Some("Open Recent"), recent_menu);
    menu.append_section(None, &files);
    let document = section(&[
        ("Open in Editor", "win.open-in-editor"),
        ("Show in Files", "win.show-document"),
        ("Open Terminal Here", "win.open-terminal"),
        ("Copy Path", "win.copy-document-path"),
    ]);
    if on_omarchy() {
        document.append(Some("Share…"), Some("win.share"));
    }
    menu.append_section(None, &document);
    let zoom_section = gio::Menu::new();
    let zoom_item = gio::MenuItem::new(None, None);
    zoom_item.set_attribute_value("custom", Some(&"zoom".to_variant()));
    zoom_section.append_item(&zoom_item);
    menu.append_section(None, &zoom_section);
    let view = section(&[
        ("Show Sidebar", "win.toggle-sidebar"),
        ("Show Companion", "win.toggle-companion"),
        ("Split View", "win.split-view"),
        ("Full Width", "win.wide-mode"),
        ("Reload", "win.reload"),
    ]);
    menu.append_section(None, &view);
    menu.append_section(
        None,
        &section(&[("Export as PDF…", "win.export-pdf"), ("Print…", "win.print")]),
    );
    menu.append_section(
        None,
        &section(&[
            ("Settings", "win.settings"),
            ("Keyboard Shortcuts", "win.shortcuts"),
            ("About Mdow", "win.about"),
        ]),
    );
    menu.append_section(None, &section(&[("Close Tab", "win.close-tab"), ("Quit", "app.quit")]));
    menu
}

pub fn set_accels(app: &adw::Application) {
    let accels: &[(&str, &[&str])] = &[
        ("win.new-window", &["<Control>n"]),
        ("win.menu", &["F10"]),
        ("win.palette", &["<Control>k", "<Control>t", "<Control><Shift>p"]),
        ("win.open-file", &["<Control>o"]),
        ("win.open-folder", &["<Control><Shift>o"]),
        ("win.close-tab", &["<Control>w"]),
        ("win.close-window", &["<Control><Shift>w"]),
        ("win.reload", &["<Control>r", "F5"]),
        ("win.show-document", &["<Control><Alt>r"]),
        ("win.open-in-editor", &["<Control><Shift>e"]),
        ("win.copy-document-path", &["<Control><Alt>c"]),
        ("win.print", &["<Control>p"]),
        ("win.find", &["<Control>f"]),
        ("win.find-next", &["<Control>g", "F3"]),
        ("win.find-prev", &["<Control><Shift>g", "<Shift>F3"]),
        ("win.go-back", &["<Control>bracketleft", "<Alt>Left"]),
        ("win.go-forward", &["<Control>bracketright", "<Alt>Right"]),
        ("win.previous-heading", &["<Control><Alt>Up"]),
        ("win.next-heading", &["<Control><Alt>Down"]),
        ("win.toggle-sidebar", &["<Control>b"]),
        ("win.toggle-companion", &["<Control><Alt>0"]),
        ("win.sidebar-mode::outline", &["<Control><Alt>1"]),
        ("win.sidebar-mode::folder", &["<Control><Alt>2"]),
        ("win.sidebar-mode::recents", &["<Control><Alt>3"]),
        ("win.zoom-in", &["<Control>equal", "<Control>plus", "<Control>KP_Add"]),
        ("win.zoom-out", &["<Control>minus", "<Control>KP_Subtract"]),
        ("win.zoom-reset", &["<Control>0", "<Control>KP_0"]),
        ("win.shortcuts", &["<Control>slash", "<Control>question"]),
        ("win.settings", &["<Control>comma"]),
        ("win.next-tab", &["<Control>Tab", "<Control><Shift>bracketright", "<Control><Alt>Right"]),
        (
            "win.prev-tab",
            &[
                "<Control><Shift>Tab",
                "<Control><Shift>ISO_Left_Tab",
                "<Control><Shift>bracketleft",
                "<Control><Alt>Left",
            ],
        ),
        ("win.wide-mode", &["<Control><Shift>l"]),
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
