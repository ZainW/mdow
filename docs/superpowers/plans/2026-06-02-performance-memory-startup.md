# Performance Memory Startup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce Mdow startup time, renderer bundle weight, and avoidable memory use with measured before/after evidence.

**Architecture:** Keep the current Electron/React architecture, but stop loading expensive renderer modules until their feature is actually needed. Preserve current markdown output and sidebar UX while making heavy parsing/highlighting and folder tree code lazy.

**Tech Stack:** Electron, React 19, Vite/electron-vite, Comark, Shiki, Vitest, Playwright Electron perf harness.

---

### Task 1: Prove Current Lazy-Loading Gaps

**Files:**
- Modify: `apps/desktop/src/renderer/src/lib/markdown.lazy.test.ts`
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.test.tsx`

- [ ] Add a markdown test proving plain documents do not need math, highlight, or Mermaid plugins.
- [ ] Add a sidebar test proving the folder tree module is not loaded while Recents is active.
- [ ] Run targeted tests and confirm they fail against the current code.

### Task 2: Lazy Markdown Plugins and Renderer Import

**Files:**
- Modify: `apps/desktop/src/renderer/src/lib/markdown.ts`
- Modify: `apps/desktop/src/renderer/src/main.tsx`
- Modify: `apps/desktop/src/renderer/src/hooks/useMarkdownRender.ts`

- [ ] Build parser variants from content features so plain documents skip math, Shiki, and Mermaid plugin imports.
- [ ] Dynamically load Shiki themes and only the language grammars used by present code fences.
- [ ] Change renderer startup and markdown rendering hooks to dynamically import `lib/markdown`.
- [ ] Run targeted markdown and startup tests.

### Task 3: Lazy Folder Tree UI

**Files:**
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.tsx`
- Modify: `apps/desktop/src/renderer/src/components/Sidebar.test.tsx`

- [ ] Use `React.lazy`/`Suspense` for `FolderTree`.
- [ ] Keep the empty folder state immediate and avoid loading `@pierre/trees` until a folder tree is visible.
- [ ] Run targeted sidebar tests.

### Task 4: Session Restore and Render Cache Bounds

**Files:**
- Modify: `apps/desktop/src/renderer/src/hooks/useAppInit.ts`
- Modify: `apps/desktop/src/renderer/src/store/slices/tab-slice.ts`
- Test: targeted renderer store/hook tests.

- [ ] Restore the active session tab first and mark the app initialized before hydrating inactive tabs.
- [ ] Bound cached render results so large inactive documents do not accumulate indefinitely.
- [ ] Run targeted tests.

### Task 5: Verify and Measure

**Commands:**
- `pnpm run --filter desktop test -- markdown.lazy Sidebar useMarkdownRender app-store MarkdownView.startup`
- `pnpm run --filter desktop build`
- `pnpm run --filter desktop test:electron:perf`
- `pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test`
- `npx -y react-doctor@latest . --verbose --diff`

- [ ] Record bundle chunk sizes before/after.
- [ ] Record Electron perf harness metrics before/after.
- [ ] Keep only changes with meaningful metric improvement and acceptable UX.
