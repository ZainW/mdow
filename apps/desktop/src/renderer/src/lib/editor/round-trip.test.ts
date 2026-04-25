import { describe, it, expect } from 'vitest'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { parseMarkdown } from './parser'
import { serializeMarkdown } from './serializer'

const FIXTURES_DIR = join(__dirname, '__fixtures__')

describe('round-trip', () => {
  const fixtures = readdirSync(FIXTURES_DIR).filter((f) => f.endsWith('.md'))

  for (const name of fixtures) {
    it(`round-trips ${name}`, () => {
      const input = readFileSync(join(FIXTURES_DIR, name), 'utf8')
      const doc = parseMarkdown(input)
      const output = serializeMarkdown(doc)
      expect(output.trim()).toBe(input.trim())
    })
  }
})
