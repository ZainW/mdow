---
title: AI Companion
description: Ask questions about your markdown with Claude Code, Codex, Cursor or OpenCode
category: Guide
order: 3
---

# AI Companion

The companion is a panel beside the document where you can ask about what you're reading. It runs an agent you already have on your machine, so there's no Mdow account and no separate model bill.

## Agents

Pick an agent from the companion's header. Mdow finds each one on your `PATH`, including your login shell's, and needs no Node install for any of them.

| Agent    | How Mdow runs it                 | Uses                              |
| -------- | -------------------------------- | --------------------------------- |
| OpenCode | Its own server, `opencode serve` | The providers you use in OpenCode |
| Claude   | Claude Code's stream-json mode   | Your Claude plan                  |
| Codex    | Codex's app server               | Your ChatGPT plan                 |
| Cursor   | The Agent Client Protocol (ACP)  | Cursor's agent, read-only         |

To use a command somewhere else, set it under `companionAgentCommands` in Mdow's settings file.

## Context

You choose what goes with each question:

- The focused document
- Files from the open folder
- Specific files or folders, picked with `@` tags

## Source citations

Answers link back to the files they draw on. Select a citation to open its source in Mdow and check the answer against the original.

## Edits need your OK

Mdow never changes a document on its own. Reading, searching and thinking run freely. When an agent wants to edit a file, the change comes back as a diff that you accept or decline, and the tab shows that a review is waiting. Anything else, like running a command, fetching from the web or deleting a file, is declined and shown in the chat.
