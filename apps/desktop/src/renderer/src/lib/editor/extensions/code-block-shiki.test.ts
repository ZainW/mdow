import { describe, it, expect } from 'vitest'
import { Editor } from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import { CodeBlockShiki } from './code-block-shiki'

describe('CodeBlockShiki', () => {
  it('preserves the language attribute', () => {
    const editor = new Editor({
      extensions: [StarterKit.configure({ codeBlock: false }), CodeBlockShiki],
      content: {
        type: 'doc',
        content: [
          {
            type: 'codeBlock',
            attrs: { language: 'ts' },
            content: [{ type: 'text', text: 'const x = 1' }],
          },
        ],
      },
    })
    const block = editor.state.doc.firstChild
    expect(block?.type.name).toBe('codeBlock')
    expect(block?.attrs.language).toBe('ts')
    editor.destroy()
  })
})
