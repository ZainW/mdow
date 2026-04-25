import { Node } from '@tiptap/core'

export const Frontmatter = Node.create({
  name: 'frontmatter',
  group: 'block',
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      source: { default: '' },
    }
  },
  parseHTML() {
    return [{ tag: 'div[data-type="frontmatter"]' }]
  },
  renderHTML({ HTMLAttributes }) {
    return ['div', { 'data-type': 'frontmatter', ...HTMLAttributes }]
  },
})
