/** Anchor ids used by the changelog page and RSS feed, e.g. `v1.10.0` → `v1-10-0`. */
export function versionAnchor(version: string): string {
  return version.replace(/\./g, '-')
}

export function formatReleaseDate(iso: string, month: 'short' | 'long' = 'short'): string {
  return new Date(iso).toLocaleDateString('en-US', {
    year: 'numeric',
    month,
    day: 'numeric',
    timeZone: 'UTC',
  })
}
