//! The sidebar: Recents, Folder and Outline, switched by a segmented control, with Settings in
//! the footer — the desktop's `Sidebar.tsx`. Everything it triggers goes through window actions.

use crate::doc::Heading;
use crate::files::{self, Scan, TreeNode};
use crate::prefs::SidebarMode;
use crate::segmented::Segmented;
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub fn empty_state(icon: &str, title: &str, hint: &str) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
    b.add_css_class("mdow-empty");
    b.set_valign(gtk::Align::Start);
    let image = gtk::Image::from_icon_name(icon);
    b.append(&image);
    let t = gtk::Label::new(Some(title));
    t.add_css_class("mdow-empty-title");
    b.append(&t);
    if !hint.is_empty() {
        let h = gtk::Label::new(Some(hint));
        h.add_css_class("mdow-empty-hint");
        h.set_wrap(true);
        h.set_justify(gtk::Justification::Center);
        h.set_max_width_chars(24);
        b.append(&h);
    }
    b
}

fn group_label(text: &str) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    b.add_css_class("mdow-group-label");
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    l.set_hexpand(true);
    b.append(&l);
    b
}

/// Show a menu at a pointer position inside `widget`.
pub fn popup_menu(widget: &impl IsA<gtk::Widget>, x: f64, y: f64, menu: &gio::Menu) {
    let popover = gtk::PopoverMenu::from_model(Some(menu));
    popover.set_parent(widget);
    popover.set_has_arrow(false);
    popover.set_halign(gtk::Align::Start);
    popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    popover.connect_closed(|popover| {
        let popover = popover.clone();
        glib::idle_add_local_once(move || popover.unparent());
    });
    popover.popup();
}

fn menu_item(label: &str, action: &str, target: &str) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(label), None);
    item.set_action_and_target_value(Some(action), Some(&target.to_variant()));
    item
}

pub fn path_menu(path: &str, open_label: &str, with_remove: bool) -> gio::Menu {
    let menu = gio::Menu::new();
    let main = gio::Menu::new();
    main.append_item(&menu_item(open_label, "win.open-path", path));
    main.append_item(&menu_item("Copy Path", "win.copy-path", path));
    main.append_item(&menu_item("Show in Folder", "win.show-in-folder", path));
    menu.append_section(None, &main);
    if with_remove {
        let danger = gio::Menu::new();
        danger.append_item(&menu_item("Remove from Recents", "win.remove-recent", path));
        menu.append_section(None, &danger);
    }
    menu
}

fn file_row(name: &str, dir: Option<&str>, icon: &str) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 1);
    b.add_css_class("mdow-row");
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let image = gtk::Image::from_icon_name(icon);
    image.add_css_class("mdow-row-icon");
    image.set_pixel_size(14);
    top.append(&image);
    let label = gtk::Label::new(Some(name));
    label.set_xalign(0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_hexpand(true);
    top.append(&label);
    b.append(&top);
    if let Some(dir) = dir.filter(|d| !d.is_empty()) {
        let d = gtk::Label::new(Some(dir));
        d.add_css_class("mdow-row-dir");
        d.set_xalign(0.0);
        d.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        d.set_margin_start(22);
        b.append(&d);
    }
    b
}

struct Folder {
    root: RefCell<Option<PathBuf>>,
    scan: RefCell<Scan>,
    filter: gtk::SearchEntry,
    count: gtk::Label,
    note: gtk::Label,
    name: gtk::Label,
    list: gtk::ListView,
    selection: gtk::SingleSelection,
    stack: gtk::Stack,
    tree_stack: gtk::Stack,
    opening: gtk::Revealer,
    active: RefCell<Option<PathBuf>>,
}

struct Outline {
    store: gio::ListStore,
    list: gtk::ListView,
    stack: gtk::Stack,
    empty_title: gtk::Label,
    empty_hint: gtk::Label,
    active: Cell<Option<usize>>,
    rows: RefCell<HashMap<usize, gtk::Widget>>,
}

