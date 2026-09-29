import { describe, expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, realpathSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { scanWorkspace, visibleRows, workspaceFiles } from './workspace'

function fixture() {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'mdow-ws-')))
  const files = [
    'README.md',
    'notes/b.markdown',
    'notes/a10.md',
    'notes/a2.md',
    'site/index.html',
    'notes/image.png',
    '.hidden/x.md',
    'node_modules/pkg/readme.md',
    'build/out.md',
    'empty/nothing.txt',
  ]
  for (const file of files) {
    mkdirSync(join(root, file, '..'), { recursive: true })
    writeFileSync(join(root, file), '# x')
  }
  return root
}

describe('scanWorkspace', () => {
  test('lists supported documents, folders first, with natural sort', () => {
    const root = fixture()
    const scan = scanWorkspace(root)
    if (!scan.ok) throw new Error(scan.error)
    expect(visibleRows(scan.root, new Set()).map((row) => `${row.depth}:${row.name}`)).toEqual([
      '0:notes',
      '1:a2.md',
      '1:a10.md',
      '1:b.markdown',
      '0:site',
      '1:index.html',
      '0:README.md',
    ])
    expect(workspaceFiles(scan.root)).toHaveLength(5)
  })

  test('collapsed folders hide their children', () => {
    const root = fixture()
    const scan = scanWorkspace(root)
    if (!scan.ok) throw new Error(scan.error)
    const rows = visibleRows(scan.root, new Set([join(root, 'notes')]))
    expect(rows.map((row) => row.name)).toEqual(['notes', 'site', 'index.html', 'README.md'])
  })

  test('reports missing folders and files', () => {
    expect(scanWorkspace('/definitely/not/here')).toEqual({ ok: false, error: 'missing' })
    const root = fixture()
    expect(scanWorkspace(join(root, 'README.md'))).toEqual({ ok: false, error: 'not-directory' })
  })
})
