import { useMemo } from 'react'
import type { TreeNode } from '../../../../shared/types'
import { basename, resolveRelativePath } from '../../lib/path-utils'
import { useAppStore } from '../../store/app-store'
import type { DocumentResolver } from '../ai-elements/markdown'

export function flattenDocuments(nodes: TreeNode[]): { path: string; name: string }[] {
  const output: { path: string; name: string }[] = []
  const walk = (list: TreeNode[]) => {
    for (const node of list) {
      if (node.isDirectory && node.children) walk(node.children)
      else if (!node.isDirectory) output.push({ path: node.path, name: node.name })
    }
  }
  walk(nodes)
  return output
}

/**
 * Resolves a path the model mentioned to a document in the open folder: absolute paths,
 * paths relative to the folder or the viewed document, and unambiguous bare file names.
 */
export function createDocumentResolver(
  documents: string[],
  folder: string | null,
  activePath: string | null,
): DocumentResolver {
  const known = new Set(documents)
  const byName = new Map<string, string[]>()
  for (const path of documents) {
    const name = basename(path)
    byName.set(name, [...(byName.get(name) ?? []), path])
  }

  return (reference) => {
    if (known.has(reference)) return reference
    const bases = [folder ? `${folder}/.` : null, activePath].filter(
      (base): base is string => base !== null,
    )
    for (const base of bases) {
      const candidate = resolveRelativePath(base, reference)
      if (known.has(candidate)) return candidate
    }
    if (activePath && basename(activePath) === reference) return activePath
    const matches = byName.get(basename(reference))
    return matches?.length === 1 ? matches[0] : null
  }
}

export function useDocumentResolver(): DocumentResolver {
  const folderTree = useAppStore((state) => state.folderTree)
  const folder = useAppStore((state) => state.openFolderPath)
  const activePath = useAppStore((state) => {
    const tab = state.tabs.find((candidate) => candidate.id === state.activeTabId)
    return tab?.path ?? null
  })
  return useMemo(
    () =>
      createDocumentResolver(
        flattenDocuments(folderTree).map((doc) => doc.path),
        folder,
        activePath,
      ),
    [folderTree, folder, activePath],
  )
}
