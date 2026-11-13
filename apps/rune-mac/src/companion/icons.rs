//! The companion's own icons, drawn like `crate::icons`: thin lines on a 20 point grid, close
//! to the SF Symbols the Swift companion uses.

use rune::Icon;

macro_rules! icon {
    ($id:ident, $name:literal, $body:literal) => {
        pub static $id: Icon = Icon { name: $name, view: 20.0, body: $body };
    };
}

// square.and.pencil
icon!(
    NEW_CHAT,
    "mdow-co-new-chat",
    r#"<path d="M9.5 3.5H5a1.5 1.5 0 00-1.5 1.5v10A1.5 1.5 0 005 16.5h10a1.5 1.5 0 001.5-1.5v-4.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/><path d="M14.6 3.2a1.4 1.4 0 012 2L10.2 11.6l-2.7.7.7-2.7z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>"#
);
// at
icon!(
    AT,
    "mdow-co-at",
    r#"<circle cx="10" cy="10" r="3" stroke="currentColor" stroke-width="1.4"/><path d="M13 7v4.2a2 2 0 004 0V10a7 7 0 10-2.8 5.6" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#
);
// brain, simplified to a head with a spark of thought
icon!(
    THOUGHT,
    "mdow-co-thought",
    r#"<path d="M7.5 16.5v-2.2A5.8 5.8 0 014 9a6 6 0 0111.6-1.8l1 2.6-1.3.4V13a1.5 1.5 0 01-1.5 1.5h-1.3v2" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/><path d="M8 7.3a1.8 1.8 0 113 1.4c-.6.4-.9.8-.9 1.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>"#
);
// pencil.line
icon!(
    PENCIL,
    "mdow-co-pencil",
    r#"<path d="M12.8 3.8a1.6 1.6 0 012.3 2.3L7 14.2l-3 .8.8-3z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M10 16.5h6.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#
);
// doc.badge.plus
icon!(
    DOC_PLUS,
    "mdow-co-doc-plus",
    r#"<path d="M10 17.5H5.5A1.5 1.5 0 014 16V4a1.5 1.5 0 011.5-1.5h6L15 6v3.5" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M11.5 2.5V6H15M15 12v5M12.5 14.5h5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#
);
// exclamationmark.triangle
icon!(
    WARNING,
    "mdow-co-warning",
    r#"<path d="M8.7 3.8a1.5 1.5 0 012.6 0l6 10.4a1.5 1.5 0 01-1.3 2.3H4a1.5 1.5 0 01-1.3-2.3z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M10 8v3.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><circle cx="10" cy="14" r=".9" fill="currentColor"/>"#
);
// exclamationmark
icon!(
    BANG,
    "mdow-co-bang",
    r#"<path d="M10 4v8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><circle cx="10" cy="15.8" r="1.3" fill="currentColor"/>"#
);
// minus
icon!(
    MINUS,
    "mdow-co-minus",
    r#"<path d="M5 10h10" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>"#
);
// arrow.uturn.backward
icon!(
    UNDO,
    "mdow-co-undo",
    r#"<path d="M7.5 4.5L4 8l3.5 3.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/><path d="M4.5 8H12a4 4 0 010 8H8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>"#
);
// xmark, bolder than the header's close
icon!(
    XMARK,
    "mdow-co-xmark",
    r#"<path d="M5.5 5.5l9 9M14.5 5.5l-9 9" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>"#
);
// list.bullet
icon!(
    LIST,
    "mdow-co-list",
    r#"<circle cx="4.5" cy="5.5" r="1.1" fill="currentColor"/><circle cx="4.5" cy="10" r="1.1" fill="currentColor"/><circle cx="4.5" cy="14.5" r="1.1" fill="currentColor"/><path d="M8 5.5h8.5M8 10h8.5M8 14.5h8.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>"#
);
// lightbulb
icon!(
    BULB,
    "mdow-co-bulb",
    r#"<path d="M7.5 13.5c0-1.6-2.5-2.8-2.5-6a5 5 0 0110 0c0 3.2-2.5 4.4-2.5 6z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M7.8 16h4.4M8.8 18h2.4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#
);
// text.badge.checkmark
icon!(
    TEXT_CHECK,
    "mdow-co-text-check",
    r#"<path d="M3.5 5h11M3.5 9h8M3.5 13h5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><path d="M11 14.2l2 2 4-4.4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#
);
// scissors
icon!(
    SCISSORS,
    "mdow-co-scissors",
    r#"<circle cx="5.5" cy="14.5" r="2.3" stroke="currentColor" stroke-width="1.3"/><circle cx="14.5" cy="14.5" r="2.3" stroke="currentColor" stroke-width="1.3"/><path d="M7 12.8L14.5 3M13 12.8L5.5 3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>"#
);
// questionmark.text.page
icon!(
    QUESTION,
    "mdow-co-question",
    r#"<path d="M5.5 2.5h9A1.5 1.5 0 0116 4v12a1.5 1.5 0 01-1.5 1.5h-9A1.5 1.5 0 014 16V4a1.5 1.5 0 011.5-1.5z" stroke="currentColor" stroke-width="1.3"/><path d="M8.2 7.6a1.9 1.9 0 113 1.5c-.7.5-1.2.9-1.2 1.8" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/><circle cx="10" cy="13.6" r=".9" fill="currentColor"/>"#
);
// terminal: a command the agent ran (or was declined)
icon!(
    TERMINAL,
    "mdow-co-terminal",
    r#"<rect x="3" y="4" width="14" height="12" rx="2" stroke="currentColor" stroke-width="1.3"/><path d="M6.5 8l2.2 2-2.2 2M10.5 12.5h3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>"#
);
// globe: a page the agent fetched
icon!(
    GLOBE,
    "mdow-co-globe",
    r#"<circle cx="10" cy="10" r="6.5" stroke="currentColor" stroke-width="1.3"/><path d="M3.5 10h13M10 3.5c-2 2-2.8 4.2-2.8 6.5s.8 4.5 2.8 6.5c2-2 2.8-4.2 2.8-6.5S12 5.5 10 3.5z" stroke="currentColor" stroke-width="1.2"/>"#
);
// arrow.clockwise: try again
icon!(
    REFRESH,
    "mdow-co-refresh",
    r#"<path d="M15.5 10a5.5 5.5 0 11-1.6-3.9M15.5 4v3.5H12" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#
);
// arrow.up: send
icon!(
    ARROW_UP,
    "mdow-co-arrow-up",
    r#"<path d="M10 15.5v-11M5.5 9L10 4.5 14.5 9" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"#
);
