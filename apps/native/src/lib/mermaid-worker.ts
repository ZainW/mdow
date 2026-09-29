/// <reference lib="webworker" />
// Lays out Mermaid diagrams off the UI thread. The first diagram warms ELK for ~80ms and large
// ones take longer; on the main thread that was a visible stall right after opening a document.
import { renderMermaid, type MermaidPalette } from './mermaid'

declare const self: Worker

self.onmessage = (event: MessageEvent<{ id: number; source: string; palette: MermaidPalette }>) => {
  const { id, source, palette } = event.data
  self.postMessage({ id, result: renderMermaid(source, palette) })
}
