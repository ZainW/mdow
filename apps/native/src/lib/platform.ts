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

const APPEARANCE_COMMAND = IS_MAC
  ? ['defaults', 'read', '-g', 'AppleInterfaceStyle']
  : ['gsettings', 'get', 'org.gnome.desktop.interface', 'color-scheme']

function schemeFrom(output: string): ColorScheme {
  return (IS_MAC ? output.trim() === 'Dark' : output.includes('dark')) ? 'dark' : 'light'
}

/**
 * The same question without blocking the UI thread, for polling. The synchronous version spends
 * ~5ms in a subprocess, which is a dropped frame at 120Hz every time it runs.
 */
export async function systemColorSchemeAsync(): Promise<ColorScheme> {
  try {
    const proc = Bun.spawn(APPEARANCE_COMMAND, { stdout: 'pipe', stderr: 'ignore' })
    return schemeFrom(await new Response(proc.stdout).text())
  } catch {
    return 'light'
  }
}

/** gpuix exposes no window appearance, so ask the OS. Used once at startup. */
export function systemColorScheme(): ColorScheme {
  try {
    const result = Bun.spawnSync(APPEARANCE_COMMAND, { stdout: 'pipe', stderr: 'ignore' })
    return schemeFrom(result.stdout.toString())
  } catch {
    return 'light'
  }
}
