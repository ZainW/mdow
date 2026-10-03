import { describe, expect, it } from 'vitest'
import type { OpencodeEvent } from './opencode-client'
import { OpencodeTurnTranslator, fileChangesFrom, toolTitle } from './opencode-events'

const SESSION = 'ses_test'
const DIR = '/docs'

function event(type: string, data: Record<string, unknown>): OpencodeEvent {
  return { type, data: { sessionID: SESSION, ...data } }
}

const PATCH = '--- notes.md\n+++ notes.md\n@@ -1 +1 @@\n-teh\n+the\n'

// Shapes recorded from `opencode serve` 2.0.16.
describe('OpencodeTurnTranslator', () => {
  it('ignores events from other sessions', () => {
    const translator = new OpencodeTurnTranslator(SESSION, DIR)
    const result = translator.translate({
      type: 'session.text.delta',
      data: { sessionID: 'ses_other', delta: 'hi' },
    })
    expect(result.updates).toEqual([])
  })

  it('streams reasoning, then closes it when text starts', () => {
    const translator = new OpencodeTurnTranslator(SESSION, DIR)
    expect(
      translator.translate(event('session.reasoning.delta', { delta: 'Hmm' })).updates,
    ).toEqual([{ kind: 'thinking', text: 'Hmm' }])
    expect(translator.translate(event('session.text.delta', { delta: 'Fixed.' })).updates).toEqual([
      { kind: 'thinking-done' },
      { kind: 'delta', text: 'Fixed.' },
    ])
  })

  it('shows reads as titled tool rows and cites the document', () => {
    const translator = new OpencodeTurnTranslator(SESSION, DIR)
    translator.translate(event('session.tool.input.started', { id: 'call_1', name: 'read' }))
    const called = translator.translate(
      event('session.tool.called', { id: 'call_1', input: { path: '/docs/notes.md' } }),
    )
    expect(called.updates[0]).toMatchObject({
      kind: 'tool',
      toolCallId: 'call_1',
      name: 'Read notes.md',
      state: 'running',
    })
    expect(called.updates[1]).toEqual({
      kind: 'citation',
      citation: { sourceId: 'read:/docs/notes.md', path: '/docs/notes.md', label: 'notes.md' },
    })

    const done = translator.translate(
      event('session.tool.success', {
        id: 'call_1',
        content: [{ type: 'text', text: '1: # Notes' }],
      }),
    )
    expect(done.updates).toEqual([
      {
        kind: 'tool',
        toolCallId: 'call_1',
        name: 'Read notes.md',
        state: 'completed',
        output: '1: # Notes',
      },
    ])
  })

  it('turns an edit permission into a pending change card, then applies it', () => {
    const translator = new OpencodeTurnTranslator(SESSION, DIR)
    expect(
      translator.translate(event('session.tool.input.started', { id: 'call_2', name: 'edit' }))
        .updates,
    ).toEqual([])

    const asked = translator.translate(
      event('permission.asked', {
        id: 'per_1',
        action: 'edit',
        resources: ['notes.md'],
        metadata: {
          files: [
            { file: 'notes.md', patch: PATCH, status: 'modified', additions: 1, deletions: 1 },
          ],
        },
        source: { type: 'tool', id: 'call_2' },
      }),
    )
    expect(asked.updates).toEqual([
      {
        kind: 'change',
        toolCallId: 'call_2',
        permissionId: 'per_1',
        status: 'pending',
        files: [
          {
            path: '/docs/notes.md',
            displayPath: 'notes.md',
            patch: PATCH,
            additions: 1,
            deletions: 1,
            status: 'modified',
          },
        ],
      },
    ])

    const applied = translator.translate(
      event('session.tool.success', {
        id: 'call_2',
        content: [{ type: 'text', text: 'Edited notes.md' }],
        metadata: { files: [{ file: 'notes.md', patch: PATCH, additions: 1, deletions: 1 }] },
      }),
    )
    expect(applied.updates[0]).toMatchObject({
      kind: 'change',
      toolCallId: 'call_2',
      status: 'applied',
    })
  })

  it('marks a rejected edit as rejected rather than failed', () => {
    const translator = new OpencodeTurnTranslator(SESSION, DIR)
    translator.translate(event('session.tool.input.started', { id: 'call_3', name: 'edit' }))
    translator.translate(
      event('permission.asked', { id: 'per_2', action: 'edit', source: { id: 'call_3' } }),
    )
    expect(
      translator.translate({
        type: 'permission.replied',
        data: { sessionID: SESSION, requestID: 'per_2', reply: 'reject' },
      }).updates,
    ).toEqual([{ kind: 'change', toolCallId: 'call_3', status: 'rejected' }])
    expect(
      translator.translate(
        event('session.tool.failed', { id: 'call_3', error: { message: 'Rejected' } }),
      ).updates,
    ).toEqual([{ kind: 'change', toolCallId: 'call_3', status: 'rejected' }])
  })

  it('declines shell permissions automatically', () => {
    const translator = new OpencodeTurnTranslator(SESSION, DIR)
    const result = translator.translate(
      event('permission.asked', {
        id: 'per_3',
        action: 'shell',
        resources: ['ls'],
        source: { id: 'call_4' },
      }),
    )
    expect(result.updates).toEqual([])
    expect(result.autoReject?.permissionId).toBe('per_3')
    expect(result.autoReject?.reason).toContain('shell')
  })

  it('reports how the turn ended', () => {
    const translator = new OpencodeTurnTranslator(SESSION, DIR)
    expect(translator.translate(event('session.execution.succeeded', {})).finish).toEqual({
      outcome: 'succeeded',
    })
    expect(
      translator.translate(
        event('session.execution.failed', {
          error: { type: 'provider.auth', message: 'Sign in again' },
        }),
      ).finish,
    ).toEqual({ outcome: 'failed', error: 'Sign in again' })
    expect(translator.translate(event('session.execution.interrupted', {})).finish).toEqual({
      outcome: 'interrupted',
    })
    expect(translator.translate(event('session.execution.started', {})).finish).toBeUndefined()
  })
})

describe('toolTitle', () => {
  it('describes common tools in plain words', () => {
    expect(toolTitle('grep', { pattern: 'roadmap' })).toBe('Searched for “roadmap”')
    expect(toolTitle('glob', { pattern: '**/*.md' })).toBe('Looked through the folder')
    expect(toolTitle('mystery')).toBe('Mystery')
  })
})

describe('fileChangesFrom', () => {
  it('resolves relative paths against the session folder and skips malformed entries', () => {
    expect(
      fileChangesFrom({ files: [{ file: 'a.md', status: 'added' }, { nope: true }, 'x'] }, DIR),
    ).toEqual([
      {
        path: '/docs/a.md',
        displayPath: 'a.md',
        patch: '',
        additions: 0,
        deletions: 0,
        status: 'added',
      },
    ])
    expect(fileChangesFrom(undefined, DIR)).toEqual([])
  })
})
