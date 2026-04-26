import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import type { Node as ProseMirrorNode } from '@tiptap/pm/model'

const searchKey = new PluginKey<SearchPluginState>('search')

interface Match {
  from: number
  to: number
}

interface SearchPluginState {
  matches: Match[]
  currentIndex: number
  decos: DecorationSet
}

interface SearchStorage {
  query: string
  matchCount: number
  currentIndex: number
}

declare module '@tiptap/core' {
  interface Storage {
    search: SearchStorage
  }
  interface Commands<ReturnType> {
    search: {
      setSearchQuery: (q: string) => ReturnType
      nextMatch: () => ReturnType
      prevMatch: () => ReturnType
    }
  }
}

function findMatches(doc: ProseMirrorNode, query: string): Match[] {
  if (!query) return []
  const matches: Match[] = []
  const lower = query.toLowerCase()
  doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return
    const text = node.text.toLowerCase()
    let idx = 0
    while ((idx = text.indexOf(lower, idx)) !== -1) {
      matches.push({ from: pos + idx, to: pos + idx + query.length })
      idx += query.length
    }
  })
  return matches
}

export const Search = Extension.create({
  name: 'search',
  addStorage() {
    return { query: '', matchCount: 0, currentIndex: 0 } satisfies SearchStorage
  },
  addCommands() {
    return {
      setSearchQuery:
        (q: string) =>
        ({ editor, tr, dispatch }) => {
          editor.storage.search.query = q
          const matches = findMatches(editor.state.doc, q)
          editor.storage.search.matchCount = matches.length
          editor.storage.search.currentIndex = matches.length > 0 ? 0 : -1
          if (dispatch)
            dispatch(
              tr.setMeta(searchKey, {
                matches,
                currentIndex: matches.length > 0 ? 0 : -1,
              }),
            )
          return true
        },
      nextMatch:
        () =>
        ({ editor, tr, dispatch }) => {
          const count = editor.storage.search.matchCount
          if (count === 0) return false
          editor.storage.search.currentIndex = (editor.storage.search.currentIndex + 1) % count
          if (dispatch)
            dispatch(tr.setMeta(searchKey, { currentIndex: editor.storage.search.currentIndex }))
          return true
        },
      prevMatch:
        () =>
        ({ editor, tr, dispatch }) => {
          const count = editor.storage.search.matchCount
          if (count === 0) return false
          editor.storage.search.currentIndex =
            (editor.storage.search.currentIndex - 1 + count) % count
          if (dispatch)
            dispatch(tr.setMeta(searchKey, { currentIndex: editor.storage.search.currentIndex }))
          return true
        },
    }
  },
  addProseMirrorPlugins() {
    // Use a shared mutable ref so the plugin's apply() can read/write storage
    // without aliasing `this` (which triggers no-this-alias lint rule).
    const storageRef: SearchStorage = this.storage as SearchStorage
    return [
      new Plugin<SearchPluginState>({
        key: searchKey,
        state: {
          init: () => ({ matches: [], currentIndex: -1, decos: DecorationSet.empty }),
          apply(tr, value, _oldState, newState) {
            const meta = tr.getMeta(searchKey) as Partial<SearchPluginState> | undefined
            let matches = value.matches
            let currentIndex = value.currentIndex
            if (meta) {
              if (meta.matches) matches = meta.matches
              if (typeof meta.currentIndex === 'number') currentIndex = meta.currentIndex
            } else if (tr.docChanged && storageRef.query) {
              matches = findMatches(newState.doc, storageRef.query)
              currentIndex = matches.length > 0 ? 0 : -1
              storageRef.matchCount = matches.length
              storageRef.currentIndex = currentIndex
            }
            const decos = matches.map((m, i) =>
              Decoration.inline(m.from, m.to, {
                class: i === currentIndex ? 'search-match search-match-current' : 'search-match',
              }),
            )
            return { matches, currentIndex, decos: DecorationSet.create(newState.doc, decos) }
          },
        },
        props: {
          decorations(state) {
            return this.getState(state)?.decos
          },
        },
      }),
    ]
  },
})
