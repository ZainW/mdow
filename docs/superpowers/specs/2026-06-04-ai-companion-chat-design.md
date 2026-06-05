# AI Companion Chat

Date: 2026-06-04

## Goal

Add an AI companion chat to Mdow so users can ask questions about the markdown documents they are
reading. The first version is a read-only docs companion. It should connect to local ACP-compatible
agents already on the user's computer, with opencode ACP and a Codex ACP adapter as the preferred
starting points.

The feature has two surfaces that share one conversation:

- A compact right-side companion panel for asking questions while reading.
- A full-screen expanded mode for focused chat without starting a separate session.

## Selected Direction

Build an ACP-first read-only companion.

Mdow acts as the ACP client. A new Electron main-process companion service starts and talks to a
local ACP agent process over stdio. The renderer never spawns commands and never receives secrets.
It only uses typed IPC methods and streamed update events.

The initial provider candidates are:

- `opencode acp`
- `npx @zed-industries/codex-acp`
- A user-configured custom command

Provider detection should try built-in candidates and the custom command. If no provider is usable,
the companion shows a setup empty state with detected provider rows, install hints, retry, and a
custom command field.

Detection must not silently install packages or trigger interactive package-manager prompts. If the
Codex ACP adapter is not already available, Mdow should show an install hint instead of invoking an
installing `npx` flow.

The first version does not expose filesystem-write, shell, terminal, or editing capabilities. It
answers from the active document plus markdown files in the open folder, and it renders validated
source citations back into Mdow.

## Current App Context

Mdow is an Electron app with:

- Main-process file, folder, store, path-validation, and IPC services.
- A preload API that exposes typed renderer-safe methods on `window.api`.
- A React renderer with Zustand for UI state and TanStack Query for async data.
- A left sidebar with persisted Recents, Folder, and Outline modes.
- shadcn/ui components in `apps/desktop/src/renderer/src/components/ui/`.
- Tailwind CSS v4 tokens and compact UI scale variables in `index.css`.

The companion should fit these patterns instead of introducing renderer-side subprocess access or a
parallel state architecture.

## Architecture

New main-process companion units:

- `companion/provider-detection.ts`
  - Checks whether candidate provider commands can start or report usable ACP support.
  - Returns provider IDs, labels, command display strings, and availability status.

- `companion/acp-client.ts`
  - Wraps ACP JSON-RPC over child-process stdio.
  - Handles initialize, optional authentication, session creation, prompt turns, streaming updates,
    cancellation, and shutdown.
  - Converts ACP events into Mdow companion update types.

- `companion/context-builder.ts`
  - Builds bounded read-only context from the active document and open folder markdown files.
  - Uses existing path validation and markdown extension rules.
  - Assigns stable context source IDs that can be validated before rendering citations.

- `companion/service.ts`
  - Owns provider selection, process lifecycle, current ACP session, prompt orchestration, and IPC
    events.
  - Refuses any write, shell, terminal, or unsupported tool request in read-only mode.

New shared types:

- Provider status and companion settings.
- Session, message, streaming update, citation, and context summary types.
- IPC constants for companion actions and events.

Renderer integration:

- Zustand keeps session-only messages and UI state.
- Provider preference and custom command persist through `electron-store`.
- `App.tsx` renders the right companion panel beside the document area and renders the full-screen
  surface near existing dialogs.

## IPC Shape

The exact names can be refined during implementation, but the first version should have channels
with these responsibilities:

- `companion:detect-providers`: return detected provider statuses.
- `companion:get-settings`: return preferred provider and custom command.
- `companion:save-settings`: persist preferred provider and custom command.
- `companion:start-session`: start or reuse a read-only ACP session for the selected provider.
- `companion:send`: send a user prompt with active tab and open-folder context metadata.
- `companion:cancel`: cancel the active prompt turn.
- `companion:shutdown`: stop the active provider process when appropriate.
- `companion:update`: stream assistant deltas, status, citations, warnings, errors, and completion.

The preload API exposes these through `window.api` with typed methods and subscriptions, matching
the existing IPC style.

## Context And Data Flow

When the user sends a message:

1. The renderer immediately appends a local user message.
2. The renderer calls the companion send IPC method with the selected provider and current app
   context metadata.
3. The main process starts or reuses the provider ACP session.
4. The context builder reads the active document first, then open-folder markdown files within size
   limits.
5. The ACP prompt includes the user question, context packet, source IDs, and read-only system
   instructions.