pub struct Sidebar {
    pub root: gtk::Box,
    modes: Segmented,
    stack: gtk::Stack,
    recents_box: gtk::ListBox,
    recents_stack: gtk::Stack,
    recents: RefCell<Vec<String>>,
    folder: Rc<Folder>,
    outline: Rc<Outline>,
    updating_mode: Cell<bool>,
}

impl Sidebar {
    pub fn new() -> Rc<Sidebar> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("mdow-sidebar");

        // Mode switcher, inside a header that also drags the window.
        let modes = Segmented::new("mdow-mode-tabs");
        modes.widget().set_hexpand(true);
        for mode in SidebarMode::ALL {
            let icon = match mode {
                SidebarMode::Recents => "document-open-recent-symbolic",
                SidebarMode::Folder => "folder-symbolic",
                SidebarMode::Outline => "view-list-bullet-symbolic",
            };
            modes.add_icon_label(mode.id(), icon, mode.label());
        }
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        header.add_css_class("mdow-sidebar-header");
        let start_controls = gtk::WindowControls::new(gtk::PackType::Start);
        header.append(&start_controls);
        header.append(modes.widget());
        let handle = gtk::WindowHandle::new();
        handle.set_child(Some(&header));
        root.append(&handle);

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        stack.set_transition_duration(120);
        root.append(&stack);

