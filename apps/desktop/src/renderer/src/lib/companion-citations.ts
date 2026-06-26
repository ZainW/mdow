import type { CompanionCitation, CompanionContextSource } from '../../../shared/types'

export function extractCompanionCitations(
  content: string,
  sources: CompanionContextSource[],
): { text: string; citations: CompanionCitation[] } {
  const byId = new Map(sources.map((source) => [source.id, source]))
  const citations: CompanionCitation[] = []
  const text = content
    .replace(/\[\[source:([a-zA-Z0-9_-]+)\]\]/g, (_match, sourceId: string) => {
      const source = byId.get(sourceId)
      if (source && !citations.some((citation) => citation.sourceId === sourceId)) {
        citations.push({
          sourceId,
          title: source.title,
          path: source.path,
          heading: source.heading,
        })
      }
      return ''
    })
    .replace(/[ \t]+\n/g, '\n')
    .trim()
  return { text, citations }
}
