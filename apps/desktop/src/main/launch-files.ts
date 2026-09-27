import type { WebContents } from 'electron'

// Documents handed to the app before any renderer can receive them: argv on a cold launch, or
// macOS `open-file`, which fires before `ready` when a file is double-clicked in Finder. They wait
// here until a renderer asks for its initial state, instead of being pushed at a page that may
// not be listening yet.
const pending: string[] = []
const readyRenderers = new Set<number>()

export function queueLaunchFile(path: string): void {
  if (!pending.includes(path)) pending.push(path)
}

export function takeLaunchFiles(): string[] {
  return pending.splice(0)
}

export function markRendererReady(contents: WebContents): void {
  const id = contents.id
  if (readyRenderers.has(id)) return
  readyRenderers.add(id)
  contents.once('destroyed', () => readyRenderers.delete(id))
  // A reload re-runs startup, so the renderer must ask again before it can take pushes.
  contents.once('did-start-navigation', () => readyRenderers.delete(id))
}

export function isRendererReady(contents: WebContents): boolean {
  return readyRenderers.has(contents.id)
}
