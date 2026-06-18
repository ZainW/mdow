import { describe, expect, it, beforeEach, afterEach } from 'vitest'
import { join } from 'path'
import { mkdtemp, mkdir, writeFile, rm } from 'fs/promises'
import { tmpdir } from 'os'
import { scanFolder, insertFileNode, removeFileNode } from './folder-service'
import type { TreeNode } from '../shared/types'

describe('scanFolder', () => {
  let tempDir: string

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), 'mdview-test-'))
  })

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true })
  })

  it('returns empty tree for empty directory', async () => {
    const result = await scanFolder(tempDir)
    expect(result.tree).toEqual([])
    expect(result.truncated).toBe(false)
  })

  it('finds markdown files', async () => {
    await writeFile(join(tempDir, 'readme.md'), '# Hello')
    await writeFile(join(tempDir, 'notes.markdown'), 'notes')
    await writeFile(join(tempDir, 'doc.mdx'), 'mdx content')

    const result = await scanFolder(tempDir)
    expect(result.tree).toHaveLength(3)
    expect(result.tree.map((n) => n.name).toSorted()).toEqual([
      'doc.mdx',
      'notes.markdown',
      'readme.md',
    ])
  })

  it('finds html documents', async () => {
    await writeFile(join(tempDir, 'preview.html'), '<h1>Preview</h1>')
    await writeFile(join(tempDir, 'legacy.HTM'), '<p>Legacy</p>')

    const result = await scanFolder(tempDir)
    expect(result.tree.map((n) => n.name).toSorted()).toEqual(['legacy.HTM', 'preview.html'])
  })

  it('ignores unsupported document files', async () => {
    await writeFile(join(tempDir, 'readme.md'), '# Hello')
    await writeFile(join(tempDir, 'script.ts'), 'code')
    await writeFile(join(tempDir, 'style.css'), 'css')
    await writeFile(join(tempDir, 'data.json'), '{}')

    const result = await scanFolder(tempDir)
    expect(result.tree).toHaveLength(1)
    expect(result.tree[0].name).toBe('readme.md')
  })

  it('ignores hidden files and directories', async () => {
    await writeFile(join(tempDir, '.hidden.md'), 'hidden')
    await mkdir(join(tempDir, '.git'))
    await writeFile(join(tempDir, '.git', 'config.md'), 'git config')
    await writeFile(join(tempDir, 'visible.md'), 'visible')

    const result = await scanFolder(tempDir)
    expect(result.tree).toHaveLength(1)
    expect(result.tree[0].name).toBe('visible.md')
  })

  it('skips well-known ignored directories like node_modules', async () => {
    await mkdir(join(tempDir, 'node_modules', 'pkg'), { recursive: true })
    await writeFile(join(tempDir, 'node_modules', 'pkg', 'readme.md'), 'pkg readme')
    await mkdir(join(tempDir, 'dist'))
    await writeFile(join(tempDir, 'dist', 'out.md'), 'built')
    await writeFile(join(tempDir, 'visible.md'), 'visible')

    const result = await scanFolder(tempDir)
    expect(result.tree).toHaveLength(1)
    expect(result.tree[0].name).toBe('visible.md')
  })

  it('scans subdirectories recursively', async () => {
    await mkdir(join(tempDir, 'docs'))
    await writeFile(join(tempDir, 'docs', 'guide.md'), 'guide')
    await writeFile(join(tempDir, 'readme.md'), 'readme')

    const result = await scanFolder(tempDir)
    expect(result.tree).toHaveLength(2)

    const dir = result.tree[0]
    expect(dir.name).toBe('docs')
    expect(dir.isDirectory).toBe(true)
    expect(dir.children).toHaveLength(1)
    expect(dir.children![0].name).toBe('guide.md')

    expect(result.tree[1].name).toBe('readme.md')
    expect(result.tree[1].isDirectory).toBe(false)
  })

  it('sorts directories before files', async () => {
    await writeFile(join(tempDir, 'zebra.md'), 'z')
    await mkdir(join(tempDir, 'alpha'))
    await writeFile(join(tempDir, 'alpha', 'file.md'), 'a')

    const result = await scanFolder(tempDir)
    expect(result.tree[0].isDirectory).toBe(true)
    expect(result.tree[0].name).toBe('alpha')
    expect(result.tree[1].isDirectory).toBe(false)
    expect(result.tree[1].name).toBe('zebra.md')
  })

  it('sorts alphabetically within same type', async () => {
    await writeFile(join(tempDir, 'charlie.md'), 'c')
    await writeFile(join(tempDir, 'alpha.md'), 'a')
    await writeFile(join(tempDir, 'bravo.md'), 'b')

    const result = await scanFolder(tempDir)
    expect(result.tree.map((n) => n.name)).toEqual(['alpha.md', 'bravo.md', 'charlie.md'])
  })

  it('excludes empty directories (no supported documents inside)', async () => {
    await mkdir(join(tempDir, 'empty'))
    await mkdir(join(tempDir, 'has-code'))
    await writeFile(join(tempDir, 'has-code', 'script.ts'), 'code')

    const result = await scanFolder(tempDir)
    expect(result.tree).toHaveLength(0)
  })

  it('includes directories that have nested markdown', async () => {
    await mkdir(join(tempDir, 'outer'))
    await mkdir(join(tempDir, 'outer', 'inner'))
    await writeFile(join(tempDir, 'outer', 'inner', 'deep.md'), 'deep')

    const result = await scanFolder(tempDir)
    expect(result.tree).toHaveLength(1)
    expect(result.tree[0].name).toBe('outer')
    expect(result.tree[0].children![0].name).toBe('inner')
    expect(result.tree[0].children![0].children![0].name).toBe('deep.md')
  })

  it('sets correct paths on nodes', async () => {
    await writeFile(join(tempDir, 'file.md'), 'content')

    const result = await scanFolder(tempDir)
    expect(result.tree[0].path).toBe(join(tempDir, 'file.md'))
  })

  it('handles case-insensitive extensions', async () => {
    await writeFile(join(tempDir, 'upper.MD'), 'upper')
    await writeFile(join(tempDir, 'mixed.Markdown'), 'mixed')

    const result = await scanFolder(tempDir)
    expect(result.tree).toHaveLength(2)
  })

  it('file nodes have no children property', async () => {
    await writeFile(join(tempDir, 'file.md'), 'content')

    const result = await scanFolder(tempDir)
    expect(result.tree[0].children).toBeUndefined()
  })

  it('does not recurse past the max depth (8)', async () => {
    // Build a chain 12 levels deep with a markdown file at the bottom.
    let path = tempDir
    for (let i = 0; i < 12; i++) {
      path = join(path, `level${i}`)
      // oxlint-disable-next-line no-await-in-loop -- test fixture setup
      await mkdir(path)
    }
    await writeFile(join(path, 'deep.md'), 'too deep')

    const result = await scanFolder(tempDir)
    // No markdown is reachable within the depth cap, so the chain is pruned out.
    expect(result.tree).toEqual([])
  })

  it('marks the result as truncated and stops once the file cap is hit', async () => {
    // Cap is 5000 — write a hair over so we can observe truncation without
    // making the test painfully slow.
    const total = 5005
    await Promise.all(
      Array.from({ length: total }, (_, i) => writeFile(join(tempDir, `f${i}.md`), '')),
    )

    const result = await scanFolder(tempDir)
    expect(result.truncated).toBe(true)
    expect(result.tree.length).toBeLessThanOrEqual(5000)
    expect(result.tree.length).toBeGreaterThan(0)
  })
})

