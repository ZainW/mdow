// A small client for the OpenCode 2 HTTP API (`opencode serve`). The published SDK still
// targets the pre-2.0 event names, so Mdow speaks to the endpoints it needs directly.

export interface OpencodeModelRef {
  providerID: string
  id: string
}

export interface OpencodeModelInfo {
  id: string
  providerID: string
  name?: string
  status?: string
}

export interface OpencodeProviderInfo {
  id: string
  name?: string
}

export interface OpencodePermissionRule {
  action: string
  resource: string
  effect: 'allow' | 'deny' | 'ask'
}

export interface OpencodeEvent {
  id?: string
  type: string
  data: Record<string, unknown>
}

export type OpencodePermissionReply = 'once' | 'always' | 'reject'

export class OpencodeHttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message)
  }
}

type FetchImpl = typeof fetch

function locationQuery(directory: string): string {
  return `location%5Bdirectory%5D=${encodeURIComponent(directory)}`
}

function errorMessage(status: number, body: string): string {
  try {
    const parsed = JSON.parse(body) as { message?: unknown; error?: { message?: unknown } }
    const message = parsed.error?.message ?? parsed.message
    if (typeof message === 'string' && message) return message
  } catch {
    // Not JSON; fall through to the raw body.
  }
  return body.trim().slice(0, 200) || `OpenCode request failed (${status})`
}

/** Splits an SSE byte stream into `data:` payloads. Exported for tests. */
export function createSseParser(onData: (data: string) => void) {
  let buffer = ''
  let dataLines: string[] = []
  return (chunk: string) => {
    buffer += chunk
    let newline = buffer.indexOf('\n')
    while (newline !== -1) {
      const line = buffer.slice(0, newline).replace(/\r$/, '')
      buffer = buffer.slice(newline + 1)
      if (line === '') {
        if (dataLines.length > 0) onData(dataLines.join('\n'))
        dataLines = []
      } else if (line.startsWith('data:')) {
        dataLines.push(line.slice(5).replace(/^ /, ''))
      }
      newline = buffer.indexOf('\n')
    }
  }
}

export function parseOpencodeEvent(data: string): OpencodeEvent | null {
  try {
    const parsed = JSON.parse(data) as Partial<OpencodeEvent>
    if (typeof parsed.type !== 'string') return null
    const payload =
      typeof parsed.data === 'object' && parsed.data !== null
        ? (parsed.data as Record<string, unknown>)
        : {}
    return { id: parsed.id, type: parsed.type, data: payload }
  } catch {
    return null
  }
}

export class OpencodeClient {
  private readonly authorization: string

  constructor(
    private readonly baseUrl: string,
    password: string,
    private readonly fetchImpl: FetchImpl = fetch,
  ) {
    this.authorization = `Basic ${Buffer.from(`opencode:${password}`).toString('base64')}`
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method,
      headers: {
        authorization: this.authorization,
        ...(body === undefined ? {} : { 'content-type': 'application/json' }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
    const text = await response.text()
    if (!response.ok)
      throw new OpencodeHttpError(response.status, errorMessage(response.status, text))
    return (text ? JSON.parse(text) : undefined) as T
  }

  async listModels(directory: string): Promise<OpencodeModelInfo[]> {
    const result = await this.request<{ data: OpencodeModelInfo[] }>(
      'GET',
      `/api/model?${locationQuery(directory)}`,
    )
    return result.data
  }

  async listProviders(directory: string): Promise<OpencodeProviderInfo[]> {
    const result = await this.request<{ data: OpencodeProviderInfo[] }>(
      'GET',
      `/api/provider?${locationQuery(directory)}`,
    )
    return result.data
  }

  async defaultModel(directory: string): Promise<OpencodeModelRef | null> {
    const result = await this.request<{ data: OpencodeModelRef | null }>(
      'GET',
      `/api/model/default?${locationQuery(directory)}`,
    )
    return result.data
  }

  async createSession(input: {
    directory: string
    model: OpencodeModelRef | null
    permissions: OpencodePermissionRule[]
  }): Promise<{ id: string }> {
    const result = await this.request<{ data: { id: string } }>('POST', '/api/session', {
      location: { directory: input.directory },
      ...(input.model ? { model: input.model } : {}),
      permissions: input.permissions,
    })
    return result.data
  }

  async setInstructions(sessionId: string, key: string, value: string): Promise<void> {
    await this.request(
      'PUT',
      `/api/experimental/session/${sessionId}/instructions/entries/${encodeURIComponent(key)}`,
      { value },
    )
  }

  async switchModel(sessionId: string, model: OpencodeModelRef): Promise<void> {
    await this.request('POST', `/api/session/${sessionId}/model`, { model })
  }

  async prompt(sessionId: string, text: string): Promise<void> {
    await this.request('POST', `/api/session/${sessionId}/prompt`, { text })
  }

  async interrupt(sessionId: string): Promise<void> {
    await this.request('POST', `/api/session/${sessionId}/interrupt`)
  }

  async replyPermission(
    sessionId: string,
    requestId: string,
    decision: OpencodePermissionReply,
    message?: string,
  ): Promise<void> {
    await this.request('POST', `/api/session/${sessionId}/permission/${requestId}/reply`, {
      decision,
      ...(message ? { message } : {}),
    })
  }

  /**
   * Streams server events for a folder until `signal` aborts, reconnecting with backoff when
   * the connection drops.
   */
  // Reconnects are sequential by design: each attempt waits for the previous stream to end.
  /* oxlint-disable eslint/no-await-in-loop */
  async subscribe(
    directory: string,
    onEvent: (event: OpencodeEvent) => void,
    signal: AbortSignal,
  ): Promise<void> {
    let attempt = 0
    while (!signal.aborted) {
      try {
        const response = await this.fetchImpl(
          `${this.baseUrl}/api/event?${locationQuery(directory)}`,
          {
            headers: { authorization: this.authorization, accept: 'text/event-stream' },
            signal,
          },
        )
        if (!response.ok || !response.body) {
          throw new OpencodeHttpError(response.status, 'OpenCode event stream unavailable')
        }
        attempt = 0
        const parse = createSseParser((data) => {
          const event = parseOpencodeEvent(data)
          if (event) onEvent(event)
        })
        const decoder = new TextDecoder()
        for await (const chunk of response.body as unknown as AsyncIterable<Uint8Array>) {
          parse(decoder.decode(chunk, { stream: true }))
        }
      } catch (error) {
        if (signal.aborted) return
        if (error instanceof OpencodeHttpError && error.status === 401) throw error
      }
      attempt += 1
      const delay = Math.min(250 * 2 ** attempt, 5_000)
      await new Promise((resolve) => setTimeout(resolve, delay))
    }
  }
  /* oxlint-enable eslint/no-await-in-loop */
}
