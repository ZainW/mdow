export interface ReleaseInfo {
  version: string
  publishedAt: string
  htmlUrl: string
  assets: {
    /** The Mac app's zip. */
    mac: string | null
    /** Mdow for Linux, the same app: `Mdow-<version>-x86_64.AppImage`. */
    appImage: string | null
    /** And as a Flatpak bundle: `Mdow-<version>-x86_64.flatpak`. */
    flatpak: string | null
  }
}

interface GhAsset {
  name: string
  browser_download_url: string
}

interface GhRelease {
  tag_name: string
  name?: string
  published_at: string
  html_url: string
  assets: GhAsset[]
}

const REPO = 'ZainW/mdow'
/** The Mac app, built on Rune like Mdow for Linux: `Mdow-Rune-<version>.zip`. */
const MAC_RUNE = /^mdow-rune-\d+\.\d+\.\d+(-[0-9a-z.]+)?\.zip$/
const LINUX_APPIMAGE = /^mdow-\d+\.\d+\.\d+(-[0-9a-z.]+)?-x86_64\.appimage$/
const LINUX_FLATPAK = /^mdow-\d+\.\d+\.\d+(-[0-9a-z.]+)?-x86_64\.flatpak$/

export function parseRelease(release: GhRelease): ReleaseInfo | null {
  if (!release?.assets?.length) return null

  let mac: string | null = null
  let appImage: string | null = null
  let flatpak: string | null = null
  for (const asset of release.assets) {
    const normalized = asset.name.toLowerCase()
    if (MAC_RUNE.test(normalized)) mac = asset.browser_download_url
    else if (LINUX_APPIMAGE.test(normalized)) appImage = asset.browser_download_url
    else if (LINUX_FLATPAK.test(normalized)) flatpak = asset.browser_download_url
  }
  return {
    version: release.tag_name.replace(/^v/, ''),
    publishedAt: release.published_at,
    htmlUrl: release.html_url,
    assets: { mac, appImage, flatpak },
  }
}

export async function fetchLatestRelease(): Promise<ReleaseInfo | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: {
        Accept: 'application/vnd.github+json',
        'User-Agent': 'mdow-website',
      },
    })
    if (!res.ok) return null
    const json = (await res.json()) as GhRelease
    return parseRelease(json)
  } catch {
    return null
  }
}
