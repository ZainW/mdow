//! What a window shows with no document: a greeting, the open actions, recent documents and the
//! drop hint, the same screen as Mdow for Mac.

use crate::files;
use gtk::{glib, prelude::*};
use std::path::Path;

pub struct Welcome {
    pub root: gtk::ScrolledWindow,
    recents_label: gtk::Label,
    recents_list: gtk::Box,
    logo: gtk::Image,
    greeting: gtk::Label,
}

fn action_card(icon: &str, title: &str, keys: &str, action: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("mdow-action-card");
    button.set_hexpand(true);
    button.set_action_name(Some(action));
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(16);
    image.set_hexpand(true);
    image.set_halign(gtk::Align::Start);
    top.append(&image);
    let keys = gtk::Label::new(Some(keys));
    keys.add_css_class("mdow-action-keys");
    top.append(&keys);
    content.append(&top);
    let title = gtk::Label::new(Some(title));
    title.set_xalign(0.0);
    content.append(&title);
    button.set_child(Some(&content));
    button
}

fn greeting_for_hour(hour: i32) -> &'static str {
    match hour {
        5..=11 => "Good morning.",
        12..=17 => "Good afternoon.",
        _ => "Good evening.",
    }
}

fn current_greeting() -> &'static str {
    let hour = glib::DateTime::now_local().map(|now| now.hour()).unwrap_or(12);
    greeting_for_hour(hour)
}

impl Welcome {
    pub fn new() -> Welcome {
        let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        column.set_halign(gtk::Align::Center);
        column.set_valign(gtk::Align::Center);
        column.set_size_request(520, -1);
        column.set_margin_start(40);
        column.set_margin_end(40);
        column.set_margin_top(64);
        column.set_margin_bottom(64);

        let logo = gtk::Image::new();
        logo.add_css_class("mdow-logo");
        logo.set_pixel_size(44);
        logo.set_halign(gtk::Align::Start);
        logo.set_overflow(gtk::Overflow::Hidden);
        column.append(&logo);

        let greeting = gtk::Label::new(Some(current_greeting()));
        greeting.add_css_class("mdow-welcome-title");
        greeting.set_xalign(0.0);
        greeting.set_margin_top(22);
        column.append(&greeting);
        let text = gtk::Label::new(Some("Open a Markdown file or a folder to start reading."));
        text.add_css_class("mdow-welcome-text");
        text.set_xalign(0.0);
        text.set_wrap(true);
        text.set_margin_top(6);
        column.append(&text);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        actions.set_homogeneous(true);
        actions.set_margin_top(28);
        actions.append(&action_card(
            "text-x-generic-symbolic",
            "Open file",
            "Ctrl O",
            "win.open-file",
        ));
        actions.append(&action_card(
            "folder-symbolic",
            "Open folder",
            "Ctrl Shift O",
            "win.open-folder",
        ));
        column.append(&actions);

        let recents_label = gtk::Label::new(Some("Recent"));
        recents_label.add_css_class("mdow-welcome-section");
        recents_label.set_xalign(0.0);
        recents_label.set_margin_top(36);
        recents_label.set_margin_bottom(8);
        recents_label.set_margin_start(2);
        column.append(&recents_label);
        let recents_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        recents_list.add_css_class("mdow-recent-list");
        column.append(&recents_list);

        let hint =
            gtk::Label::new(Some("Drop files or a folder anywhere in the window to open them."));
        hint.add_css_class("mdow-welcome-hint");
        hint.set_xalign(0.0);
        hint.set_wrap(true);
        hint.set_margin_top(24);
        column.append(&hint);

        let root = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&column)
            .vexpand(true)
            .hexpand(true)
            .build();
        root.add_css_class("mdow-welcome");
        Welcome { root, recents_label, recents_list, logo, greeting }
    }

    pub fn set_logo(&self, paintable: Option<&gtk::gdk::Paintable>) {
        self.logo.set_paintable(paintable);
        self.logo.set_pixel_size(44);
        self.greeting.set_text(current_greeting());
    }

    pub fn set_recents(&self, recents: &[String]) {
        while let Some(child) = self.recents_list.first_child() {
            self.recents_list.remove(&child);
        }
        for path in recents.iter().take(6) {
            let button = gtk::Button::new();
            button.add_css_class("flat");
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let image = gtk::Image::from_icon_name("text-x-generic-symbolic");
            image.set_pixel_size(14);
            content.append(&image);
            let name = gtk::Label::new(Some(&files::display_name(Path::new(path))));
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            name.set_xalign(0.0);
            content.append(&name);
            let dir = gtk::Label::new(Some(&files::display_dir(Path::new(path))));
            dir.add_css_class("mdow-recent-dir");
            dir.set_ellipsize(gtk::pango::EllipsizeMode::Start);
            dir.set_hexpand(true);
            dir.set_xalign(1.0);
            content.append(&dir);
            button.set_child(Some(&content));
            button.set_tooltip_text(Some(path));
            button.set_action_name(Some("win.open-path"));
            button.set_action_target_value(Some(&path.to_variant()));
            self.recents_list.append(&button);
        }
        let has = !recents.is_empty();
        self.recents_label.set_visible(has);
        self.recents_list.set_visible(has);
    }

    pub fn set_drag_over(&self, over: bool) {
        if over {
            self.root.add_css_class("mdow-drag-over");
        } else {
            self.root.remove_css_class("mdow-drag-over");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_by_the_time_of_day() {
        assert_eq!(greeting_for_hour(4), "Good evening.");
        assert_eq!(greeting_for_hour(5), "Good morning.");
        assert_eq!(greeting_for_hour(12), "Good afternoon.");
        assert_eq!(greeting_for_hour(18), "Good evening.");
    }
}