describe('insertFileNode', () => {
  it('inserts a file at the root level in sorted position', () => {
    const nodes: TreeNode[] = [
      { name: 'a.md', path: '/root/a.md', isDirectory: false },
      { name: 'c.md', path: '/root/c.md', isDirectory: false },
    ]
    expect(insertFileNode(nodes, '/root/b.md', 'b.md', '')).toBe(true)
    expect(nodes).toHaveLength(3)
    expect(nodes[1].name).toBe('b.md')
  })

  it('inserts a file into a nested directory', () => {
    const nodes: TreeNode[] = [
      {
        name: 'docs',
        path: '/root/docs',
        isDirectory: true,
        children: [{ name: 'a.md', path: '/root/docs/a.md', isDirectory: false }],
      },
    ]
    expect(insertFileNode(nodes, '/root/docs/b.md', 'b.md', '/root/docs')).toBe(true)
    expect(nodes[0].children).toHaveLength(2)
    expect(nodes[0].children![1].name).toBe('b.md')
  })

  it('does not insert duplicates', () => {
    const nodes: TreeNode[] = [{ name: 'a.md', path: '/root/a.md', isDirectory: false }]
    expect(insertFileNode(nodes, '/root/a.md', 'a.md', '')).toBe(false)
    expect(nodes).toHaveLength(1)
  })

  it('returns false when parent directory does not exist', () => {
    const nodes: TreeNode[] = []
    expect(insertFileNode(nodes, '/missing/b.md', 'b.md', '/missing')).toBe(false)
    expect(nodes).toHaveLength(0)
  })

  it('appends file after directories', () => {
    const nodes: TreeNode[] = [
      {
        name: 'subdir',
        path: '/root/subdir',
        isDirectory: true,
        children: [],
      },
    ]
    expect(insertFileNode(nodes, '/root/a.md', 'a.md', '')).toBe(true)
    expect(nodes).toHaveLength(2)
    expect(nodes[0].isDirectory).toBe(true)
    expect(nodes[1].name).toBe('a.md')
  })
})

