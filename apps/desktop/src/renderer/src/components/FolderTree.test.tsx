import { fireEvent, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderWithProviders } from '../test/renderWithProviders'
import { useAppStore } from '../store/app-store'
import { FolderTree } from './FolderTree'

const resetPathsMock = vi.hoisted(() => vi.fn())

vi.mock('../hooks/useOpenMarkdownFile', () => ({
  useOpenMarkdownFile: () => vi.fn(),
}))

vi.mock('@pierre/trees', () => ({
  FileTree: class MockFileTree {
    paths: string[]

    constructor(options: { paths: string[] }) {
      this.paths = options.paths
    }

    resetPaths(paths: string[]) {
      this.paths = paths
      resetPathsMock(paths)
    }

    getItem(path: string) {
      return {
        isDirectory: () => path.endsWith('/'),
        isSelected: () => false,
        select: vi.fn(),
        expand: vi.fn(),
      }
    }

    cleanUp() {}
  },
}))

vi.mock('@pierre/trees/react', () => ({
  FileTree: ({ model }: { model: { paths: string[] } }) => (
    <div data-testid="folder-tree">{model.paths.join('|')}</div>
  ),
}))

describe('FolderTree', () => {
  beforeEach(() => {
    resetPathsMock.mockReset()
    useAppStore.setState({
      openFolderPath: '/docs',
      folderTree: [
        { name: 'guide.md', path: '/docs/guide.md', isDirectory: false },
        { name: 'notes.md', path: '/docs/notes.md', isDirectory: false },
      ],
      folderTreeTruncated: false,
      openingPath: null,
      tabs: [],
      activeTabId: null,
    })
  })

  it('filters visible folder paths by file name', async () => {
    renderWithProviders(<FolderTree />)

    fireEvent.change(screen.getByPlaceholderText('Filter folder...'), {
      target: { value: 'guide' },
    })

    await waitFor(() => {
      expect(resetPathsMock).toHaveBeenLastCalledWith(['guide.md'])
    })
    expect(screen.getByText('1 match')).toBeInTheDocument()
  })
})
