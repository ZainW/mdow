import { Node } from '@tiptap/core'

export const Frontmatter = Node.create({
  name: 'frontmatter',
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
    return [{ tag: 'pre[data-type="frontmatter"]' }]
  },
  renderHTML({ node, HTMLAttributes }) {
    return [
      'pre',
      { 'data-type': 'frontmatter', class: 'frontmatter-block', ...HTMLAttributes },
      node.attrs.source as string,
    ]
  },
})
