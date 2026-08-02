# Adaptive Companion Efficiency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Companion start with only the focused document, retrieve additional markdown context only when the question requires it, expose honest context usage, and add a polished Base UI model picker backed by the live OpenCode ACP session.

**Status:** Implemented and verified on 2026-08-01. The final repository gate passed typecheck, lint, formatting, and 400 desktop/web tests.

**Architecture:** The main process owns a small context planner, a per-session content ledger, and ACP configuration state. The renderer receives a compact `CompanionContextTrace` and filtered live model options through typed IPC, then renders them with shadcn components backed by Base UI. FFF is passed to OpenCode as a read-only MCP server only when an installed executable is found; otherwise the context planner uses a bounded markdown-only lexical fallback.

**Tech Stack:** Electron, React 19, TypeScript, Zustand, Vitest, Testing Library, Tailwind CSS v4, shadcn/ui `base-mira`, `@base-ui/react`, OpenCode ACP v1, optional `fff-mcp` stdio server.

## Global Constraints

- Do not eagerly inject the open folder or the first N documents.
- Initial context contains one focused markdown document unless the user explicitly attaches another file or folder.
- Use 16 KB as the small-document threshold and the maximum initial source-content budget.
- Use at most 4 KB per additional retrieved range, at most 3 additional ranges per turn, and at most 32 KB combined source content per turn.
- Markdown links are metadata until a retrieval decision reads their targets.
- Context ledger keys are absolute path plus content hash and are cleared when the provider session restarts or the working folder changes.
- Context estimates describe only bytes Mdow injected; they do not claim the provider's total context-window usage.
- Model values come only from the active ACP session's `configOptions`; do not hardcode a global model catalog.
- Only display live values whose IDs begin with `openai/`, `opencode/`, or `opencode-go/`.
- Change models with `session/set_config_option` and treat the returned full `configOptions` array as authoritative.
- Use shadcn/ui components backed by Base UI; do not introduce Radix UI packages.
- FFF is optional and read-only. If `fff-mcp` is absent, Companion remains fully usable with bounded local retrieval.
- Preserve read-only Companion behavior: no writes, terminals, or permission grants.
- Run repository scripts through `pnpm run`; do not invoke oxlint, oxfmt, or tsgo directly.

---

## File Structure

- `apps/desktop/src/main/companion/context-builder.ts` — orchestrates focus-first packet construction and prompt formatting.
- `apps/desktop/src/main/companion/context-selection.ts` — selects whole small docs or deterministic sections from large docs and extracts link metadata.
- `apps/desktop/src/main/companion/context-ledger.ts` — tracks content hashes already sent in the current ACP session.
- `apps/desktop/src/main/companion/retrieval.ts` — detects cross-document intent and performs bounded markdown-only fallback search.
- `apps/desktop/src/main/companion/fff.ts` — resolves an installed `fff-mcp` executable and creates the ACP stdio descriptor.
- `apps/desktop/src/main/companion/acp-client.ts` — retains ACP session config options, changes model configuration, and accepts optional MCP descriptors.
- `apps/desktop/src/main/companion/service.ts` — owns the session ledger, context planner, live models, and renderer updates.
- `apps/desktop/src/shared/types.ts` — shared trace, model, settings, packet, and IPC types.
- `apps/desktop/src/main/ipc.ts` and `apps/desktop/src/preload/index.ts` — typed model-selection bridge.
- `apps/desktop/src/renderer/src/components/companion/CompanionHeader.tsx` — title, provider state, model picker, expand, and close controls.
- `apps/desktop/src/renderer/src/components/companion/CompanionModelPicker.tsx` — Base UI/shadcn combobox for live model options.
- `apps/desktop/src/renderer/src/components/companion/CompanionContextBar.tsx` — compact focused/adaptive/usage capsules and details popover.
- `apps/desktop/src/renderer/src/components/companion/CompanionComposer.tsx` — textarea, attachment mentions, send/cancel controls.
- `apps/desktop/src/renderer/src/components/companion/CompanionMessages.tsx` — conversation rendering extracted from the current panel.
- `apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx` — thin workspace composition and lifecycle wiring.

