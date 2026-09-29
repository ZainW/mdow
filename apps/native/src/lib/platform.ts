import type { ColorScheme } from './theme'

export const IS_MAC = process.platform === 'darwin'

function spawnQuiet(cmd: string[], stdin?: string) {
  try {
    const proc = Bun.spawn(cmd, {
      stdin: stdin === undefined ? 'ignore' : new Blob([stdin]),
      stdout: 'ignore',
      stderr: 'ignore',
    })
    return proc.exited.then((code) => code === 0).catch(() => false)
  } catch {
    return Promise.resolve(false)
  }
}

/** Open a URL or a file with the OS default handler. */
export function openExternal(target: string) {
  return spawnQuiet(IS_MAC ? ['open', target] : ['xdg-open', target])
}

export function revealInFileManager(path: string) {
  if (IS_MAC) return spawnQuiet(['open', '-R', path])
  return spawnQuiet(['xdg-open', path.replace(/\/[^/]*$/, '') || '/'])
}

/** gpuix has no clipboard write API; Cmd+C only copies the text selection. */
export async function copyToClipboard(text: string) {
  if (IS_MAC) return spawnQuiet(['pbcopy'], text)
  if (process.env.WAYLAND_DISPLAY && (await spawnQuiet(['wl-copy'], text))) return true
  return spawnQuiet(['xclip', '-selection', 'clipboard'], text)
}

/** gpuix exposes no window appearance, so ask the OS. Cheap enough to poll. */
export function systemColorScheme(): ColorScheme {
  try {
    if (IS_MAC) {
      const result = Bun.spawnSync(['defaults', 'read', '-g', 'AppleInterfaceStyle'], {
        stdout: 'pipe',
        stderr: 'ignore',
      })
      return result.stdout.toString().trim() === 'Dark' ? 'dark' : 'light'
    }
    const result = Bun.spawnSync(
      ['gsettings', 'get', 'org.gnome.desktop.interface', 'color-scheme'],
      { stdout: 'pipe', stderr: 'ignore' },
    )
    return result.stdout.toString().includes('dark') ? 'dark' : 'light'
  } catch {
    return 'light'
  }
}
