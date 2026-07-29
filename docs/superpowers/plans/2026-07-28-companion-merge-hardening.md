# Companion Merge Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Companion protocol-correct, stream-safe, accessible, responsive, and ready for a
second packaged-app merge audit.

**Architecture:** Preserve the current ACP client → service → IPC → Zustand → React flow. Harden
each boundary with small stateful helpers and explicit terminal updates, then present background
activity through compact disclosures so the answer remains primary.

**Tech Stack:** Electron, TypeScript, React 19, Zustand, Tailwind CSS v4, Vitest, Testing Library.

## Global Constraints

- Use `pnpm run` scripts from the repository instructions.
- Keep Companion read-only and local.
- Do not add dependencies.
- Write and observe a failing regression test before every production behavior change.
- Preserve the existing Mdow design language in light and dark themes.

---

### Task 1: ACP protocol correctness

**Files:**
- Modify: `apps/desktop/src/main/companion/acp-client.ts`
- Modify: `apps/desktop/src/main/companion/acp-client.test.ts`
- Read: `apps/desktop/package.json`

**Interfaces:**
- Consumes: `AcpClientOptions`
- Produces: `AcpClientOptions.requestTimeoutMs?: number`, protocol-correct initialization

- [ ] **Step 1: Add failing initialization and timeout tests**

```ts
expect(messages.map((message) => message.method)).not.toContain('notifications/initialized')
expect(initialize.params.clientCapabilities.fs.readTextFile).toBe(false)
await expect(client.createSession('/tmp/docs')).rejects.toThrow(/session\/new timed out/)
```

- [ ] **Step 2: Run the focused test and confirm the expected failures**

```bash
pnpm run --filter desktop test -- src/main/companion/acp-client.test.ts
```

- [ ] **Step 3: Implement the protocol fix**

```ts
export interface AcpClientOptions {
  requestTimeoutMs?: number
}

// initialize with the package version and truthful capabilities.
// Do not send notifications/initialized.
// Reject and delete pending RPC requests when requestTimeoutMs elapses.
```

- [ ] **Step 4: Re-run the focused test**

```bash
pnpm run --filter desktop test -- src/main/companion/acp-client.test.ts
```

### Task 2: Citation stream and terminal states

**Files:**
- Modify: `apps/desktop/src/main/companion/service.ts`
- Modify: `apps/desktop/src/main/companion/service.test.ts`
- Modify: `apps/desktop/src/shared/types.ts`

**Interfaces:**
- Produces: `CompanionUpdate` variant `{ kind: 'cancelled'; messageId: string }`
- Produces: per-prompt buffered citation sanitizer

- [ ] **Step 1: Add failing split-citation and cancellation tests**

```ts
service.acceptClientUpdate({ kind: 'delta', text: 'See src:/docs/over' })
service.acceptClientUpdate({ kind: 'delta', text: 'view.md for details.' })
expect(sent).toContainEqual(expect.objectContaining({ kind: 'citation' }))
expect(visibleText).toBe('See  for details.')
expect(cancelUpdate.kind).toBe('cancelled')
```

- [ ] **Step 2: Run the focused test and confirm the failures**

```bash
pnpm run --filter desktop test -- src/main/companion/service.test.ts
```

- [ ] **Step 3: Implement buffering, de-duplication, flushing, and cancellation**

```ts
private citationBuffer = ''
private emittedCitationIds = new Set<string>()

private consumeTextDelta(text: string): string
private flushTextDelta(): string
```

- [ ] **Step 4: Re-run service and store tests**

```bash
pnpm run --filter desktop test -- src/main/companion/service.test.ts src/renderer/src/store/slices/companion-slice.test.ts
```

### Task 3: Immediate request state and accessible progressive UI

**Files:**
- Modify: `apps/desktop/src/renderer/src/store/slices/companion-slice.ts`
- Modify: `apps/desktop/src/renderer/src/store/slices/companion-slice.test.ts`
- Modify: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx`
- Create: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`
- Modify: `apps/desktop/src/renderer/src/components/ai-elements/reasoning.tsx`
- Modify: `apps/desktop/src/renderer/src/components/ai-elements/tool.tsx`
- Modify: `apps/desktop/src/renderer/src/components/ai-elements/markdown.tsx`

**Interfaces:**
- Produces: `beginCompanionRequest(): void`
- Produces: `cancelCompanionRequest(): void`
- Produces: keyboard-operable mention listbox

- [ ] **Step 1: Add failing store and component tests**

```ts
useAppStore.getState().beginCompanionRequest()
expect(useAppStore.getState().companionStreaming).toBe(true)
expect(screen.getByRole('button', { name: 'Cancel' })).toBeEnabled()
await user.keyboard('@r{ArrowDown}{Enter}')
expect(screen.getByText('@risks.md')).toBeVisible()
```

- [ ] **Step 2: Run the focused tests and confirm the failures**

```bash
pnpm run --filter desktop test -- src/renderer/src/store/slices/companion-slice.test.ts src/renderer/src/components/companion/CompanionPanel.test.tsx
```

- [ ] **Step 3: Implement the immediate request state and compact activity rows**

```ts
beginCompanionRequest: () => {
  const ensured = ensureAssistant(get().companionMessages)
  set({ companionStreaming: true, companionMessages: ensured.messages })
}
```

Keep thinking and running tools collapsed by default. Derive companion markdown HTML synchronously.

- [ ] **Step 4: Implement combobox keyboard behavior**

Track the active suggestion index, update it on arrow keys, select it on Enter, and dismiss it on
Escape. Apply the appropriate combobox/listbox ARIA attributes.

- [ ] **Step 5: Re-run focused tests**

```bash
pnpm run --filter desktop test -- src/renderer/src/store/slices/companion-slice.test.ts src/renderer/src/components/companion/CompanionPanel.test.tsx
```

### Task 4: Responsive drawer and expanded dialog

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.tsx`
- Modify: `apps/desktop/src/renderer/src/components/companion/CompanionPanel.test.tsx`

**Interfaces:**
- Produces: overlay Companion below `lg`
- Produces: expanded dialog with `sm:max-w-none`

- [ ] **Step 1: Add failing class assertions**

```ts
expect(panel).toHaveClass('max-lg:fixed')
expect(dialog).toHaveClass('sm:max-w-none')
```

- [ ] **Step 2: Run the component test and confirm the failures**

```bash
pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionPanel.test.tsx
```

- [ ] **Step 3: Implement responsive overlay and dialog sizing**

Use a fixed inset-right overlay below `lg` with a viewport-clamped width and retain the inline
drawer at `lg` and above. Add `min-w-0` to the document main region.

- [ ] **Step 4: Re-run the component test**

```bash
pnpm run --filter desktop test -- src/renderer/src/components/companion/CompanionPanel.test.tsx
```

### Task 5: Full verification, package, and visual audit

**Files:**
- Create: Design Report output under the task artifact directory
- Do not modify: the retained Design Report reference

**Interfaces:**
- Produces: verified macOS ARM64 package
- Produces: current-run screenshots and a rendered Design Report DOCX

- [ ] **Step 1: Run repository verification**

```bash
pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test
```

- [ ] **Step 2: Build the macOS distribution**

```bash
pnpm run --filter desktop build:dist -- --mac
```

- [ ] **Step 3: Exercise the packaged app with Computer Use**

Test streaming, thinking, tools, citations, opening a source, cancellation, keyboard mentions,
expanded mode, dark mode, and a 720 px window.

- [ ] **Step 4: Create and verify the retained-template report**

Clone the reference DOCX, fill the documented slots, render every page, inspect every rendered
page, and run the structural/template fidelity checks.
