/**
 * Convert heading text to a URL-safe id.
 * Used by both heading id injection (server-render) and TOC extraction (loader).
 * MUST stay in sync between both call sites.
 */
export function slugify(text: string): string {
  return text
    .toLowerCase()
    .trim()
    .replace(/[^\w\s-]/g, '')
    .replace(/\s+/g, '-')
}
