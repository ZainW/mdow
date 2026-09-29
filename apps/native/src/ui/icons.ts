// Embedded as text so `bun build --compile` carries them inside the binary.
import alertCircle from '../../assets/icons/alert-circle.svg' with { type: 'text' }
import check from '../../assets/icons/check.svg' with { type: 'text' }
import chevronDown from '../../assets/icons/chevron-down.svg' with { type: 'text' }
import chevronRight from '../../assets/icons/chevron-right.svg' with { type: 'text' }
import chevronUp from '../../assets/icons/chevron-up.svg' with { type: 'text' }
import clock from '../../assets/icons/clock.svg' with { type: 'text' }
import code from '../../assets/icons/code.svg' with { type: 'text' }
import command from '../../assets/icons/command.svg' with { type: 'text' }
import copy from '../../assets/icons/copy.svg' with { type: 'text' }
import expand from '../../assets/icons/expand.svg' with { type: 'text' }
import externalLink from '../../assets/icons/external-link.svg' with { type: 'text' }
import fileText from '../../assets/icons/file-text.svg' with { type: 'text' }
import file from '../../assets/icons/file.svg' with { type: 'text' }
import folderOpen from '../../assets/icons/folder-open.svg' with { type: 'text' }
import folder from '../../assets/icons/folder.svg' with { type: 'text' }
import image from '../../assets/icons/image.svg' with { type: 'text' }
import list from '../../assets/icons/list.svg' with { type: 'text' }
import mdowLogo from '../../assets/icons/mdow-logo.svg' with { type: 'text' }
import search from '../../assets/icons/search.svg' with { type: 'text' }
import settings from '../../assets/icons/settings.svg' with { type: 'text' }
import sidebar from '../../assets/icons/sidebar.svg' with { type: 'text' }
import x from '../../assets/icons/x.svg' with { type: 'text' }
// Command palette and settings.
import arrowLeftRight from '../../assets/icons/arrow-left-right.svg' with { type: 'text' }
import filePlus from '../../assets/icons/file-plus.svg' with { type: 'text' }
import fileSearch from '../../assets/icons/file-search.svg' with { type: 'text' }
import keyboard from '../../assets/icons/keyboard.svg' with { type: 'text' }
import monitor from '../../assets/icons/monitor.svg' with { type: 'text' }
import moon from '../../assets/icons/moon.svg' with { type: 'text' }
import moveHorizontal from '../../assets/icons/move-horizontal.svg' with { type: 'text' }
import refreshCw from '../../assets/icons/refresh-cw.svg' with { type: 'text' }
import rotateCcw from '../../assets/icons/rotate-ccw.svg' with { type: 'text' }
import sun from '../../assets/icons/sun.svg' with { type: 'text' }
import zoomIn from '../../assets/icons/zoom-in.svg' with { type: 'text' }
import zoomOut from '../../assets/icons/zoom-out.svg' with { type: 'text' }

export const ICONS = {
  'alert-circle': alertCircle,
  check: check,
  'chevron-down': chevronDown,
  'chevron-right': chevronRight,
  'chevron-up': chevronUp,
  clock: clock,
  code: code,
  command: command,
  copy: copy,
  expand: expand,
  'external-link': externalLink,
  file: file,
  'file-text': fileText,
  'folder-open': folderOpen,
  folder: folder,
  image: image,
  list: list,
  'mdow-logo': mdowLogo,
  search: search,
  settings: settings,
  sidebar: sidebar,
  x: x,
  'arrow-left-right': arrowLeftRight,
  'file-plus': filePlus,
  'file-search': fileSearch,
  keyboard: keyboard,
  monitor: monitor,
  moon: moon,
  'move-horizontal': moveHorizontal,
  'refresh-cw': refreshCw,
  'rotate-ccw': rotateCcw,
  sun: sun,
  'zoom-in': zoomIn,
  'zoom-out': zoomOut,
}

export type IconName = keyof typeof ICONS
