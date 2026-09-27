import { prefersReducedMotion } from './motion'

type Target = Element | Range
type Block = 'start' | 'center'

interface ScrollToOptions {
  block?: Block
  /** Animate when the target is close enough that the animation can land accurately. */
  smooth?: boolean
  /** Extra pixels between the scroller's top edge and the target when `block` is 'start'. */
  offset?: number
}

// Past this distance (in viewports) the rendered layout is mostly estimates, so a smooth
// animation would aim at a position that no longer holds the target by the time it arrives.
const SMOOTH_MAX_VIEWPORTS = 2
// Blocks near the new position render at their real size a frame or two after the jump, so the
// target has to hold still for a few frames before we trust it.
const STABLE_FRAMES = 3
const MAX_CORRECTION_FRAMES = 30

function targetRect(target: Target): DOMRect {
  const rect = target.getBoundingClientRect()
  if (rect.width === 0 && rect.height === 0 && target instanceof Range) {
    return target.startContainer.parentElement?.getBoundingClientRect() ?? rect
  }
  return rect
}

function targetElement(target: Target): Element | null {
  return target instanceof Range ? target.startContainer.parentElement : target
}

function isConnected(target: Target): boolean {
  return target instanceof Range ? target.startContainer.isConnected : target.isConnected
}

function distance(scroller: HTMLElement, target: Target, block: Block, offset: number): number {
  const box = scroller.getBoundingClientRect()
  const rect = targetRect(target)
  if (block === 'center') {
    return rect.top + rect.height / 2 - (box.top + scroller.clientHeight / 2)
  }
  return rect.top - box.top - offset
}

function align(scroller: HTMLElement, target: Target, block: Block, offset: number): void {
  // Native scrollIntoView makes the target's content-visibility ancestors lay out for real,
  // which a scrollTop computed from estimated heights cannot do.
  targetElement(target)?.scrollIntoView({ block, inline: 'nearest', behavior: 'instant' })
  const delta = distance(scroller, target, block, offset)
  if (Math.abs(delta) > 1) scroller.scrollTop += delta
}

/**
 * Scrolls `target` into place inside `scroller`, then keeps correcting for a few frames.
 *
 * Long documents use `content-visibility: auto`, so blocks far from the viewport have estimated
 * heights. The jump lands on the target, then the blocks around it render at their real size
 * and can push it around; re-aligning until it holds still makes long jumps exact.
 * Any user scroll input cancels the correction. Returns a cancel function.
 */
export function scrollToTarget(
  scroller: HTMLElement,
  target: Target,
  { block = 'start', smooth = false, offset = 0 }: ScrollToOptions = {},
): () => void {
  const initial = distance(scroller, target, block, offset)
  const near = Math.abs(initial) < scroller.clientHeight * SMOOTH_MAX_VIEWPORTS
  if (smooth && near && !prefersReducedMotion()) {
    scroller.scrollBy({ top: initial, behavior: 'smooth' })
    return () => {}
  }

  let frame = 0
  let stableFrames = 0
  let remaining = MAX_CORRECTION_FRAMES
  const cancel = () => {
    cancelAnimationFrame(frame)
    scroller.removeEventListener('wheel', cancel)
    scroller.removeEventListener('pointerdown', cancel)
    window.removeEventListener('keydown', cancel, true)
  }
  const step = () => {
    if (!isConnected(target)) return cancel()
    if (Math.abs(distance(scroller, target, block, offset)) > 1) {
      align(scroller, target, block, offset)
      stableFrames = 0
    } else {
      stableFrames += 1
    }
    remaining -= 1
    if (stableFrames >= STABLE_FRAMES || remaining <= 0) return cancel()
    frame = requestAnimationFrame(step)
  }

  scroller.addEventListener('wheel', cancel, { passive: true })
  scroller.addEventListener('pointerdown', cancel)
  window.addEventListener('keydown', cancel, true)
  align(scroller, target, block, offset)
  frame = requestAnimationFrame(step)
  return cancel
}

export function findMarkdownScroller(from: Element): HTMLElement | null {
  return from.closest<HTMLElement>('[data-markdown-scroller]')
}
