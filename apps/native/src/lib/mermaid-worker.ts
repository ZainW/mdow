/// <reference lib="webworker" />
// Lays out Mermaid diagrams off the UI thread. Pele is fast, but large diagrams still take a few
// milliseconds that would land right as a document opens.
import { renderMermaid, type MermaidPalette } from './mermaid'

declare const self: Worker

self.onmessage = (event: MessageEvent<{ id: number; source: string; palette: MermaidPalette }>) => {
  const { id, source, palette } = event.data
  self.postMessage({ id, result: renderMermaid(source, palette) })
}
