declare module 'md4x' {
  export function init(): Promise<void>
  export function renderToHtml(markdown: string): string
}
