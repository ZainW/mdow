export interface SearchResult {
  path: string
  name: string
  score: number
}

function fuzzyScore(query: string, target: string): number {
  const q = query.toLowerCase()
  const t = target.toLowerCase()

  let score = 0
  let queryIndex = 0
  let consecutive = 0

  for (let i = 0; i < t.length && queryIndex < q.length; i++) {
    if (t[i] === q[queryIndex]) {
      score += 1

      if (consecutive > 0) {
        score += consecutive * 2
      }
      consecutive++

      if (i === 0 || t[i - 1] === '/' || t[i - 1] === '-' || t[i - 1] === '_' || t[i - 1] === '.') {
        score += 5
      }

      if (queryIndex === 0 && i === 0) {
        score += 3
      }

      queryIndex++
    } else {
      consecutive = 0
    }
  }

  if (queryIndex < q.length) return 0

  return score
}

export function fuzzySearch(
  query: string,
  items: { path: string; name: string }[],
  maxResults = 50
): SearchResult[] {
  if (!query.trim()) return items.slice(0, maxResults).map((item) => ({ ...item, score: 0 }))

  const results: SearchResult[] = []

  for (const item of items) {
    const nameScore = fuzzyScore(query, item.name)
    const pathScore = fuzzyScore(query, item.path)
    const score = Math.max(nameScore * 1.5, pathScore)

    if (score > 0) {
      results.push({ ...item, score })
    }
  }

  results.sort((a, b) => b.score - a.score)
  return results.slice(0, maxResults)
}
