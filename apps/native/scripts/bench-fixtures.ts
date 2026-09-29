/**
 * Write the synthetic documents `scripts/bench.tsx` is measured against. Each one stresses a
 * different way a reader row can get expensive.
 *
 * Usage: bun scripts/bench-fixtures.ts [out-dir]   (default: tmp/bench)
 */
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

const out = resolve(process.argv[2] ?? 'tmp/bench')
mkdirSync(out, { recursive: true })
const section = readFileSync(join(import.meta.dir, '../tests/fixtures/bench-section.md'), 'utf8')
const lines = (count: number, line: (i: number) => string) =>
  Array.from({ length: count }, (_, i) => line(i)).join('\n')

const fixtures: Record<string, string> = {
  // Many ordinary blocks: the virtual list's own overhead.
  'huge.md': Array.from({ length: 1000 }, (_, i) => `# Section ${i + 1}\n\n${section}`).join(
    '\n\n---\n\n',
  ),
  // One list, one table and one fence that each used to be a single row laid out in full.
  'biglist.md': `# Big list\n\n${lines(3000, (i) =>
    [
      `- [${i % 3 ? ' ' : 'x'}] Task number ${i} with a [link](https://example.com/${i}) and *emphasis*`,
      ...(i % 10 === 0 ? ['  - nested child item'] : []),
    ].join('\n'),
  )}\n`,
  'bigtable.md': `# Big table\n\n| id | name | description | status | owner |\n|---|---|---|---|---|\n${lines(
    1500,
    (i) =>
      `| ${i} | item-${i} | Some **longer** description text for row ${i} with \`code\` | ok | @user${i % 17} |`,
  )}\n`,
  'bigcode.md': `# Big code\n\n\`\`\`ts\n${lines(
    6000,
    (i) =>
      `export function fn${i}(a: number, b: string): string { return b.repeat(a) + "${i}" } // comment ${i}`,
  )}\n\`\`\`\n`,
  // Plain long prose.
  'prose.md': `# Long prose\n\n${lines(
    400,
    (i) =>
      `## Section ${i}\n\n${'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. '.repeat(20)}\n`,
  )}`,
}

for (const [name, content] of Object.entries(fixtures)) writeFileSync(join(out, name), content)
console.log(`wrote ${Object.keys(fixtures).length} fixtures to ${out}`)
