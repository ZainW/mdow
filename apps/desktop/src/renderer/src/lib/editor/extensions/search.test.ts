import { describe, it, expect } from 'vitest'
import { Editor } from '@tiptap/core'
import { editorExtensions } from '../schema'
import { Search } from './search'

describe('Search extension', () => {
  it('counts matches', () => {
    const editor = new Editor({
      extensions: [...editorExtensions, Search],
      content: {
        type: 'doc',
        content: [{ type: 'paragraph', content: [{ type: 'text', text: 'hello hello world' }] }],
      },
    })
    editor.commands.setSearchQuery('hello')
    expect(editor.storage.search.matchCount).toBe(2)
    editor.destroy()
  })
})
