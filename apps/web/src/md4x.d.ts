declare module 'md4x' {
  export interface HtmlOptions {
    heal?: boolean
    full?: boolean
    headingIds?: boolean
  }

  export function init(): Promise<void>
  export function renderToHtml(markdown: string, opts?: HtmlOptions): string
}