        // Recents.
        let recents_box = gtk::ListBox::new();
        recents_box.set_selection_mode(gtk::SelectionMode::None);
        recents_box.set_activate_on_single_click(true);
        recents_box.connect_row_activated(|list, row| {
            let path = row.widget_name();
            let _ = list.activate_action("win.open-path", Some(&path.to_variant()));
        });
        let recents_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        recents_content.append(&group_label("Recents"));
        recents_content.append(&recents_box);
        let recents_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&recents_content)
            .vexpand(true)
            .build();
        let recents_stack = gtk::Stack::new();
        recents_stack.add_named(&recents_scroll, Some("list"));
        recents_stack.add_named(
            &empty_state(
                "document-open-recent-symbolic",
                "No recents yet",
                "Files you open will appear here.",
            ),
            Some("empty"),
        );
        stack.add_named(&recents_stack, Some(SidebarMode::Recents.id()));

        let folder = Rc::new(Self::build_folder());
        stack.add_named(&folder.stack, Some(SidebarMode::Folder.id()));

        let outline = Rc::new(Self::build_outline());
        stack.add_named(&outline.stack, Some(SidebarMode::Outline.id()));

        // Footer.
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        footer.add_css_class("mdow-sidebar-footer");
        let settings = gtk::Button::new();
        settings.add_css_class("flat");
        settings.set_hexpand(true);
        settings.set_action_name(Some("win.settings"));
        settings.set_tooltip_text(Some("Settings"));
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let image = gtk::Image::from_icon_name("emblem-system-symbolic");
        image.set_pixel_size(14);
        content.append(&image);
        let label = gtk::Label::new(Some("Settings"));
        content.append(&label);
        settings.set_child(Some(&content));
        footer.append(&settings);
        root.append(&footer);

        let sidebar = Rc::new(Sidebar {
            root,
            modes,
            stack,
            recents_box,
            recents_stack,
            recents: RefCell::new(Vec::new()),
            folder,
            outline,
            updating_mode: Cell::new(false),
        });

        let weak = Rc::downgrade(&sidebar);
        sidebar.modes.connect_changed(move |name| {
            let Some(sidebar) = weak.upgrade() else { return };
            if sidebar.updating_mode.get() {
                return;
            }
            let _ = sidebar.root.activate_action("win.sidebar-mode", Some(&name.to_variant()));
        });
        sidebar
    }

    fn build_folder() -> Folder {
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let label = group_label("Folder");
        let name = gtk::Label::new(None);
        name.add_css_class("mdow-folder-name");
        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        label.append(&name);
        content.append(&label);

        let filter = gtk::SearchEntry::new();
        filter.set_placeholder_text(Some("Filter folder..."));
        filter.add_css_class("mdow-filter");
        content.append(&filter);
        let count = gtk::Label::new(None);
        count.add_css_class("mdow-match-count");
        count.set_xalign(0.0);
        count.set_visible(false);
        content.append(&count);
        let note = gtk::Label::new(Some("Large folder: some files hidden"));
        note.add_css_class("mdow-note");
        note.set_xalign(0.0);
        note.set_wrap(true);
        note.set_tooltip_text(Some(
            "Some files were hidden to keep the app responsive. Open a smaller subfolder to see them.",
        ));
        note.set_visible(false);
        content.append(&note);

        let opening = gtk::Revealer::new();
        let opening_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        opening_box.add_css_class("mdow-match-count");
        let spinner = gtk::Spinner::new();
        spinner.start();
        spinner.set_size_request(12, 12);
        opening_box.append(&spinner);
        opening_box.append(&gtk::Label::new(Some("Opening…")));
        opening.set_child(Some(&opening_box));
        content.append(&opening);

        let selection = gtk::SingleSelection::new(None::<gio::ListModel>);
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let expander = gtk::TreeExpander::new();
            expander.set_indent_for_icon(true);
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            b.add_css_class("mdow-row");
            let image = gtk::Image::new();
            image.add_css_class("mdow-row-icon");
            image.set_pixel_size(14);
            let label = gtk::Label::new(None);
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_hexpand(true);
            b.append(&image);
            b.append(&label);
            expander.set_child(Some(&b));
            item.set_child(Some(&expander));

            let click = gtk::GestureClick::new();
            click.set_button(gdk::BUTTON_SECONDARY);
            let weak_item = item.downgrade();
            let weak_expander = expander.downgrade();
            click.connect_pressed(move |gesture, _, x, y| {
                let (Some(list_item), Some(expander)) =
                    (weak_item.upgrade(), weak_expander.upgrade())
                else {
                    return;
                };
                let Some(node) = tree_node(&list_item) else { return };
                gesture.set_state(gtk::EventSequenceState::Claimed);
                let path = node.path.to_string_lossy().into_owned();
                let menu = if node.is_dir() {
                    let menu = gio::Menu::new();
                    let s = gio::Menu::new();
                    s.append_item(&menu_item("Copy Path", "win.copy-path", &path));
                    s.append_item(&menu_item("Show in Folder", "win.show-in-folder", &path));
                    menu.append_section(None, &s);
                    menu
                } else {
                    path_menu(&path, "Open", false)
                };
                popup_menu(&expander, x, y, &menu);
            });
            expander.add_controller(click);
        });
        factory.connect_bind(|_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let Some(row) = item.item().and_downcast::<gtk::TreeListRow>() else { return };
            let Some(expander) = item.child().and_downcast::<gtk::TreeExpander>() else { return };
            expander.set_list_row(Some(&row));
            let Some(node) = tree_node(item) else { return };
            let Some(b) = expander.child().and_downcast::<gtk::Box>() else { return };
            let image = b.first_child().and_downcast::<gtk::Image>();
            let label = b.last_child().and_downcast::<gtk::Label>();
            if let Some(image) = image {
                image.set_icon_name(Some(if node.is_dir() {
                    "folder-symbolic"
                } else {
                    "text-x-generic-symbolic"
                }));
            }
            if let Some(label) = label {
                label.set_text(&node.name);
            }
            expander.set_tooltip_text(Some(&node.path.to_string_lossy()));
        });
        let list = gtk::ListView::new(Some(selection.clone()), Some(factory));
        list.add_css_class("mdow-tree");
        list.set_single_click_activate(true);
        list.connect_activate(|list, position| {
            let Some(model) = list.model() else { return };
            let Some(row) = model.item(position).and_downcast::<gtk::TreeListRow>() else { return };
            let Some(node) = row_node(&row) else { return };
            if node.is_dir() {
                row.set_expanded(!row.is_expanded());
            } else {
                let path = node.path.to_string_lossy().into_owned();
                let _ = list.activate_action("win.open-path", Some(&path.to_variant()));
            }
        });
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .vexpand(true)
            .build();
        let tree_stack = gtk::Stack::new();
        tree_stack.set_vexpand(true);
        tree_stack.add_named(&scroll, Some("tree"));
        let no_match = gtk::Label::new(Some("No matching files"));
        no_match.add_css_class("mdow-empty-hint");
        no_match.set_valign(gtk::Align::Start);
        no_match.set_margin_top(24);
        tree_stack.add_named(&no_match, Some("none"));
        content.append(&tree_stack);

        let stack = gtk::Stack::new();
        stack.add_named(&content, Some("folder"));
        stack.add_named(
            &empty_state(
                "folder-open-symbolic",
                "No folder open",
                "Use the menu, keyboard shortcut, or drag a folder onto this window. Right-click a file to show it in its folder.",
            ),
            Some("empty"),
        );
        stack.set_visible_child_name("empty");

        Folder {
            root: RefCell::new(None),
            scan: RefCell::new(Scan::default()),
            filter,
            count,
            note,
            name,
            list,
            selection,
            stack,
            tree_stack,
            opening,
            active: RefCell::new(None),
        }
    }

    fn build_outline() -> Outline {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let label = gtk::Label::new(None);
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            b.add_css_class("mdow-row");
            b.append(&label);
            item.set_child(Some(&b));
        });
        let list = gtk::ListView::new(
            Some(gtk::NoSelection::new(Some(store.clone()))),
            Some(factory.clone()),
        );
        list.add_css_class("mdow-outline");
        list.set_single_click_activate(true);
        list.connect_activate(|list, position| {
            let _ = list.activate_action("win.goto-heading", Some(&position.to_variant()));
        });
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .vexpand(true)
            .build();
        scroll.set_margin_top(4);
        let stack = gtk::Stack::new();
        stack.add_named(&scroll, Some("list"));
        let empty = empty_state(
            "view-list-bullet-symbolic",
            "No document open",
            "Open a document to see its outline.",
        );
        let empty_title = empty
            .first_child()
            .and_then(|w| w.next_sibling())
            .and_downcast::<gtk::Label>()
            .expect("title");
        let empty_hint = empty.last_child().and_downcast::<gtk::Label>().expect("hint");
        stack.add_named(&empty, Some("empty"));
        stack.set_visible_child_name("empty");
        Outline {
            store,
            list,
            stack,
            empty_title,
            empty_hint,
            active: Cell::new(None),
            rows: RefCell::new(HashMap::new()),
        }
    }

    /// Wire the outline rows, which need the shared active index.
    pub fn connect(self: &Rc<Self>) {
        let factory = self
            .outline
            .list
            .factory()
            .and_downcast::<gtk::SignalListItemFactory>()
            .expect("factory");
        let weak = Rc::downgrade(&self.outline);
        factory.connect_bind(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let Some(outline) = weak.upgrade() else { return };
            let Some(obj) = item.item().and_downcast::<glib::BoxedAnyObject>() else { return };
            let heading = obj.borrow::<Heading>();
            let Some(row) = item.child() else { return };
            let label = row.first_child().and_downcast::<gtk::Label>();
            if let Some(label) = label {
                label.set_text(&heading.text);
            }
            row.set_margin_start(((heading.level.max(1) - 1) as i32) * 10);
            row.set_tooltip_text(Some(&heading.text));
            let position = item.position() as usize;
            if let Some(list_row) = row.parent() {
                if outline.active.get() == Some(position) {
                    list_row.add_css_class("mdow-active");
                } else {
                    list_row.remove_css_class("mdow-active");
                }
                outline.rows.borrow_mut().insert(position, list_row);
            }
        });
        let weak = Rc::downgrade(&self.outline);
        factory.connect_unbind(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else { return };
            let Some(row) = item.child().and_then(|c| c.parent()) else { return };
            if let Some(outline) = weak.upgrade() {
                let mut rows = outline.rows.borrow_mut();
                let position = item.position() as usize;
                if rows.get(&position) == Some(&row) {
                    rows.remove(&position);
                }
            }
            row.remove_css_class("mdow-active");
        });

        let weak = Rc::downgrade(self);
        self.folder.filter.connect_search_changed(move |_| {
            if let Some(sidebar) = weak.upgrade() {
                sidebar.rebuild_tree();
            }
        });
    }

    pub fn set_mode(&self, mode: SidebarMode) {
        self.updating_mode.set(true);
        self.modes.set_active_name(mode.id());
        self.updating_mode.set(false);
        self.stack.set_visible_child_name(mode.id());
    }

    pub fn set_recents(&self, recents: &[String], active: Option<&Path>) {
        *self.recents.borrow_mut() = recents.to_vec();
        while let Some(child) = self.recents_box.first_child() {
            self.recents_box.remove(&child);
        }
        for path in recents {
            let p = Path::new(path);
            let content = file_row(
                &files::display_name(p),
                Some(&files::display_dir(p)),
                "text-x-generic-symbolic",
            );
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&content));
            row.set_widget_name(path);
            row.set_tooltip_text(Some(path));
            let click = gtk::GestureClick::new();
            click.set_button(gdk::BUTTON_SECONDARY);
            let target = path.clone();
            let weak_row = row.downgrade();
            click.connect_pressed(move |gesture, _, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                if let Some(row) = weak_row.upgrade() {
                    popup_menu(&row, x, y, &path_menu(&target, "Open", true));
                }
            });
            row.add_controller(click);
            self.recents_box.append(&row);
        }
        self.recents_stack.set_visible_child_name(if recents.is_empty() {
            "empty"
        } else {
            "list"
        });
        self.set_recents_active(active);
    }

    pub fn set_recents_active(&self, active: Option<&Path>) {
        let mut child = self.recents_box.first_child();
        while let Some(widget) = child {
            let is_active = active.is_some_and(|a| a.as_os_str() == widget.widget_name().as_str());
            if is_active {
                widget.add_css_class("mdow-active");
            } else {
                widget.remove_css_class("mdow-active");
            }
            child = widget.next_sibling();
        }
    }

    pub fn set_folder(&self, folder: Option<(PathBuf, Scan)>) {
        let f = &self.folder;
        match folder {
            Some((root, scan)) => {
                f.name.set_text(&files::display_name(&root));
                f.name.set_tooltip_text(Some(&root.to_string_lossy()));
                f.note.set_visible(scan.truncated);
                let changed_root = f.root.borrow().as_deref() != Some(root.as_path());
                if changed_root {
                    f.filter.set_text("");
                }
                *f.root.borrow_mut() = Some(root);
                *f.scan.borrow_mut() = scan;
                f.stack.set_visible_child_name("folder");
            }
            None => {
                *f.root.borrow_mut() = None;
                *f.scan.borrow_mut() = Scan::default();
                f.stack.set_visible_child_name("empty");
            }
        }
        self.rebuild_tree();
    }

    pub fn folder_root(&self) -> Option<PathBuf> {
        self.folder.root.borrow().clone()
    }

    pub fn folder_files(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        files::flatten_files(&self.folder.scan.borrow().tree, &mut out);
        out
    }

    fn rebuild_tree(&self) {
        let f = &self.folder;
        let Some(root) = f.root.borrow().clone() else {
            f.selection.set_model(None::<&gio::ListModel>);
            return;
        };
        let query = f.filter.text().to_string();
        let filtering = !query.trim().is_empty();
        let nodes = files::filter_tree(&f.scan.borrow().tree, &root, &query);
        let n = files::count_files(&nodes);
        f.count.set_visible(filtering);
        f.count.set_text(&format!("{n} match{}", if n == 1 { "" } else { "es" }));
        let top = node_store(&nodes);
        let model = gtk::TreeListModel::new(top, false, filtering, |item| {
            let obj = item.downcast_ref::<glib::BoxedAnyObject>()?;
            let node = obj.borrow::<TreeNode>();
            node.children.as_ref().map(|children| node_store(children).upcast::<gio::ListModel>())
        });
        f.selection.set_model(Some(&model));
        f.tree_stack.set_visible_child_name(if nodes.is_empty() { "none" } else { "tree" });
        let active = f.active.borrow().clone();
        self.reveal_in_tree(active.as_deref());
    }

    /// Expand the folders above `path` and select it.
    pub fn reveal_in_tree(&self, path: Option<&Path>) {
        let f = &self.folder;
        *f.active.borrow_mut() = path.map(Path::to_path_buf);
        let Some(model) = f.selection.model().and_downcast::<gtk::TreeListModel>() else { return };
        let Some(path) = path else {
            f.selection.set_selected(gtk::INVALID_LIST_POSITION);
            return;
        };
        let Some(root) = f.root.borrow().clone() else { return };
        if !path.starts_with(&root) {
            f.selection.set_selected(gtk::INVALID_LIST_POSITION);
            return;
        }
        let mut i = 0;
        while i < model.n_items() {
            let Some(row) = model.item(i).and_downcast::<gtk::TreeListRow>() else { break };
            if let Some(node) = row_node(&row) {
                if node.path == path {
                    f.selection.set_selected(i);
                    f.list.scroll_to(i, gtk::ListScrollFlags::NONE, None);
                    return;
                }
                if node.is_dir() && path.starts_with(&node.path) && !row.is_expanded() {
                    row.set_expanded(true);
                }
            }
            i += 1;
        }
        f.selection.set_selected(gtk::INVALID_LIST_POSITION);
    }

    pub fn set_opening(&self, opening: bool) {
        self.folder.opening.set_reveal_child(opening);
    }

    pub fn set_outline(&self, headings: &[Heading], has_doc: bool, active: Option<usize>) {
        let o = &self.outline;
        o.rows.borrow_mut().clear();
        o.active.set(active);
        let items: Vec<glib::BoxedAnyObject> =
            headings.iter().cloned().map(glib::BoxedAnyObject::new).collect();
        o.store.splice(0, o.store.n_items(), &items);
        if headings.is_empty() {
            o.empty_title.set_text(if has_doc { "No headings" } else { "No document open" });
            o.empty_hint.set_text(if has_doc {
                "This document has no headings to show."
            } else {
                "Open a document to see its outline."
            });
            o.stack.set_visible_child_name("empty");
        } else {
            o.stack.set_visible_child_name("list");
        }
    }

    pub fn set_outline_active(&self, active: Option<usize>) {
        let o = &self.outline;
        let old = o.active.replace(active);
        if old == active {
            return;
        }
        {
            let rows = o.rows.borrow();
            if let Some(old) = old.and_then(|i| rows.get(&i)) {
                old.remove_css_class("mdow-active");
            }
            if let Some(row) = active.and_then(|i| rows.get(&i)) {
                row.add_css_class("mdow-active");
            }
        }
        // Scrolling binds and unbinds rows, which touches `rows`: no borrow may be held here.
        if let Some(i) = active
            && (i as u32) < o.store.n_items()
        {
            o.list.scroll_to(i as u32, gtk::ListScrollFlags::NONE, None);
        }
    }
}

fn node_store(nodes: &[TreeNode]) -> gio::ListStore {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    let items: Vec<glib::BoxedAnyObject> =
        nodes.iter().cloned().map(glib::BoxedAnyObject::new).collect();
    store.extend_from_slice(&items);
    store
}

fn row_node(row: &gtk::TreeListRow) -> Option<TreeNode> {
    let obj = row.item().and_downcast::<glib::BoxedAnyObject>()?;
    let node = obj.borrow::<TreeNode>().clone();
    Some(node)
}

fn tree_node(item: &gtk::ListItem) -> Option<TreeNode> {
    let row = item.item().and_downcast::<gtk::TreeListRow>()?;
    row_node(&row)
}