6. ACP updates stream back to the renderer as companion update events.
7. The renderer appends assistant deltas, renders status, and validates citations against the
   context source IDs before showing chips.

The prompt should tell the provider:

- Answer from the provided docs context.
- Cite source IDs and headings when making doc-specific claims.
- Say when the provided docs do not contain enough information.
- Do not edit files, run commands, or request tools.
- Mdow is read-only for this companion mode.

Context limits should be explicit. The active document gets priority. Folder context can use a
simple first-version ranking based on markdown files in the open folder, basename/title matches,
headings, and content snippets. Large folders should produce a context warning and truncation
summary rather than silently failing.

## UI Design

The compact companion lives in a right-side panel. It does not replace or modify the existing left
sidebar modes. Users can keep Recents, Folder, or Outline visible while chatting.

Right panel structure:

- Header with title, provider/status chip, expand button, and close button.
- Message stream in the middle.
- Composer at the bottom with multiline input, send/cancel control, and a small context summary
  such as `Using README.md + 14 docs`.

Full-screen mode:

- Uses the same conversation, not a separate workspace.
- Expands into a modal or overlay-style focused chat surface.
- Provides more room for messages, sources, and context summary.
- Includes a clear return control back to the compact panel.
- `Esc` exits full-screen.

Responsive behavior:

- On small windows, the right panel should become an overlay or sheet instead of shrinking the
  document into an unusable column.
- Touch targets and focus states must remain accessible.

Keyboard behavior:

- `Enter` sends from the composer.
- `Shift+Enter` inserts a newline.
- While streaming, the primary composer action becomes cancel.
- `Esc` closes full-screen or returns focus to the document where appropriate.

## AI Elements And shadcn Usage

Prefer existing Mdow shadcn/ui primitives and Vercel AI Elements components wherever practical.
AI Elements should be treated like shadcn registry components: copied into the app and owned
locally, not used as opaque remote imports.

Prioritize these AI Elements primitives for the companion UI:

- `Conversation`, `ConversationContent`, and `ConversationScrollButton` for the scrollable message
  area.
- `Message` and `MessageContent` for user and assistant turns.
- `PromptInput` for the multiline composer.
- `Response` for streamed markdown-style assistant content when it fits Mdow's renderer constraints.
- `Sources` for citation disclosure or source chips.

Adapt copied components to:

- Mdow's `@renderer/*` imports.
- Tailwind v4 tokens and CSS variables.
- Existing `Button`, `Dialog`, `Sheet`, and form primitives.
- Electron/Vite streaming over IPC rather than Next.js API routes or server actions.

Do not let imported chat components force a visual style that conflicts with Mdow's quiet reader
interface. The final UI should feel like Mdow, not a generic web chatbot.

## Empty States

No provider:

- Show provider rows for opencode, Codex ACP adapter, and custom command.
- Show install or setup hints.
- Let users retry detection.
- Let users enter a custom command.
- Warn that Mdow will run custom commands as local subprocesses.

No active or open docs:

- Prompt the user to open a markdown file or folder before asking doc-specific questions.

Provider starting:

- Show a compact loading state with cancel or retry as appropriate.

Provider error:

- Show the failed provider label and command display string.
- Preserve the user message.
- Offer retry and configure actions.

Context warning:

- Show when active file or folder files were missing, inaccessible, omitted, or truncated.

## Citations

Assistant answers should include source links back into Mdow.

First-version citation strategy:

- The context builder assigns IDs to each included file or heading snippet.
- The prompt instructs the provider to cite those IDs.
- The renderer validates returned IDs against the context packet.
- Valid citations render as compact file/heading chips below the assistant answer.
- Invalid citations are omitted or shown as plain text, not clickable trusted links.

Clicking a citation should open the referenced markdown file. If a heading ID is available, Mdow
should scroll to that heading after opening the file.

Strict refusal is not required. If the docs do not contain enough information, the companion should
say so and qualify the answer.

## State And Persistence

Session-only state:

- Current conversation messages.
- Streaming status.
- Active context summary and warnings.
- Panel open/closed and full-screen state.

Persisted settings:

- Preferred companion provider.
- Custom provider command.

Do not persist chat history in the first version. Quitting Mdow clears messages.

## Safety And Privacy

The read-only boundary is explicit:

- Renderer never starts provider commands.
- Provider subprocesses are managed only in the Electron main process.
- The initial ACP client advertises no terminal, write, or edit capabilities.
- Any provider request for filesystem writes, shell commands, or unsupported tools is refused.
- The UI explains that custom commands are local subprocesses.