### Task 1: Focus-first context packet

**Files:**
- Modify: `apps/desktop/src/shared/types.ts`
- Modify: `apps/desktop/src/main/companion/context-builder.ts`
- Modify: `apps/desktop/src/main/companion/context-builder.test.ts`

**Interfaces:**
- Consumes: `BuildContextInput { activePath, openFolderPath, tags, question?, readFile?, scan? }`.
- Produces: `CompanionContextTrace`, `CompanionContextPacket.trace`, and `buildCompanionContext(input): Promise<CompanionContextPacket>` with no implicit open-folder sources.

- [ ] **Step 1: Write the failing regression tests**

```ts
it('does not inject unrelated files from the open folder', async () => {
  const packet = await buildCompanionContext({
    activePath: active,
    openFolderPath: dir,
    tags: [],
    question: 'Summarize this document',
  })
  expect(packet.sources.map((source) => source.path)).toEqual([active])
  expect(packet.trace).toMatchObject({ focusedCount: 1, searchedCount: 0, readRangeCount: 0 })
})

it('keeps explicit file attachments but does not expand a folder tag eagerly', async () => {
  const packet = await buildCompanionContext({
    activePath: active,
    openFolderPath: dir,
    tags: [
      { kind: 'file', path: tagged, sourceId: `tag:${tagged}` },
      { kind: 'folder', path: dir, sourceId: `tag:${dir}` },
    ],
    question: 'Compare the focused and attached file',
  })
  expect(packet.sources.map((source) => source.path)).toEqual([active, tagged])
})
```

- [ ] **Step 2: Run the focused tests and confirm the eager-folder regression**

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/context-builder.test.ts`

Expected: the first test fails because `other.md` is included and `trace` is absent.

- [ ] **Step 3: Add the trace contract and remove implicit folder reads**

```ts
export interface CompanionContextTrace {
  focusedCount: number
  attachedCount: number
  searchedCount: number
  readRangeCount: number
  injectedBytes: number
  estimatedTokens: number
  retrievalMode: 'focused-only' | 'adaptive-local' | 'adaptive-fff'
  items: Array<{ path: string; reason: 'focused' | 'attached' | 'retrieved'; bytes: number }>
}

export interface CompanionContextPacket {
  sources: CompanionContextSource[]
  warnings: string[]
  summary: string
  trace: CompanionContextTrace
}
```

Set `MAX_INITIAL_SOURCE_BYTES = 16_384`, add only `activePath` plus explicit file tags, retain folder tags as retrieval scope metadata, remove the final `if (input.openFolderPath)` scan, and compute `estimatedTokens` as `Math.ceil(injectedBytes / 4)`.

- [ ] **Step 4: Update prompt formatting and existing fixtures for the trace field**

The prompt instruction must say that linked files and folders may be searched only when the question requires them; it must continue to refuse edits, terminal use, and write tools.

- [ ] **Step 5: Run the focused tests**

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/context-builder.test.ts`

Expected: all context-builder tests pass and only explicit files appear in `sources`.

- [ ] **Step 6: Commit the focus-first regression fix**

```bash
git add apps/desktop/src/shared/types.ts apps/desktop/src/main/companion/context-builder.ts apps/desktop/src/main/companion/context-builder.test.ts
git commit -m "fix: keep companion context focus first"
```

### Task 2: Large-document selection, link manifest, and session ledger

**Files:**
- Create: `apps/desktop/src/main/companion/context-selection.ts`
- Create: `apps/desktop/src/main/companion/context-selection.test.ts`
- Create: `apps/desktop/src/main/companion/context-ledger.ts`
- Create: `apps/desktop/src/main/companion/context-ledger.test.ts`
- Modify: `apps/desktop/src/main/companion/context-builder.ts`
- Modify: `apps/desktop/src/main/companion/context-builder.test.ts`

**Interfaces:**
- Produces: `selectInitialMarkdown(content: string, question: string, maxBytes?: number): SelectedMarkdown`.
- Produces: `ContextLedger.record(path: string, content: string): { hash: string; alreadySent: boolean }`, `has(path, hash)`, and `clear()`.
- `SelectedMarkdown` is `{ excerpt: string; bytes: number; wholeDocument: boolean; headings: Array<{ depth: number; text: string; line: number }>; links: Array<{ label: string; target: string }> }`.

