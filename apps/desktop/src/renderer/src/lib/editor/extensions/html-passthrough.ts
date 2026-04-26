import { Node } from '@tiptap/core'

export const HtmlPassthrough = Node.create({
  name: 'htmlBlock',
  group: 'block',
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      source: {
        default: '',
        parseHTML: (e) => e.getAttribute('data-source') ?? '',
        renderHTML: (attrs) => ({ 'data-source': attrs.source }),
      },
    }
  },
  parseHTML() {
    return [{ tag: 'pre[data-type="html-block"]' }]
  },
  // Render the raw HTML source as visible text. Rendering it as live HTML
  // would require sanitization (XSS risk in a markdown viewer that opens
  // arbitrary files); a pre block is safe and v1-acceptable.
  renderHTML({ node, HTMLAttributes }) {
    return [
      'pre',
      { 'data-type': 'html-block', class: 'html-passthrough', ...HTMLAttributes },
      node.attrs.source as string,
    ]
  },
})
