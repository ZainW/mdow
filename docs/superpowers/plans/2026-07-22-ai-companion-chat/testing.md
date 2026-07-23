# Testing

[Overview](./overview.md)

## Static gates (every phase)

- `pnpm run --filter desktop typecheck`
- `pnpm run --filter desktop lint`
- `pnpm run --filter desktop fmt:check`
- Targeted: `pnpm run --filter desktop test -- -t Companion`

## Main-process suites

- Provider detection: available / missing / failed / custom command. Never invokes installing `npx`.
- ACP client: initialize, session, prompt stream, cancel, malformed events, tool refusal in read-only mode.
- Context builder: active priority, tags, exclusions, limits, warnings.
- Service orchestration: reuse session, error preserves messages, shutdown cleans process.

## Renderer suites

- Panel open/close leaves left sidebar mode alone.
- Fullscreen shares the same messages.
- Composer send, disable while streaming, cancel, Enter / Shift+Enter.
- `@` tag picker inserts file tags into the send payload.
- Setup empty state rows, retry, custom command warning.
- Citation chips open/scroll. Invalid IDs do not become trusted links.

## Runtime (prove-it-works)

Use the Electron control surface, not only unit tests.

1. No provider installed → setup UI, retry, custom command field. No install side effects.
2. `opencode` available → detect shows available → ask about the active doc → streamed answer.
3. Tag a non-active file → answer cites it → chip opens that file.
4. Cancel mid-stream.
5. Narrow window: companion overlays instead of crushing the doc.
6. Light and dark themes.

Phase 7 adds reject/approve write checks on a temp folder.