- [ ] **Step 1: Write selection tests for small and large documents**

```ts
it('returns a small markdown document whole', () => {
  const selected = selectInitialMarkdown('# Intro\nComplete body', 'summarize')
  expect(selected.wholeDocument).toBe(true)
  expect(selected.excerpt).toContain('Complete body')
})

it('selects question-relevant sections from a large document', () => {
  const selected = selectInitialMarkdown(largeMarkdown, 'How does authentication work?')
  expect(selected.wholeDocument).toBe(false)
  expect(selected.excerpt).toContain('## Authentication')
  expect(selected.excerpt).not.toContain('x'.repeat(20_000))
  expect(selected.bytes).toBeLessThanOrEqual(16_384)
})

it('extracts markdown links as metadata without reading targets', () => {
  const selected = selectInitialMarkdown('[API guide](./api.md)', 'summarize')
  expect(selected.links).toEqual([{ label: 'API guide', target: './api.md' }])
})
```

- [ ] **Step 2: Run the new selection tests and confirm the module is missing**

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/context-selection.test.ts`

Expected: FAIL because `context-selection.ts` does not exist.

- [ ] **Step 3: Implement deterministic markdown selection**

Parse ATX headings line-by-line, score sections by case-insensitive question-term overlap in heading and body, preserve source order for equal scores, include the document title/heading map, then append top sections while the UTF-8 byte count remains at or below 16,384. Extract inline and reference-style markdown links, excluding `http:`, `https:`, `mailto:`, and hash-only targets.

- [ ] **Step 4: Write and run ledger tests**

```ts
it('marks unchanged content as already sent and changed content as new', () => {
  const ledger = new ContextLedger()
  expect(ledger.record('/docs/a.md', 'one').alreadySent).toBe(false)
  expect(ledger.record('/docs/a.md', 'one').alreadySent).toBe(true)
  expect(ledger.record('/docs/a.md', 'two').alreadySent).toBe(false)
  ledger.clear()
  expect(ledger.record('/docs/a.md', 'two').alreadySent).toBe(false)
})
```

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/context-ledger.test.ts`

Expected before implementation: FAIL because `ContextLedger` is missing. Expected after implementation: PASS using SHA-256 from `node:crypto`.

- [ ] **Step 5: Integrate selection and ledger-aware identity prompts**

Have `buildCompanionContext` call `selectInitialMarkdown`. Accept an optional `ledger?: ContextLedger`; for unchanged content emit a compact source block containing path, SHA-256 identity, heading map, and the sentence `Content unchanged from earlier in this session` instead of resending the excerpt. Count only newly injected bytes in the trace.

