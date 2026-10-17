//! The empty-window screen: logo, Open File / Open Folder, the drop hint and recent files —
//! the desktop's `WelcomeView.tsx`.

use crate::files;
use gtk::prelude::*;
use std::path::Path;

pub struct Welcome {
    pub root: gtk::Box,
    grid: gtk::Box,
    intro: gtk::Box,
    recents_column: gtk::Box,
    recents_list: gtk::Box,
    logo: gtk::Image,
}

fn outline_button(icon: &str, label: &str, action: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("mdow-outline");
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(14);
    content.append(&image);
    content.append(&gtk::Label::new(Some(label)));
    button.set_child(Some(&content));
    button.set_action_name(Some(action));
    button
}

impl Welcome {
    pub fn new() -> Welcome {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("mdow-welcome");
        root.set_vexpand(true);
        root.set_hexpand(true);

        let grid = gtk::Box::new(gtk::Orientation::Horizontal, 40);
        grid.set_valign(gtk::Align::Center);
        grid.set_halign(gtk::Align::Center);
        grid.set_vexpand(true);
        grid.set_margin_start(40);
        grid.set_margin_end(40);
        grid.set_homogeneous(true);
        grid.set_size_request(-1, -1);

        let intro = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let logo = gtk::Image::new();
        logo.add_css_class("mdow-logo");
        logo.set_pixel_size(48);
        logo.set_halign(gtk::Align::Start);
        logo.set_overflow(gtk::Overflow::Hidden);
        intro.append(&logo);
        let title = gtk::Label::new(Some("Mdow"));
        title.add_css_class("mdow-welcome-title");
        title.set_xalign(0.0);
        intro.append(&title);
        let text = gtk::Label::new(Some(
            "A quiet markdown viewer. Drop a file anywhere, or open one below.",
        ));
        text.add_css_class("mdow-welcome-text");
        text.set_wrap(true);
        text.set_max_width_chars(40);
        text.set_xalign(0.0);
        intro.append(&text);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        buttons.set_margin_top(8);
        buttons.append(&outline_button("document-open-symbolic", "Open File", "win.open-file"));
        buttons.append(&outline_button("folder-open-symbolic", "Open Folder", "win.open-folder"));
        intro.append(&buttons);
        let card = gtk::Label::new(None);
        card.set_markup(
            "<span weight=\"500\">Anywhere in this window</span>, drop <span font_family=\"Geist Mono\" size=\"92%\">.md</span> or <span font_family=\"Geist Mono\" size=\"92%\">.html</span> files or a folder.",
        );
        card.set_wrap(true);
        card.set_xalign(0.0);
        let card_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        card_box.add_css_class("mdow-dropcard");
        card_box.set_margin_top(12);
        card_box.append(&card);
        intro.append(&card_box);
        grid.append(&intro);

        let recents_column = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let label = gtk::Label::new(Some("RECENT"));
        label.add_css_class("mdow-welcome-recent-label");
        label.set_xalign(0.0);
        recents_column.append(&label);
        let recents_list = gtk::Box::new(gtk::Orientation::Vertical, 1);
        recents_column.append(&recents_list);
        grid.append(&recents_column);

        root.append(&grid);
        Welcome { root, grid, intro, recents_column, recents_list, logo }
    }

    pub fn set_logo(&self, paintable: Option<&gtk::gdk::Paintable>) {
        self.logo.set_paintable(paintable);
        self.logo.set_pixel_size(48);
    }

    pub fn set_recents(&self, recents: &[String]) {
        while let Some(child) = self.recents_list.first_child() {
            self.recents_list.remove(&child);
        }
        for path in recents.iter().take(6) {
            let button = gtk::Button::new();
            button.add_css_class("flat");
            button.add_css_class("mdow-recent");
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let image = gtk::Image::from_icon_name("text-x-generic-symbolic");
            image.set_pixel_size(14);
            content.append(&image);
            let label = gtk::Label::new(Some(&files::display_name(Path::new(path))));
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_xalign(0.0);
            content.append(&label);
            button.set_child(Some(&content));
            button.set_tooltip_text(Some(path));
            button.set_action_name(Some("win.open-path"));
            button.set_action_target_value(Some(&path.to_variant()));
            self.recents_list.append(&button);
        }
        let has = !recents.is_empty();
        self.recents_column.set_visible(has);
        // Without recents the intro centres itself, as on desktop.
        self.grid.set_homogeneous(has);
        let align = if has { gtk::Align::Fill } else { gtk::Align::Center };
        self.intro.set_halign(align);
        let mut child = self.intro.first_child();
        while let Some(widget) = child {
            widget.set_halign(if has { gtk::Align::Start } else { gtk::Align::Center });
            if let Some(label) = widget.downcast_ref::<gtk::Label>() {
                label.set_xalign(if has { 0.0 } else { 0.5 });
                label.set_justify(if has {
                    gtk::Justification::Left
                } else {
                    gtk::Justification::Center
                });
            }
            child = widget.next_sibling();
        }
    }

    pub fn set_drag_over(&self, over: bool) {
        if over {
            self.root.add_css_class("mdow-drag-over");
        } else {
            self.root.remove_css_class("mdow-drag-over");
        }
    }
}
