export interface ReleaseAsset {
  arch?: 'arm64' | 'x64'
  url: string
}

export interface ReleaseInfo {
  version: string
  publishedAt: string
  htmlUrl: string
  assets: {
    mac: { dmg: ReleaseAsset[]; zip: ReleaseAsset[]; gpuiBeta: ReleaseAsset | null }
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
  draft?: boolean
  published_at: string
  html_url: string
  assets: GhAsset[]
}

const REPO = 'ZainW/mdow'
const GPUI_ALIAS = 'mdownative-mac-beta.zip'
const GPUI_VERSIONED_SUFFIX = '-arm64-mac-beta.zip'
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

function gpuiBetaAssetType(name: string): 'alias' | 'versioned' | null {
  const normalized = name.toLowerCase()
  if (normalized === GPUI_ALIAS) return 'alias'
  if (normalized.startsWith('mdownative-') && normalized.endsWith(GPUI_VERSIONED_SUFFIX)) {
    const version = normalized.slice('mdownative-'.length, -GPUI_VERSIONED_SUFFIX.length)
    if (version.length > 0) return 'versioned'
  }
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
  let gpuiBeta: ReleaseAsset | null = null
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
    } else if (normalized.endsWith('.zip') && normalized.includes('mac')) {
      const gpuiBetaType = gpuiBetaAssetType(asset.name)
      if (gpuiBetaType === 'alias') {
        gpuiBeta = releaseAsset(asset)
      } else if (gpuiBetaType === 'versioned') {
        gpuiBeta ??= releaseAsset(asset)
      } else if (!normalized.startsWith('mdownative')) {
        zip.push(releaseAsset(asset))
      }
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
      mac: { dmg, zip, gpuiBeta },
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

/** Mdow for Mac (the Swift app in apps/macos), released separately from `mac-<version>` tags. */
export interface MacAppRelease {
  version: string
  publishedAt: string
  htmlUrl: string
  url: string
}

/** Always the newest Mdow for Mac zip; the release workflow keeps a copy on `mac-latest`. */
export const MAC_APP_DOWNLOAD_URL = `https://github.com/${REPO}/releases/download/mac-latest/Mdow-for-Mac.zip`

export function parseMacAppRelease(releases: GhRelease[]): MacAppRelease | null {
  for (const release of releases) {
    const version = /^mac-(\d[\w.-]*)$/.exec(release.tag_name)?.[1]
    if (!version || release.draft) continue
    const zip = release.assets?.find((asset) => asset.name === `Mdow-for-Mac-${version}.zip`)
    if (!zip) continue
    return {
      version,
      publishedAt: release.published_at,
      htmlUrl: release.html_url,
      url: zip.browser_download_url,
    }
  }
  return null
}

export async function fetchLatestMacAppRelease(): Promise<MacAppRelease | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=30`, {
      headers: {
        Accept: 'application/vnd.github+json',
        'User-Agent': 'mdow-website',
      },
    })
    if (!res.ok) return null
    return parseMacAppRelease((await res.json()) as GhRelease[])
  } catch {
    return null
  }
}
