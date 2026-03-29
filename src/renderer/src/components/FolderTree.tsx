import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

function TreeItem({
  node,
  depth,
  activeFilePath,
  onFileClick,
}: {
  node: TreeNode
  depth: number
  activeFilePath: string | null
  onFileClick: (path: string) => void
}) {
  const [expanded, setExpanded] = useState(depth === 0)

  if (node.isDirectory) {
    return (
      <>
        <div
          className={`sidebar-item indent-${Math.min(depth, 3)}`}
          onClick={() => setExpanded(!expanded)}
        >
          <span style={{ fontSize: 10, marginRight: 4 }}>{expanded ? '\u25BE' : '\u25B8'}</span>
          {node.name}/
        </div>
        {expanded &&
          node.children?.map((child) => (
            <TreeItem
              key={child.path}
              node={child}
              depth={depth + 1}
              activeFilePath={activeFilePath}
              onFileClick={onFileClick}
            />
          ))}
      </>
    )
  }

  return (
    <div
      className={`sidebar-item indent-${Math.min(depth, 3)} ${
        activeFilePath === node.path ? 'active' : ''
      }`}
      onClick={() => onFileClick(node.path)}
      title={node.path}
    >
      {node.name}
    </div>
  )
}

export function FolderTree() {
  const folderTree = useAppStore((s) => s.folderTree)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const activeFile = useAppStore((s) => s.activeFile)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const queryClient = useQueryClient()

  const handleFileClick = useCallback(
    async (path: string) => {
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setActiveFile, queryClient]
  )

  if (!openFolderPath || folderTree.length === 0) return null

  const folderName = openFolderPath.split(/[/\\]/).pop() || openFolderPath

  return (
    <>
      <div className="sidebar-divider" />
      <div className="sidebar-section-label" style={{ display: 'flex', justifyContent: 'space-between' }}>
        <span>Folder</span>
        <span style={{ opacity: 0.6, textTransform: 'none', fontWeight: 400 }}>{folderName}</span>
      </div>
      <div className="sidebar-list">
        {folderTree.map((node) => (
          <TreeItem
            key={node.path}
            node={node}
            depth={0}
            activeFilePath={activeFile?.path ?? null}
            onFileClick={handleFileClick}
          />
        ))}
      </div>
    </>
  )
}
