// Embedded as text so `bun build --compile` carries them inside the binary.
import alertCircle from '../../assets/icons/alert-circle.svg' with { type: 'text' }
import check from '../../assets/icons/check.svg' with { type: 'text' }
import chevronDown from '../../assets/icons/chevron-down.svg' with { type: 'text' }
import chevronRight from '../../assets/icons/chevron-right.svg' with { type: 'text' }
import chevronUp from '../../assets/icons/chevron-up.svg' with { type: 'text' }
import clock from '../../assets/icons/clock.svg' with { type: 'text' }
import command from '../../assets/icons/command.svg' with { type: 'text' }
import copy from '../../assets/icons/copy.svg' with { type: 'text' }
import expand from '../../assets/icons/expand.svg' with { type: 'text' }
import file from '../../assets/icons/file.svg' with { type: 'text' }
import folderOpen from '../../assets/icons/folder-open.svg' with { type: 'text' }
import folder from '../../assets/icons/folder.svg' with { type: 'text' }
import list from '../../assets/icons/list.svg' with { type: 'text' }
import mdowLogo from '../../assets/icons/mdow-logo.svg' with { type: 'text' }
import search from '../../assets/icons/search.svg' with { type: 'text' }
import settings from '../../assets/icons/settings.svg' with { type: 'text' }
import sidebar from '../../assets/icons/sidebar.svg' with { type: 'text' }
import x from '../../assets/icons/x.svg' with { type: 'text' }

export const ICONS = {
  'alert-circle': alertCircle,
  check: check,
  'chevron-down': chevronDown,
  'chevron-right': chevronRight,
  'chevron-up': chevronUp,
  clock: clock,
  command: command,
  copy: copy,
  expand: expand,
  file: file,
  'folder-open': folderOpen,
  folder: folder,
  list: list,
  'mdow-logo': mdowLogo,
  search: search,
  settings: settings,
  sidebar: sidebar,
  x: x,
}

export type IconName = keyof typeof ICONS