Context is sent only to the selected local ACP provider process. The provider may then use its own
configured model backend. The setup UI should communicate that local process handoff clearly.

## Error Handling

Provider errors:

- Detection can report available, missing, or failed-to-start.
- Startup failure shows a concise summary and optional stderr details if useful.
- Streaming errors preserve messages and show retry.
- Cancel stops the active turn without clearing the conversation.

Context errors:

- Missing or deleted files are omitted with a warning.
- Inaccessible files are omitted with a warning.
- Non-markdown files are excluded.
- Large folders include truncation counts.

ACP protocol errors:

- Initialization or authentication failures surface as provider setup errors.
- Malformed updates are ignored or converted to a safe error message.
- Unsupported tool requests are refused in read-only mode.

## Components

Renderer components:

- `CompanionPanel.tsx`: right-side compact companion surface.
- `CompanionFullscreen.tsx`: expanded shared-session surface.
- `CompanionMessages.tsx`: message list, streaming assistant content, citations, and warnings.
- `CompanionComposer.tsx`: prompt input, send, cancel, and keyboard handling.
- `CompanionSetup.tsx`: provider detection and custom command setup state.
- `CompanionStatus.tsx`: provider/status chip and context summary pieces if needed.
- `store/slices/companion-slice.ts`: messages, open/full-screen flags, streaming state, provider
  selection, and context summary.

Existing component integration:

- `App.tsx`: render the right panel and full-screen surface.
- `SettingsDialog.tsx`: add a compact Companion provider section for preferred provider and custom
  command.
- `useAppBindings.ts`: first version can rely on buttons; keyboard/menu shortcuts can be a follow-up.
- `shared/types.ts`: add companion types and IPC constants.
- `preload/index.ts`: expose typed companion APIs.

## Rejected Alternatives

UI-first mocked provider:

- Fastest way to polish the panel and full-screen UI.
- Delays proving the central requirement: connecting to local opencode/Codex ACP providers.

Direct CLI prompt wrapper:

- Simpler than ACP at first glance.
- Weaker streaming/session control and a worse path to ask-before-tools or full agent mode.

Full agent mode first:

- Feasible through ACP, but too much risk for the first iteration.
- Requires permission, filesystem, shell, and edit boundaries before it can be safe.

New left sidebar mode:

- Easier shell integration.
- Chat would hide Recents, Folder, or Outline while active.

Bottom dock:

- Preserves sidebars.
- Feels more like a command box than a companion and reduces reading height.

## Future Phases

Ask-before-tools mode:

- Let providers propose tool actions.
- Mdow shows explicit approvals for each file edit or command.
- Approved actions remain bounded by workspace and path validation.

Full agent mode:

- Enable normal ACP tool loops after the permission model is proven.
- Add clear workspace boundaries, command approvals, audit trail, and recovery UX.

Persistent chats:

- Restore prior conversations per folder or document if users want history.
- Keep the first version session-only.

Better retrieval:

- Add stronger folder search and citation extraction if simple context ranking is not enough.

## Testing

Main-process tests:

- Provider detection returns expected status for mocked opencode, Codex adapter, custom command, and
  missing commands.
- ACP client handles initialize, session creation, prompt updates, cancellation, shutdown, and
  malformed events with mocked stdio.
- Context builder includes the active document first, includes open-folder markdown snippets,
  excludes non-markdown files, validates paths, and respects size limits.
- Tool requests are refused in read-only mode.

Renderer tests:

- Right companion panel opens and closes without changing the left sidebar mode.
- Full-screen expands the same conversation and collapses back without losing messages.
- Setup empty state shows provider rows, install hints, custom command, retry, and safety warning.
- Composer handles send, disabled state, streaming/cancel state, `Enter`, and `Shift+Enter`.
- Citations render as file/heading chips and trigger open/scroll behavior.
- Provider errors and context warnings render accessibly.

Verification commands:

- `pnpm run --filter desktop test -- -t Companion`
- `pnpm run --filter desktop typecheck`
- `pnpm run --filter desktop lint`
- `pnpm run --filter desktop fmt:check`
- `pnpm run test`

Manual verification:

- No provider installed.
- opencode ACP installed.
- Codex ACP adapter installed if available.
- Custom command success and failure.
- Light and dark themes.
- Narrow window behavior.
