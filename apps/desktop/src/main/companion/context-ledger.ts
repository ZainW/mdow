import { createHash } from 'node:crypto'

export interface ContextLedgerRecord {
  hash: string
  alreadySent: boolean
}

function hashContent(content: string): string {
  return createHash('sha256').update(content, 'utf8').digest('hex')
}

export class ContextLedger {
  private readonly entries = new Map<string, string>()

  record(path: string, content: string): ContextLedgerRecord {
    const hash = hashContent(content)
    const alreadySent = this.entries.get(path) === hash
    this.entries.set(path, hash)
    return { hash, alreadySent }
  }

  has(path: string, hash: string): boolean {
    return this.entries.get(path) === hash
  }

  clear(): void {
    this.entries.clear()
  }
}
