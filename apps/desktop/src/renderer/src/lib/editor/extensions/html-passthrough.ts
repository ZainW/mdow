import { Node } from '@tiptap/core'

export const HtmlPassthrough = Node.create({
  name: 'htmlBlock',
  group: 'block',
  atom: true,
  selectable: true,
  addAttributes() {
    return {
      source: { default: '' },
    }
  },
  parseHTML() {
    return [{ tag: 'div[data-type="html-block"]' }]
  },
  renderHTML({ HTMLAttributes }) {
    return ['div', { 'data-type': 'html-block', ...HTMLAttributes }]
  },
})
