export interface ReleaseAsset {
  arch?: 'arm64' | 'x64'
  url: string
}

export interface ReleaseInfo {
  version: string
  publishedAt: string
  htmlUrl: string
  assets: {
    /** `app` is the Swift app; `dmg` and `zip` are the Electron builds older releases carry. */
    mac: { dmg: ReleaseAsset[]; zip: ReleaseAsset[]; app: ReleaseAsset | null }
    windows: { exe: string | null }
    linux: { appImage: string | null; gpuiBeta: ReleaseAsset | null }
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
/** The Mac app: `Mdow-<version>.zip`, and the same under a fixed name. */
const MAC_APP_ALIAS = 'mdow.zip'
const MAC_APP_VERSIONED = /^mdow-\d+\.\d+\.\d+(-[0-9a-z.]+)?\.zip$/
const GPUI_LINUX_ALIAS = 'mdownative-linux-beta.appimage'
/** The Native updater's asset: `mdow-native_<version>_x86_64.AppImage`. */
const GPUI_LINUX_VERSIONED = /^mdow-native_.+_x86_64\.appimage$/

function detectArch(name: string): 'arm64' | 'x64' | undefined {
  if (name.includes('arm64')) return 'arm64'
  if (name.includes('x64')) return 'x64'
  return undefined
}

function releaseAsset(asset: GhAsset): ReleaseAsset {
  const arch = detectArch(asset.name)
  return arch ? { arch, url: asset.browser_download_url } : { url: asset.browser_download_url }
}

function macAppAssetType(name: string): 'alias' | 'versioned' | null {
  const normalized = name.toLowerCase()
  if (normalized === MAC_APP_ALIAS) return 'alias'
  if (MAC_APP_VERSIONED.test(normalized)) return 'versioned'
  return null
}

function gpuiLinuxBetaAssetType(name: string): 'alias' | 'versioned' | null {
  const normalized = name.toLowerCase()
  if (normalized === GPUI_LINUX_ALIAS) return 'alias'
  if (GPUI_LINUX_VERSIONED.test(normalized)) return 'versioned'
  return null
}

export function parseRelease(release: GhRelease): ReleaseInfo | null {
  if (!release?.assets?.length) return null

  const dmg: ReleaseAsset[] = []
  const zip: ReleaseAsset[] = []
  let macApp: ReleaseAsset | null = null
  let linuxGpuiBeta: ReleaseAsset | null = null
  let exe: string | null = null
  let appImage: string | null = null

  for (const asset of release.assets) {
    const normalized = asset.name.toLowerCase()

    if (normalized.endsWith('.dmg')) {
      dmg.push(releaseAsset(asset))
    } else if (normalized.endsWith('.appimage') && gpuiLinuxBetaAssetType(asset.name)) {
      if (gpuiLinuxBetaAssetType(asset.name) === 'alias') {
        linuxGpuiBeta = releaseAsset(asset)
      } else {
        linuxGpuiBeta ??= releaseAsset(asset)
      }
    } else if (normalized.endsWith('.zip') && macAppAssetType(asset.name)) {
      // The versioned name is the one the release notes and update feed point at.
      if (macAppAssetType(asset.name) === 'versioned') {
        macApp = { url: asset.browser_download_url }
      } else {
        macApp ??= { url: asset.browser_download_url }
      }
    } else if (normalized.endsWith('.zip') && normalized.includes('mac')) {
      // Older releases also carried Mdow Native's Mac beta, which isn't offered anymore.
      if (!normalized.startsWith('mdownative')) zip.push(releaseAsset(asset))
    } else if (normalized.endsWith('.exe')) {
      exe ??= asset.browser_download_url
    } else if (normalized.endsWith('.appimage')) {
      appImage ??= asset.browser_download_url
    }
  }

  return {
    version: release.tag_name.replace(/^v/, ''),
    publishedAt: release.published_at,
    htmlUrl: release.html_url,
    assets: {
      mac: { dmg, zip, app: macApp },
      windows: { exe },
      linux: { appImage, gpuiBeta: linuxGpuiBeta },
    },
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