describe('removeFileNode', () => {
  it('removes a file from the root level', () => {
    const nodes: TreeNode[] = [
      { name: 'a.md', path: '/root/a.md', isDirectory: false },
      { name: 'b.md', path: '/root/b.md', isDirectory: false },
    ]
    expect(removeFileNode(nodes, '/root/a.md')).toBe(true)
    expect(nodes).toHaveLength(1)
    expect(nodes[0].name).toBe('b.md')
  })

  it('removes a file from a nested directory', () => {
    const nodes: TreeNode[] = [
      {
        name: 'docs',
        path: '/root/docs',
        isDirectory: true,
        children: [
          { name: 'a.md', path: '/root/docs/a.md', isDirectory: false },
          { name: 'b.md', path: '/root/docs/b.md', isDirectory: false },
        ],
      },
    ]
    expect(removeFileNode(nodes, '/root/docs/a.md')).toBe(true)
    expect(nodes[0].children).toHaveLength(1)
  })

  it('removes empty parent directories after file removal', () => {
    const nodes: TreeNode[] = [
      {
        name: 'empty-dir',
        path: '/root/empty-dir',
        isDirectory: true,
        children: [{ name: 'only.md', path: '/root/empty-dir/only.md', isDirectory: false }],
      },
    ]
    expect(removeFileNode(nodes, '/root/empty-dir/only.md')).toBe(true)
    expect(nodes).toHaveLength(0)
  })

  it('returns false for non-existent file', () => {
    const nodes: TreeNode[] = [{ name: 'a.md', path: '/root/a.md', isDirectory: false }]
    expect(removeFileNode(nodes, '/root/missing.md')).toBe(false)
    expect(nodes).toHaveLength(1)
  })

  it('cascades empty directory cleanup through multiple levels', () => {
    const nodes: TreeNode[] = [
      {
        name: 'a',
        path: '/root/a',
        isDirectory: true,
        children: [
          {
            name: 'b',
            path: '/root/a/b',
            isDirectory: true,
            children: [
              {
                name: 'c',
                path: '/root/a/b/c',
                isDirectory: true,
                children: [{ name: 'only.md', path: '/root/a/b/c/only.md', isDirectory: false }],
              },
            ],
          },
        ],
      },
    ]
    expect(removeFileNode(nodes, '/root/a/b/c/only.md')).toBe(true)
    expect(nodes).toHaveLength(0)
  })
})
