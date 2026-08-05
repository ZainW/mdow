---
title: AI Companion
description: Ask questions about local markdown files through OpenCode and ACP
category: Guide
order: 3
---

# AI Companion

Mdow includes a local, read-only AI companion for understanding documents without moving your workflow into another editor. It connects to installed AI providers through the Agent Client Protocol (ACP).

## Supported context

You control what accompanies each question:

- The focused document
- Supported files from the open folder
- Specific files or folders selected with `@` tags

Mdow shows the context it assembled and adds clickable citations to answers so you can return to the relevant source.

## Providers

Mdow can detect OpenCode and Codex ACP installations. You can also select a compatible custom executable. Provider and model availability depends on what is installed and configured on your machine.

Using OpenCode means Mdow can work with providers and subscriptions already configured there instead of requiring a separate Mdow model account.

## Read-only by design

The companion can read the context you select and answer questions about it, but it does not edit files. Continue using your preferred editor for changes while Mdow stays focused on reading and understanding.

## Source citations

Answers can reference the local files used as context. Select a citation to open its source in Mdow and verify the answer against the original document.
