# Focused Document and Model Picker Design

## Summary

Mdow Companion will identify the document in the most recently focused pane as the focused
document for every message. Phrases such as “this document,” “this file,” and “the current
document” will resolve to that file explicitly rather than relying on source order.

Companion will also expose a searchable model picker populated from the active ACP provider. The
selected model will persist across app restarts and will be reapplied only when the provider still
advertises it.

## Goals

- Make focused-document references deterministic in single-pane and split-view layouts.
- Resolve focus at send time so a chat remains accurate as the user changes panes.
- Load models from the live ACP session instead of maintaining a hard-coded catalog.
- Switch the current ACP session model without restarting the conversation.
- Persist a preferred model per provider and recover safely when it disappears.
- Keep the picker usable with catalogs containing hundreds of models.

## Non-goals

- Model pricing, context-window, or capability metadata that the provider does not advertise.
- Changing the model during an active response.
- Persisting chat history across app restarts.
- Giving the provider file-write or terminal permissions.

## Focused Document Semantics

The renderer already tracks one `activeTabId`, including when split view is enabled. The active tab
is updated when a pane receives focus. `CompanionComposer` will snapshot that active tab’s path when
the user sends a message.

`buildCompanionContext` will record the focused document separately from the general source list.
If the active path is a readable markdown file, the resulting context packet will include its
source ID as `focusedSourceId`. The prompt will contain a dedicated section:

```text
## Focused document
The user's phrases "this document", "this file", and "current document" refer to:
src:/absolute/path/to/document.md
```

The source remains in the ordinary context list for citation handling. Explicit tags may add more
sources but cannot replace the focused-document identity.

If no markdown document is focused, or the focused file cannot be read, the prompt will explicitly
state that there is no focused document. Companion must not infer one from tags or folder context.

## Model Discovery and Selection

`AcpClient.createSession` will parse the generalized `configOptions` returned by `session/new`.
When it finds a select option whose category is `model`, it will expose:

- The configuration option ID.
- The current model value.
- The provider-advertised model values and labels.

`AcpClient.setModel` will call `session/set_config_option` with the active session ID, model
configuration ID, and selected value. It will parse the returned configuration state and treat the
provider’s returned `currentValue` as authoritative.

No model IDs or labels will be hard-coded. Providers that do not advertise a model option will
continue to work, but the picker will remain hidden.

## Persistence

Companion settings will store a map from provider ID to preferred model ID. A saved value is a
preference, not a command:

1. Start the provider session.
2. Read the provider’s current model and available options.
3. If the saved value is still available and differs from the current value, apply it.
4. If the saved value is unavailable, use the provider’s current value and replace the stale
   preference.

Changing providers keeps each provider’s previous model preference independent.

## User Interface

The Companion header will contain a compact model button between the title and window controls.
Its label uses the provider-advertised model name and truncates within the drawer.

Activating it opens a searchable command-style picker:

- Results filter by both display name and raw model ID.
- Groups are derived from the prefix before the first `/`, with provider labels retained.
- The current model has a checkmark.
- The raw model ID appears as secondary text when it adds useful disambiguation.
- Loading, empty, and provider-error states have explicit copy.

The picker is disabled while a response is streaming or while a model change is pending. A failed
change leaves the previous model selected and shows a non-blocking Companion error.

The composer context row will show the focused document name. This is status, not a removable tag.
Explicit context tags remain removable and visually distinct.

## Data Flow

1. Opening Companion refreshes provider status and settings.
2. The renderer requests a session using the selected provider and the current folder as `cwd`.
3. The main process creates or reuses the ACP session and returns model state.
4. The renderer reconciles the returned state with the persisted preference.
5. Selecting another model invokes a dedicated IPC command and updates state only after success.
6. Sending a message snapshots the active tab path and sends it with the prompt payload.
7. The main process builds context, marks the focused source, and formats the explicit prompt.

Changing the open folder creates a new ACP session so its working directory and later tool scope
remain correct.

## Error Handling

- Missing model configuration: hide the picker without blocking chat.
- Empty model catalog: show the provider’s current value only when available.
- Stale persisted model: fall back to the provider current model and update the setting.
- Model-switch failure: retain the confirmed prior value and display the provider error.
- Provider exit: preserve renderer conversation state and offer restart on the next action.
- Focused file read failure: warn in context status and state that no focused document is present.

## Testing

Unit tests will verify:

- Focused source identity is retained when tags precede it in source ordering.
- Focused source identity is absent when the file is invalid, unreadable, or not markdown.
- Prompt formatting defines “this document” exactly once and cites the correct source ID.
- ACP session parsing accepts current OpenCode `configOptions`.
- Model switching sends the exact `session/set_config_option` request and uses returned state.
- Saved valid models are restored and stale models fall back safely.
- Model switching is blocked during streaming.
- Split-pane focus changes alter the next outgoing `activePath`.
- Picker search, current selection, loading, empty, and error states render accessibly.

Hands-on Electron verification will cover single-pane and split-pane focus, switching models before
and after a prompt, reopening Companion, restarting the app, narrow drawer layout, fullscreen
layout, and light and dark themes.

## Acceptance Criteria

- Asking about “this document” uses the document in the most recently focused pane.
- The current model is visible and selectable from the Companion header.
- The list matches the live OpenCode session and remains usable with a large catalog.
- The selected model survives restart when still available.
- Missing or changed provider model catalogs never prevent ordinary chat.
