# AI companion chat

Date: 2026-07-22

Back-link to design: [AI Companion Chat spec](../../specs/2026-06-04-ai-companion-chat-design.md)

## Context

Mdow is a quiet, view-only markdown reader. Users already open a folder of docs and want an AI that can answer from those docs, cite sources, and later propose edits. A June 2026 design already chose an ACP-first companion with opencode and Codex as providers. No companion code exists yet.

This plan implements that design, with two deltas from the new product ask:

1. Explicit `@file` / `@folder` tagging in the composer, not only automatic folder context.
2. Clear separation between OpenCode the agent (`opencode acp`) and OpenCode Go (a model subscription users configure inside OpenCode). Go is not a separate Mdow provider.

## Scope

Included in this program:

- Right-side companion panel plus full-screen mode sharing one conversation.
- Main-process ACP client over stdio. Providers: `opencode acp`, Codex ACP adapter when installed, custom command.
- Read-only docs context from the active tab, tagged files, and open-folder markdown.
- Citations that open/scroll into Mdow.
- Provider detection and setup empty state. No silent installs.

Explicitly out of the first shippable unit (v1):

- Writing files, shell, or terminal tools.
- Persisted chat history.
- Bundling or auto-installing agents.
- Direct OpenCode HTTP SDK or Codex app-server as primary transports (ACP is the contract; native SDKs stay future options if ACP gaps appear).

Edit capability lands in a later phase as ask-before-write ACP permissions, once the read-only path is proven.

Managed Workers AI chat from the [Mdow Pro program design](../../specs/2026-07-10-mdow-pro-program-design.md) stays out of this program. Keep the companion provider ID and IPC shapes open so a future `managed` provider can sit beside local ACP without a UI rewrite.

## Constraints

- Renderer stays sandboxed. No Node, no spawning agents, no secrets.
- Reuse existing path validation, markdown extension rules, Zustand slices, typed IPC.
- Fit Mdow's quiet reader look. Prefer local AI Elements copies adapted to Mdow tokens.
- Detection must not run interactive package managers.

## Alternatives

| Approach                                    | Verdict                                                                             |
| ------------------------------------------- | ----------------------------------------------------------------------------------- |
| ACP client (existing design)                | Chosen. One protocol covers opencode and Codex. Matches "bring your own AI".        |
| Embed `@opencode-ai/sdk` + `opencode serve` | Strong TypeScript DX, but OpenCode-only. Rejected as primary. Keep as escape hatch. |
| Embed `@openai/codex-sdk` / app-server      | Good for Codex GUI streaming. Same lock-in problem. Rejected as primary.            |
| Own chat-completions + tools                | Forces Mdow to own the agent loop, auth, and models. Too much surface for a reader. |

OpenCode Go fits as the user's model path inside OpenCode after they `/connect` their Go key. Mdow talks to the agent, not to Go's HTTP endpoints.

## Applicable skills

- `how` before changing IPC, store, or App shell.
- `architect` if the ACP client shape is contested during implementation.
- `control-ui` (cursor-team-kit) for Electron runtime verification.
- `/deslop` before each commit. `unslop` on prose.
- `show-me-your-work` for the decision trail under `.audit/ai-companion-chat.tsv`.
- `babysit` after the PR opens.

## Phases

1. [phase-1-domain-types-ipc.md](./phase-1-domain-types-ipc.md)
2. [phase-2-acp-client.md](./phase-2-acp-client.md)
3. [phase-3-context-and-tags.md](./phase-3-context-and-tags.md)
4. [phase-4-companion-service.md](./phase-4-companion-service.md)
5. [phase-5-panel-ui.md](./phase-5-panel-ui.md)
6. [phase-6-citations-setup.md](./phase-6-citations-setup.md)
7. [phase-7-ask-before-edit.md](./phase-7-ask-before-edit.md) (follow-up ship)

See also [testing.md](./testing.md).

## Verification

Project-level:

- `pnpm run --filter desktop test -- -t Companion`
- `pnpm run --filter desktop typecheck`
- `pnpm run --filter desktop lint`
- `pnpm run --filter desktop fmt:check`
- `pnpm run test`

Runtime done predicate (v1):

With `opencode` installed and a folder of markdown open, a user can open the companion, send a question that tags a file, receive a streamed answer with at least one valid citation chip that opens the cited doc, and cancel an in-flight turn. Write/shell tool requests from the agent are refused. Without a provider, setup shows detect/retry/custom command without installing anything.

## Implementation guidance

- Scaffold types and IPC before UI polish.
- One phase, one PR-sized commit unit when possible. Verify before the next phase.
- Prefer the June design file layout under `apps/desktop/src/main/companion/`.
- Do not expand into full agent mode until phase 7's permission model exists.
