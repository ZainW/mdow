import { describe, expect, test } from 'bun:test'
import { uppercaseLabel } from './Reader'

describe('uppercaseLabel', () => {
  test('uppercases words but keeps code, links, URLs and entities as written', () => {
    expect(uppercaseLabel('see `api` and [the guide](https://example.com/api/Guide)')).toBe(
      'SEE `api` AND [THE GUIDE](https://example.com/api/Guide)',
    )
    expect(uppercaseLabel('home <https://example.com/Path> page')).toBe(
      'HOME <https://example.com/Path> PAGE',
    )
    expect(uppercaseLabel('docs at https://example.com/Docs now')).toBe(
      'DOCS AT https://example.com/Docs NOW',
    )
    expect(uppercaseLabel('a&nbsp;b &amp; hash')).toBe('A&nbsp;B &amp; HASH')
  })
})
