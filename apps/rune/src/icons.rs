//! The header's and sidebar's icons: thin line drawings on a 20 point grid, close to the SF
//! Symbols the Swift app uses.

use rune::Icon;

macro_rules! icon {
    ($id:ident, $name:literal, $body:literal) => {
        pub static $id: Icon = Icon { name: $name, view: 20.0, body: $body };
    };
}

icon!(
    SIDEBAR,
    "mdow-sidebar",
    r#"<rect x="2.5" y="3.5" width="15" height="13" rx="2.5" stroke="currentColor" stroke-width="1.3"/><path d="M7.5 3.5v13" stroke="currentColor" stroke-width="1.3"/>"#
);
icon!(
    BACK,
    "mdow-back",
    r#"<path d="M12 4.5L6.5 10l5.5 5.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    FORWARD,
    "mdow-forward",
    r#"<path d="M8 4.5l5.5 5.5L8 15.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    SEARCH,
    "mdow-search",
    r#"<circle cx="8.75" cy="8.75" r="5.25" stroke="currentColor" stroke-width="1.4"/><path d="M12.6 12.6l4 4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#
);
icon!(
    SPARKLE,
    "mdow-sparkle",
    r#"<path d="M10 2.5c.5 3.6 1.9 5 5.5 5.5-3.6.5-5 1.9-5.5 5.5-.5-3.6-1.9-5-5.5-5.5 3.6-.5 5-1.9 5.5-5.5z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M15.5 13.5c.2 1.3.7 1.8 2 2-1.3.2-1.8.7-2 2-.2-1.3-.7-1.8-2-2 1.3-.2 1.8-.7 2-2z" fill="currentColor"/>"#
);
icon!(
    MORE,
    "mdow-more",
    r#"<circle cx="5" cy="10" r="1.3" fill="currentColor"/><circle cx="10" cy="10" r="1.3" fill="currentColor"/><circle cx="15" cy="10" r="1.3" fill="currentColor"/>"#
);
icon!(
    PLUS,
    "mdow-plus",
    r#"<path d="M10 4.5v11M4.5 10h11" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#
);
icon!(
    CLOSE,
    "mdow-close",
    r#"<path d="M6 6l8 8M14 6l-8 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#
);
icon!(
    CHEVRON_RIGHT,
    "mdow-chevron-right",
    r#"<path d="M8 5.5l4.5 4.5L8 14.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    CHEVRON_DOWN,
    "mdow-chevron-down",
    r#"<path d="M5.5 8l4.5 4.5L14.5 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    CHEVRON_UP,
    "mdow-chevron-up",
    r#"<path d="M5.5 12l4.5-4.5 4.5 4.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    FOLDER,
    "mdow-folder",
    r#"<path d="M2.5 5.5a1.5 1.5 0 011.5-1.5h3.4l1.6 1.8h7a1.5 1.5 0 011.5 1.5v7.7a1.5 1.5 0 01-1.5 1.5H4a1.5 1.5 0 01-1.5-1.5z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>"#
);
icon!(
    DOC,
    "mdow-doc",
    r#"<path d="M5.5 2.5h6l3.5 3.5v10a1.5 1.5 0 01-1.5 1.5h-8A1.5 1.5 0 014 16V4a1.5 1.5 0 011.5-1.5z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M11.5 2.5V6H15M7 10h6M7 13h4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#
);
icon!(
    CLOCK,
    "mdow-clock",
    r#"<circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.3"/><path d="M10 6v4.3l2.8 1.7" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    OUTLINE,
    "mdow-outline",
    r#"<path d="M4 5.5h12M7 10h9M10 14.5h6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><circle cx="4.2" cy="10" r="0.9" fill="currentColor"/><circle cx="7.2" cy="14.5" r="0.9" fill="currentColor"/>"#
);
icon!(
    SUN,
    "mdow-sun",
    r#"<circle cx="10" cy="10" r="3.2" stroke="currentColor" stroke-width="1.3"/><path d="M10 2.5v1.8M10 15.7v1.8M2.5 10h1.8M15.7 10h1.8M4.7 4.7l1.3 1.3M14 14l1.3 1.3M4.7 15.3L6 14M14 6l1.3-1.3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#
);
icon!(
    MOON,
    "mdow-moon",
    r#"<path d="M15.8 12.3A6.5 6.5 0 017.7 4.2a6.5 6.5 0 108.1 8.1z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>"#
);
icon!(
    GEAR,
    "mdow-gear",
    r#"<circle cx="10" cy="10" r="2.4" stroke="currentColor" stroke-width="1.3"/><path d="M10 2.8l1.2 1.9 2.2-.6.4 2.2 2.1.9-.9 2.1 1.4 1.7-1.9 1.2.4 2.2-2.2.2-.8 2.1-2-1-1.9 1-.8-2.1-2.2-.2.4-2.2L3.5 11l1.4-1.7L4 7.2l2.1-.9.4-2.2 2.2.6z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>"#
);
icon!(
    COPY,
    "mdow-copy",
    r#"<rect x="6.5" y="6.5" width="10" height="10" rx="2" stroke="currentColor" stroke-width="1.3"/><path d="M13.5 6.5V5a1.5 1.5 0 00-1.5-1.5H5A1.5 1.5 0 003.5 5v7A1.5 1.5 0 005 13.5h1.5" stroke="currentColor" stroke-width="1.3"/>"#
);
icon!(
    CHECK,
    "mdow-check",
    r#"<path d="M4.5 10.5l3.5 3.5 7.5-8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    COMMAND,
    "mdow-command",
    r#"<path d="M7 7h6v6H7zM7 7V5.5A1.5 1.5 0 105.5 7H7zm6 0h1.5A1.5 1.5 0 1013 5.5V7zm0 6v1.5a1.5 1.5 0 101.5-1.5H13zm-6 0H5.5A1.5 1.5 0 107 14.5V13z" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>"#
);
icon!(
    HASH,
    "mdow-hash",
    r#"<path d="M8 3.5l-1.5 13M13.5 3.5l-1.5 13M4 7.5h12.5M3.5 12.5H16" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#
);
icon!(
    LOGO_MARK,
    "mdow-logo",
    r#"<rect x="2" y="2" width="16" height="16" rx="4.5" fill="currentColor" opacity="0.14"/><path d="M5.5 13.5v-7l2.75 3.5L11 6.5v7M13.5 6.5v7M12 11.5l1.5 2 1.5-2" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    SEND,
    "mdow-send",
    r#"<path d="M10 16V4.5M5 9.5l5-5 5 5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"#
);
icon!(
    STOP,
    "mdow-stop",
    r#"<rect x="5.5" y="5.5" width="9" height="9" rx="1.5" fill="currentColor"/>"#
);
icon!(
    ALERT_NOTE,
    "mdow-alert-note",
    r#"<circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.5"/><path d="M10 9v4.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><circle cx="10" cy="6.4" r="1" fill="currentColor"/>"#
);
icon!(
    ALERT_TIP,
    "mdow-alert-tip",
    r#"<path d="M7.2 13.2c-1.5-1-2.4-2.6-2.4-4.4a5.2 5.2 0 0110.4 0c0 1.8-.9 3.4-2.4 4.4v1.3H7.2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M7.8 17h4.4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>"#
);
icon!(
    ALERT_IMPORTANT,
    "mdow-alert-important",
    r#"<path d="M3.5 5a1.5 1.5 0 011.5-1.5h10A1.5 1.5 0 0116.5 5v7a1.5 1.5 0 01-1.5 1.5H9l-3.5 3v-3H5A1.5 1.5 0 013.5 12z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M10 6.3v3.6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><circle cx="10" cy="11.8" r=".95" fill="currentColor"/>"#
);
icon!(
    ALERT_WARNING,
    "mdow-alert-warning",
    r#"<path d="M8.7 3.6a1.5 1.5 0 012.6 0l6 10.6a1.5 1.5 0 01-1.3 2.3H4a1.5 1.5 0 01-1.3-2.3z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M10 7.5v3.8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><circle cx="10" cy="13.6" r=".95" fill="currentColor"/>"#
);
icon!(
    ALERT_CAUTION,
    "mdow-alert-caution",
    r#"<path d="M7.1 2.8h5.8l4.3 4.3v5.8l-4.3 4.3H7.1l-4.3-4.3V7.1z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M10 6.4v4.2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><circle cx="10" cy="13.2" r=".95" fill="currentColor"/>"#
);
