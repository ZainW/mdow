import { Node } from '@tiptap/core'

export const MermaidBlock = Node.create({
  name: 'mermaidBlock',
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
    return [{ tag: 'div[data-type="mermaid"]' }]
  },
  renderHTML({ HTMLAttributes }) {
    return ['div', { 'data-type': 'mermaid', ...HTMLAttributes }]
  },
  // Node view comes in Task 9 alongside the Editor component.
})
