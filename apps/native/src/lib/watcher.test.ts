import { describe, expect, test } from 'bun:test'
import { mkdtempSync, realpathSync, renameSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { DEBOUNCE_MS, FileWatcher } from './watcher'

const settle = () => Bun.sleep(DEBOUNCE_MS * 3)

describe('FileWatcher', () => {
  test('coalesces bursts of writes into one reload', async () => {
    const dir = realpathSync(mkdtempSync(join(tmpdir(), 'mdow-watch-')))
    const file = join(dir, 'doc.md')
    writeFileSync(file, 'a')
    const changes: string[] = []
    const watcher = new FileWatcher((path) => changes.push(path))
    watcher.sync([file])
    // FSEvents can replay the fixture's own creation right after the watch starts.
    await Bun.sleep(300)
    changes.length = 0
    for (let i = 0; i < 5; i++) writeFileSync(file, `v${i}`)
    await settle()
    watcher.close()
    expect(changes).toEqual([file])
  })

  test('survives atomic saves that rename over the file', async () => {
    const dir = realpathSync(mkdtempSync(join(tmpdir(), 'mdow-watch-')))
    const file = join(dir, 'doc.md')
    writeFileSync(file, 'a')
    const changes: string[] = []
    const watcher = new FileWatcher((path) => changes.push(path))
    watcher.sync([file])
    // FSEvents can replay the fixture's own creation right after the watch starts.
    await Bun.sleep(300)
    changes.length = 0
    writeFileSync(join(dir, '.doc.md.tmp'), 'b')
    renameSync(join(dir, '.doc.md.tmp'), file)
    await settle()
    writeFileSync(join(dir, '.doc.md.tmp'), 'c')
    renameSync(join(dir, '.doc.md.tmp'), file)
    await settle()
    watcher.close()
    expect(changes).toEqual([file, file])
  })

  test('ignores sibling files that are not open', async () => {
    const dir = realpathSync(mkdtempSync(join(tmpdir(), 'mdow-watch-')))
    const file = join(dir, 'doc.md')
    writeFileSync(file, 'a')
    const changes: string[] = []
    const watcher = new FileWatcher((path) => changes.push(path))
    watcher.sync([file])
    // FSEvents can replay the fixture's own creation right after the watch starts.
    await Bun.sleep(300)
    changes.length = 0
    writeFileSync(join(dir, 'other.md'), 'x')
    await settle()
    watcher.close()
    expect(changes).toEqual([])
  })
})
