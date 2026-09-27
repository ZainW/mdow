import { renderMarkdownInThread } from './markdown'

interface RenderRequest {
  id: number
  text: string
}

self.onmessage = async (event: MessageEvent<RenderRequest>) => {
  const { id, text } = event.data
  try {
    const result = await renderMarkdownInThread(text)
    self.postMessage({ id, result })
  } catch (error) {
    self.postMessage({ id, error: error instanceof Error ? error.message : String(error) })
  }
}
