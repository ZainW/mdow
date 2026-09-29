/** Imperative reader commands from keys, menus, the outline and links. */
export type ReaderCommand =
  | { type: 'scroll'; to: 'top' | 'bottom' }
  | { type: 'page'; direction: 1 | -1 }
  | { type: 'line'; direction: 1 | -1 }
  | { type: 'block'; index: number }
  | { type: 'slug'; slug: string }
  | { type: 'find'; direction: 1 | -1 }

type Listener = (command: ReaderCommand) => void

const listeners = new Set<Listener>()

export function sendReader(command: ReaderCommand) {
  for (const listener of listeners) listener(command)
}

export function onReaderCommand(listener: Listener) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}