- [ ] **Step 6: Run all companion main-process tests**

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/*.test.ts`

Expected: selection, ledger, builder, ACP, provider, and service tests pass.

- [ ] **Step 7: Commit section selection and ledger**

```bash
git add apps/desktop/src/main/companion/context-selection.ts apps/desktop/src/main/companion/context-selection.test.ts apps/desktop/src/main/companion/context-ledger.ts apps/desktop/src/main/companion/context-ledger.test.ts apps/desktop/src/main/companion/context-builder.ts apps/desktop/src/main/companion/context-builder.test.ts
git commit -m "feat: add bounded companion context selection"
```

### Task 3: Adaptive retrieval with optional FFF

**Files:**
- Create: `apps/desktop/src/main/companion/retrieval.ts`
- Create: `apps/desktop/src/main/companion/retrieval.test.ts`
- Create: `apps/desktop/src/main/companion/fff.ts`
- Create: `apps/desktop/src/main/companion/fff.test.ts`
- Modify: `apps/desktop/src/main/companion/context-builder.ts`
- Modify: `apps/desktop/src/main/companion/acp-client.ts`
- Modify: `apps/desktop/src/main/companion/acp-client.test.ts`
- Modify: `apps/desktop/src/main/companion/service.ts`
- Modify: `apps/desktop/src/main/companion/service.test.ts`

**Interfaces:**
- Produces: `shouldRetrieve(question, activePath, tags): boolean`.
- Produces: `retrieveMarkdownRanges(input): Promise<RetrievedRange[]>`, where each range is `{ path, excerpt, startLine, endLine, bytes, score }`.
- Produces: `resolveFffMcp(commandExists?): Promise<AcpMcpServer | null>` and `AcpMcpServer { name, command, args, env }`.
- Extends: `AcpClient.createSession(cwd: string, mcpServers?: AcpMcpServer[]): Promise<AcpSessionState>`.

- [ ] **Step 1: Write cross-document intent and bounded retrieval tests**

```ts
expect(shouldRetrieve('Summarize this', active, [])).toBe(false)
expect(shouldRetrieve('Compare this with architecture.md', active, [])).toBe(true)
expect(shouldRetrieve('What do the docs in this folder say about caching?', active, folderTags)).toBe(true)

const ranges = await retrieveMarkdownRanges({
  question: 'How is caching invalidated?',
  roots: [docsDir],
  excludedPaths: [active],
})
expect(ranges.length).toBeLessThanOrEqual(3)
expect(ranges.every((range) => range.bytes <= 4_096)).toBe(true)
expect(ranges.reduce((sum, range) => sum + range.bytes, 0)).toBeLessThanOrEqual(12_288)
expect(ranges[0]?.path).toBe(cacheDoc)
```

- [ ] **Step 2: Run retrieval tests and confirm failure**

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/retrieval.test.ts`

Expected: FAIL because the retrieval module is missing.

- [ ] **Step 3: Implement bounded local retrieval**

Scan only explicit folder-tag roots or `openFolderPath`, include only `.md`, `.markdown`, and `.mdx`, ignore hidden/build directories, tokenize meaningful question terms, score filename/heading/body matches, and return at most three 4,096-byte line-aligned excerpts. Skip retrieval entirely when `shouldRetrieve` is false. Keep the combined packet at or below 32,768 bytes.

- [ ] **Step 4: Write FFF resolution and ACP descriptor tests**

```ts
it('returns a read-only stdio descriptor only when fff-mcp exists', async () => {
  await expect(resolveFffMcp(async () => '/opt/homebrew/bin/fff-mcp')).resolves.toEqual({
    name: 'fff',
    command: '/opt/homebrew/bin/fff-mcp',
    args: [],
    env: [],
  })
  await expect(resolveFffMcp(async () => null)).resolves.toBeNull()
})
```

The resolver checks absolute common locations plus `which fff-mcp` without installing anything. `createSession` includes `[descriptor]` only when it is present; otherwise it sends `mcpServers: []`.

- [ ] **Step 5: Integrate adaptive packet construction in the service**

Create one `ContextLedger` per live client. Clear it in `shutdownClient` and whenever `cwd` changes. Resolve FFF before `session/new`; pass its descriptor to OpenCode. Use the bounded local retrieval before the prompt only when `shouldRetrieve` is true, and set trace mode to `adaptive-fff` when FFF is connected or `adaptive-local` otherwise. FFF augments the provider's search capability; the deterministic preflight retrieval keeps context behavior testable and provider-independent.

- [ ] **Step 6: Verify service trace and session-reset behavior**

Add service tests that send the same active file twice and assert the second prompt contains only unchanged-content identity metadata, then start a different working directory and assert full content is sent again. Assert a summary-only question never calls the folder scanner.

- [ ] **Step 7: Run companion main-process tests and commit**

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/*.test.ts`

Expected: all companion main-process tests pass.

```bash
git add apps/desktop/src/main/companion/retrieval.ts apps/desktop/src/main/companion/retrieval.test.ts apps/desktop/src/main/companion/fff.ts apps/desktop/src/main/companion/fff.test.ts apps/desktop/src/main/companion/context-builder.ts apps/desktop/src/main/companion/acp-client.ts apps/desktop/src/main/companion/acp-client.test.ts apps/desktop/src/main/companion/service.ts apps/desktop/src/main/companion/service.test.ts
git commit -m "feat: retrieve companion context adaptively"
```

### Task 4: Live OpenCode model configuration

**Files:**
- Modify: `apps/desktop/src/shared/types.ts`
- Modify: `apps/desktop/src/main/companion/acp-client.ts`
- Modify: `apps/desktop/src/main/companion/acp-client.test.ts`
- Modify: `apps/desktop/src/main/companion/service.ts`
- Modify: `apps/desktop/src/main/companion/service.test.ts`
- Modify: `apps/desktop/src/main/store.ts`
- Modify: `apps/desktop/src/main/ipc.ts`
- Modify: `apps/desktop/src/main/ipc.test.ts`
- Modify: `apps/desktop/src/preload/index.ts`

**Interfaces:**
- Produces: `CompanionModelOption { value, name, description?, provider: 'openai' | 'opencode' | 'opencode-go' }`.
- Produces: `CompanionModelState { options, currentValue, stale, unavailableReason? }`.
- Produces: `AcpClient.getModelState(): CompanionModelState` and `setModel(value: string): Promise<CompanionModelState>`.
- Produces IPC: `getCompanionModels(): Promise<CompanionModelState>` and `setCompanionModel(value: string): Promise<CompanionModelState>`.

- [ ] **Step 1: Extend the fake ACP agent and write failing client tests**

Have fake `session/new` return a `configOptions` array with `category: 'model'`, values from allowed and disallowed prefixes. Add `session/set_config_option` response handling.

```ts
expect(client.getModelState().options.map((option) => option.value)).toEqual([
  'openai/gpt-5.4',
  'opencode/claude-sonnet-4-5',
  'opencode-go/kimi-k2.5',
])
await client.setModel('openai/gpt-5.4')
expect(lastRequest.params).toMatchObject({
  sessionId: 'sess_test',
  configId: 'model',
  value: 'openai/gpt-5.4',
})
```

- [ ] **Step 2: Run the ACP tests and confirm missing configuration support**

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/acp-client.test.ts`

Expected: FAIL because `getModelState` and `setModel` do not exist.

- [ ] **Step 3: Parse and synchronize ACP config options**

Retain the full `configOptions` from `session/new`, select the first option whose category is `model` or whose ID is `model`, filter its values by the three approved prefixes, preserve agent ordering, and map the prefix to the provider group. Handle `config_option_update` notifications. `setModel` rejects a value not in the current filtered options, calls `session/set_config_option`, and replaces all cached config options with the response array.

- [ ] **Step 4: Add service, persistence, IPC, and preload tests**

Add `companionLastModel: string | null` to persisted `AppState` defaults/schema. Expose current live state after `startSession`; apply the saved model only if it exists in the live options. Persist only a model value confirmed by the ACP response. Add IPC validation that accepts a non-empty string and rejects path/control characters.

- [ ] **Step 5: Implement the typed bridge**

Add `COMPANION_GET_MODELS` and `COMPANION_SET_MODEL` constants, service methods `getModels()` and `setModel(value)`, main handlers, and preload methods. When no session exists return `{ options: [], currentValue: null, stale: true, unavailableReason: 'Start Companion to load models' }`.

- [ ] **Step 6: Run the affected test suites and commit**

Run: `pnpm run --filter desktop test -- apps/desktop/src/main/companion/*.test.ts apps/desktop/src/main/ipc.test.ts apps/desktop/src/main/store*.test.ts`

Expected: all affected tests pass.

```bash
git add apps/desktop/src/shared/types.ts apps/desktop/src/main/companion/acp-client.ts apps/desktop/src/main/companion/acp-client.test.ts apps/desktop/src/main/companion/service.ts apps/desktop/src/main/companion/service.test.ts apps/desktop/src/main/store.ts apps/desktop/src/main/ipc.ts apps/desktop/src/main/ipc.test.ts apps/desktop/src/preload/index.ts
git commit -m "feat: expose live opencode models"
```

### Task 5: Base UI model picker and compact context bar

**Files:**
- Create with shadcn CLI: `apps/desktop/src/renderer/src/components/ui/combobox.tsx`
- Create with shadcn CLI: `apps/desktop/src/renderer/src/components/ui/popover.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionModelPicker.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionModelPicker.test.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionContextBar.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionContextBar.test.tsx`
- Modify: `apps/desktop/src/renderer/src/store/slices/companion-slice.ts`
- Modify: `apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`

**Interfaces:**
- Consumes: `CompanionModelState`, `CompanionContextTrace`, `window.api.getCompanionModels`, and `window.api.setCompanionModel`.
- Produces: `CompanionModelPicker({ state, disabled, onStateChange })` and `CompanionContextBar({ trace, warnings })`.

- [ ] **Step 1: Install the Base UI shadcn components**

Run: `npx shadcn@latest add -c apps/desktop combobox popover`

Expected: local shadcn files are generated using the existing `base-mira` style and `@base-ui/react`; `package.json` gains no Radix dependency.

- [ ] **Step 2: Write failing component and store tests**

```tsx
render(<CompanionModelPicker state={modelState} disabled={false} onStateChange={onChange} />)
await user.click(screen.getByRole('combobox', { name: /model/i }))
expect(screen.getByText('ChatGPT subscription')).toBeInTheDocument()
expect(screen.getByText('OpenCode Zen')).toBeInTheDocument()
expect(screen.getByText('OpenCode Go')).toBeInTheDocument()

render(<CompanionContextBar trace={trace} warnings={[]} />)
expect(screen.getByText('1 focused')).toBeInTheDocument()
expect(screen.getByText('Adaptive')).toBeInTheDocument()
expect(screen.getByText(/≈.*added/)).toBeInTheDocument()
```

Run: `pnpm run --filter desktop test -- apps/desktop/src/renderer/src/components/companion/CompanionModelPicker.test.tsx apps/desktop/src/renderer/src/components/companion/CompanionContextBar.test.tsx apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`

Expected: FAIL because the components and state fields are missing.

- [ ] **Step 3: Add model and trace state to the companion slice**

Add `contextTrace: CompanionContextTrace | null`, `modelState: CompanionModelState`, `loadModels()`, and `selectModel(value)`. The `context` update stores both `trace` and warnings. On selection, show the previous confirmed value until `setCompanionModel` resolves; on rejection restore it and set the existing error field.

- [ ] **Step 4: Implement the grouped model combobox**

Group live options by prefix with labels `ChatGPT subscription`, `OpenCode Zen`, and `OpenCode Go`. The trigger shows the selected option's name, supports search, disables unavailable state with the service-provided reason, and does not display providers absent from the live options.

- [ ] **Step 5: Implement the compact context capsules and popover**

Render a single-row bar containing the focused filename/count capsule, an `Adaptive` capsule with local/FFF status in accessible text, and `≈{tokens} added`. The popover lists each trace item with reason and byte estimate plus warnings. Keep the collapsed row under 36 px high and truncate long filenames.

- [ ] **Step 6: Run renderer tests and dependency audit**

Run: `pnpm run --filter desktop test -- apps/desktop/src/renderer/src/components/companion/CompanionModelPicker.test.tsx apps/desktop/src/renderer/src/components/companion/CompanionContextBar.test.tsx apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`

Run: `rg -n "@radix-ui|radix-ui" apps/desktop/package.json pnpm-lock.yaml`

Expected: tests pass and no new Radix package is present.

- [ ] **Step 7: Commit the Base UI controls**

```bash
git add apps/desktop/package.json pnpm-lock.yaml apps/desktop/src/renderer/src/components/ui/combobox.tsx apps/desktop/src/renderer/src/components/ui/popover.tsx apps/desktop/src/renderer/src/components/companion/CompanionModelPicker.tsx apps/desktop/src/renderer/src/components/companion/CompanionModelPicker.test.tsx apps/desktop/src/renderer/src/components/companion/CompanionContextBar.tsx apps/desktop/src/renderer/src/components/companion/CompanionContextBar.test.tsx apps/desktop/src/renderer/src/store/slices/companion-slice.ts apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts
git commit -m "feat: polish companion context controls"
```

### Task 6: Split and integrate the full-screen chat workspace

**Files:**
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionHeader.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionComposer.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionMessages.tsx`
- Modify: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx`
- Modify: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`

**Interfaces:**
- Consumes: the store actions and components from Task 5.
- Produces: the same `CompanionPanel` public props and full-screen workspace behavior already used by `App.tsx`.

- [ ] **Step 1: Add integration tests for the selected UX**

```tsx
expect(screen.getByRole('main', { name: /companion/i })).toBeInTheDocument()
expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
expect(screen.getByRole('combobox', { name: /model/i })).toBeInTheDocument()
expect(screen.getByText('1 focused')).toBeInTheDocument()
expect(screen.queryByText(/using .* \+ .* more/i)).not.toBeInTheDocument()
```

Also assert that opening model/context popovers does not move the composer and that Escape closes the popover before leaving the workspace.

- [ ] **Step 2: Run the panel test and observe missing integration**

Run: `pnpm run --filter desktop test -- apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`

Expected: FAIL until the header, model picker, and context bar are composed.

- [ ] **Step 3: Extract focused presentation components**

Move existing message rendering without behavior changes into `CompanionMessages`, move textarea/mention/send logic into `CompanionComposer`, and move title/provider/expand/close controls into `CompanionHeader`. Keep `CompanionPanel` responsible for lifecycle subscriptions, layout, and composing the three regions.

- [ ] **Step 4: Integrate the model picker and context bar**

Load models immediately after a successful session start. Place the model picker in the header. Place `CompanionContextBar` immediately above the composer, replacing the large summary/warnings block. Retain accessible warning text inside the details popover and continue announcing errors through the existing live region.

- [ ] **Step 5: Run panel and store tests**

Run: `pnpm run --filter desktop test -- apps/desktop/src/renderer/src/components/companion/*.test.tsx apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`

Expected: full-screen workspace, model picker, context bar, composer, and message tests pass.

- [ ] **Step 6: Commit the integrated workspace**

```bash
git add apps/desktop/src/renderer/src/components/companion/CompanionHeader.tsx apps/desktop/src/renderer/src/components/companion/CompanionComposer.tsx apps/desktop/src/renderer/src/components/companion/CompanionMessages.tsx apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx
git commit -m "refactor: compose companion full chat workspace"
```

### Task 7: Full verification and Electron smoke test

**Files:**
- Modify if required by formatting only: files changed in Tasks 1–6.
- Update: `docs/superpowers/plans/2026-08-01-adaptive-companion-efficiency.md` checkbox state.

**Interfaces:**
- Consumes: complete feature.
- Produces: verified repository and a reproducible manual test path.

- [ ] **Step 1: Run the project verification skill command sequence**

Run: `pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test`

Expected: typecheck, lint, formatting, and all Vitest suites pass. Existing lint warnings may remain; no new errors are allowed.

- [ ] **Step 2: Fix only failures caused by this implementation and rerun the exact failing command**

Use `pnpm run fmt` for formatting changes, then rerun `pnpm run fmt:check`. For test/type failures, patch the smallest responsible file and rerun the focused suite before the full sequence.

- [ ] **Step 3: Restart the development Electron process**

Stop the existing Mdow dev process cleanly, then run `pnpm run dev` from the repository root. Confirm the renderer URL is reachable and a development Electron window launches independently of the installed release app.

- [ ] **Step 4: Smoke-test the user flow**

Open a folder containing several markdown files, focus one document, enter full-screen Companion, and ask `Summarize this document`. Confirm the context bar reports one focused source and no search/read ranges. Ask `Compare this with architecture.md`; confirm adaptive retrieval is reported and no more than three ranges are listed. Open the model picker; confirm it reflects only the current OpenCode session's available `openai/`, `opencode/`, and `opencode-go/` values. Switch models and send a message; confirm the selection remains after the ACP response.

- [ ] **Step 5: Inspect the final diff and commit verification adjustments**

Run: `git status --short && git diff --check && git diff --stat HEAD~6..HEAD`

Expected: no whitespace errors, no unexpected files, and only Companion/full-screen/dev-runtime work in scope.

```bash
git add docs/superpowers/plans/2026-08-01-adaptive-companion-efficiency.md
git commit -m "docs: record companion verification"
```
