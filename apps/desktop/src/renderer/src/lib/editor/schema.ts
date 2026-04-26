// oxlint-disable-next-line eslint-plugin-import/no-named-as-default -- StarterKit is the canonical default export for @tiptap/starter-kit
import StarterKit from '@tiptap/starter-kit'
import { CodeBlockShiki } from './extensions/code-block-shiki'
import { MermaidBlock } from './extensions/mermaid-block'
import { Frontmatter } from './extensions/frontmatter'
import { HtmlPassthrough } from './extensions/html-passthrough'

export const editorExtensions = [
  StarterKit.configure({
    codeBlock: false,
  }),
  CodeBlockShiki,
  MermaidBlock,
  Frontmatter,
  HtmlPassthrough,
]
