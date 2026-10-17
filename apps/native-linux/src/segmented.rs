//! A segmented control: grouped toggle buttons, one active at a time. It stands in for
//! `AdwToggleGroup` (libadwaita 1.7) so the app also runs on libadwaita 1.5.

use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type Callback = Rc<dyn Fn(&str)>;

struct Inner {
    root: gtk::Box,
    buttons: RefCell<Vec<(String, gtk::ToggleButton)>>,
    callbacks: RefCell<Vec<Callback>>,
}

#[derive(Clone)]
pub struct Segmented(Rc<Inner>);

impl Segmented {
    pub fn new(css_class: &str) -> Segmented {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        root.add_css_class(css_class);
        root.set_homogeneous(true);
        Segmented(Rc::new(Inner {
            root,
            buttons: RefCell::new(Vec::new()),
            callbacks: RefCell::new(Vec::new()),
        }))
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.0.root
    }

    pub fn add(&self, name: &str, child: &impl IsA<gtk::Widget>, tooltip: Option<&str>) {
        let button = gtk::ToggleButton::new();
        button.set_child(Some(child));
        button.set_tooltip_text(tooltip);
        if let Some((_, first)) = self.0.buttons.borrow().first() {
            button.set_group(Some(first));
        }
        let weak = Rc::downgrade(&self.0);
        let id = name.to_string();
        button.connect_toggled(move |button| {
            if !button.is_active() {
                return;
            }
            let Some(inner) = weak.upgrade() else { return };
            let callbacks: Vec<Callback> = inner.callbacks.borrow().clone();
            for cb in callbacks {
                cb(&id);
            }
        });
        self.0.root.append(&button);
        self.0.buttons.borrow_mut().push((name.to_string(), button));
    }

    pub fn add_label(&self, name: &str, label: &str) {
        self.add(name, &gtk::Label::new(Some(label)), None);
    }

    /// An icon and a label side by side.
    pub fn add_icon_label(&self, name: &str, icon: &str, label: &str) {
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        content.set_halign(gtk::Align::Center);
        let image = gtk::Image::from_icon_name(icon);
        image.set_pixel_size(14);
        content.append(&image);
        content.append(&gtk::Label::new(Some(label)));
        self.add(name, &content, Some(label));
    }

    pub fn set_active_name(&self, name: &str) {
        let button =
            self.0.buttons.borrow().iter().find(|(n, _)| n == name).map(|(_, b)| b.clone());
        if let Some(button) = button
            && !button.is_active()
        {
            button.set_active(true);
        }
    }

    pub fn connect_changed(&self, f: impl Fn(&str) + 'static) {
        self.0.callbacks.borrow_mut().push(Rc::new(f));
    }
}
