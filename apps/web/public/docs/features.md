---
title: Features
description: What makes Mdow special
category: Guide
order: 4
---

# Features

## Native and GPU-rendered

Mdow is written in Rust on Rune, a UI framework with its own GPU renderer. There's no browser engine behind the page. Each top-level block is its own virtual row, so long documents open and scroll as easily as short ones.

## AI companion

Ask about the focused document, an open folder, or files and folders picked with `@` tags, with Claude Code, Codex, Cursor or OpenCode. Citations link back to the source files, and any edit an agent suggests comes back as a diff for you to accept or decline. See the [AI companion guide](/docs/ai-companion).

## Tabbed reading

Open multiple documents at once and switch between them from the tab bar. Close tabs with a click or keyboard shortcut, reorder them with drag and drop, and reopen recent files from the sidebar or app menu.

![Mdow with several tabs open and the Outline sidebar](/screenshots/outline-light.webp)

## Side-by-side reading

Use split view to compare two documents in the same window. Each pane can target a different tab, so you can keep source notes beside rendered docs or compare Markdown and HTML output.

## Syntax highlighting

Code blocks get syntax colours for every grammar bat ships, a language label, and a copy button on hover, in both light and dark themes.

![Mdow rendering a document with syntax highlighting and Mermaid](/screenshots/reading-light.webp)

## Mermaid diagrams

Mermaid blocks are drawn natively, in the theme's colours, right where you wrote them. No browser or JavaScript is involved.

## Math

`$$…$$` and ` ```math ` blocks, and inline math, are typeset natively with STIX Two Math.

## Light and dark themes

Mdow follows your system's appearance, or you can pick Light or Dark in Settings. Light is warm paper, dark is neutral ink, with one clay accent.

## Omarchy themes

On Omarchy, with Appearance on Automatic, Mdow takes its colours from the current theme and its corners from Hyprland. Run `omarchy theme set` and open windows recolour, diagrams and formulas included.

## HTML documents

Open local `.html` and `.htm` files alongside Markdown. Mdow renders HTML previews in a sandboxed viewer and resolves local document links through the same safe file-opening flow.

## File tree sidebar

Open a folder and browse its contents in a collapsible tree view. Click any Markdown or HTML file to view it instantly, filter the visible folder list, and switch between Recents, Outline, and Folder views from the sidebar.

## Command palette

Press `Cmd+K` (or `Ctrl+K` on Linux) to open the command palette. Jump to files, open folders, toggle the sidebar, switch wide or split view, open settings, and more without leaving the keyboard. See the [shortcuts page](/docs/shortcuts) for the full list.

## Find in document

Press `Cmd+F` to search within the current document. Matches are highlighted as you type, with keyboard navigation between results.

## Live file watching

When you edit a file in your text editor, Mdow picks up changes automatically and refreshes the view.

## Text width

Choose how wide the reading column is in Settings, or from View › Text Width on the Mac.

## Drag and drop

Drop `.md`, `.markdown`, `.mdx`, `.html`, or `.htm` files anywhere in the window to open them instantly. You can also drop a folder to browse supported documents from the sidebar.
