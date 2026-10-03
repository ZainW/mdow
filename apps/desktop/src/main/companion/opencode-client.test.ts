import { describe, expect, it, vi } from 'vitest'
import {
  OpencodeClient,
  OpencodeHttpError,
  createSseParser,
  parseOpencodeEvent,
} from './opencode-client'

describe('createSseParser', () => {
  it('joins data lines and handles events split across chunks', () => {
    const received: string[] = []
    const parse = createSseParser((data) => received.push(data))
    parse('event: message\ndata: {"type":"a",')
    parse('"data":{}}\n\ndata: one\r\ndata: two\n\n')
    expect(received).toEqual(['{"type":"a","data":{}}', 'one\ntwo'])
  })
})

describe('parseOpencodeEvent', () => {
  it('returns typed events and drops malformed payloads', () => {
    expect(
      parseOpencodeEvent('{"id":"evt_1","type":"session.idle","data":{"sessionID":"s"}}'),
    ).toEqual({ id: 'evt_1', type: 'session.idle', data: { sessionID: 's' } })
    expect(parseOpencodeEvent('{"data":{}}')).toBeNull()
    expect(parseOpencodeEvent('not json')).toBeNull()
  })
})

describe('OpencodeClient', () => {
  it('sends basic auth and scopes requests to a folder', async () => {
    const fetchImpl = vi.fn(async () => new Response(JSON.stringify({ data: [] })))
    const client = new OpencodeClient('http://127.0.0.1:9', 'secret', fetchImpl as typeof fetch)
    await client.listModels('/my docs')
    const [url, init] = fetchImpl.mock.calls[0] as unknown as [string, RequestInit]
    expect(url).toBe('http://127.0.0.1:9/api/model?location%5Bdirectory%5D=%2Fmy%20docs')
    expect((init.headers as Record<string, string>).authorization).toBe(
      `Basic ${Buffer.from('opencode:secret').toString('base64')}`,
    )
  })

  it('surfaces the server error message', async () => {
    const fetchImpl = vi.fn(
      async () =>
        new Response(JSON.stringify({ error: { message: 'Unknown model' } }), { status: 400 }),
    )
    const client = new OpencodeClient('http://x', 'p', fetchImpl as typeof fetch)
    await expect(client.prompt('ses_1', 'hi')).rejects.toEqual(
      new OpencodeHttpError(400, 'Unknown model'),
    )
  })

  it('replies to permissions with the decision and an optional message', async () => {
    const fetchImpl = vi.fn(async () => new Response(null, { status: 204 }))
    const client = new OpencodeClient('http://x', 'p', fetchImpl as typeof fetch)
    await client.replyPermission('ses_1', 'per_1', 'reject', 'No thanks')
    const [url, init] = fetchImpl.mock.calls[0] as unknown as [string, RequestInit]
    expect(url).toBe('http://x/api/session/ses_1/permission/per_1/reply')
    expect(JSON.parse(init.body as string)).toEqual({ decision: 'reject', message: 'No thanks' })
  })
})
