import type { FileTreeIconConfig } from '@pierre/trees'
import { iconSize } from './icons'

const size = iconSize.md
const viewBox = '0 0 24 24'

const lucideIcon = (name: string) => ({ name, viewBox, width: size, height: size })

/** Lucide stroke icons for @pierre/trees — matches sidebar / tab icon language. */
export const fileTreeIcons = {
  set: 'minimal',
  colored: false,
  spriteSheet: `<svg data-icon-sprite aria-hidden="true" width="0" height="0">
  <symbol id="mdow-chevron" viewBox="0 0 24 24">
    <path d="m6 9 6 6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
  </symbol>
  <symbol id="mdow-file" viewBox="0 0 24 24">
    <path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M14 2v5a1 1 0 0 0 1 1h5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
  </symbol>
  <symbol id="mdow-file-text" viewBox="0 0 24 24">
    <path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M14 2v5a1 1 0 0 0 1 1h5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M10 9H8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M16 13H8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M16 17H8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
  </symbol>
</svg>`,
  remap: {
    'file-tree-icon-chevron': lucideIcon('mdow-chevron'),
    'file-tree-icon-file': lucideIcon('mdow-file'),
  },
  byFileExtension: {
    md: lucideIcon('mdow-file-text'),
    markdown: lucideIcon('mdow-file-text'),
    mdx: lucideIcon('mdow-file-text'),
  },
} satisfies FileTreeIconConfig
