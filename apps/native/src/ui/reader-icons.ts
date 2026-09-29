// Icons used only by the reader chrome, kept apart from the shared registry in icons.ts.
import arrowLeftRight from '../../assets/icons/arrow-left-right.svg' with { type: 'text' }
import fileText from '../../assets/icons/file-text.svg' with { type: 'text' }
import foldHorizontal from '../../assets/icons/fold-horizontal.svg' with { type: 'text' }

export const READER_ICONS = {
  'arrow-left-right': arrowLeftRight,
  'file-text': fileText,
  'fold-horizontal': foldHorizontal,
}

export type ReaderIconName = keyof typeof READER_ICONS
