// Animate physical height, never scale text or controls. Share the same motion in both centers.
export function useDrawerTransition() {
  const running = new WeakMap<HTMLElement, Animation>()
  const interrupted = new WeakMap<HTMLElement, { height: number; opacity: string }>()
  const clear = (el: HTMLElement) => {
    for (const property of ['height', 'opacity', 'overflow']) el.style.removeProperty(property)
  }
  const cancel = (element: Element) => {
    const el = element as HTMLElement
    interrupted.set(el, { height: el.getBoundingClientRect().height, opacity: getComputedStyle(el).opacity })
    running.get(el)?.cancel()
    running.delete(el)
    clear(el)
  }
  const animate = (element: Element, done: () => void, opening: boolean) => {
    const el = element as HTMLElement
    const previous = interrupted.get(el)
    interrupted.delete(el)
    const fullHeight = el.getBoundingClientRect().height
    const style = getComputedStyle(el)
    if (!el.animate || document.documentElement.classList.contains('reduce-motion')
      || window.matchMedia('(prefers-reduced-motion: reduce)').matches) { clear(el); done(); return }
    const full = { height: `${fullHeight}px`, opacity: '1', paddingTop: style.paddingTop, paddingBottom: style.paddingBottom }
    const empty = { height: '0px', opacity: '0', paddingTop: '0px', paddingBottom: '0px' }
    const from = previous ? { ...full, height: `${previous.height}px`, opacity: previous.opacity } : opening ? empty : full
    el.style.overflow = 'hidden'
    const animation = el.animate([from, opening ? full : empty], {
      duration: 350, easing: 'cubic-bezier(0.4, 0, 0.2, 1)', fill: 'both',
    })
    running.set(el, animation)
    animation.onfinish = () => {
      if (running.get(el) !== animation) return
      running.delete(el)
      animation.cancel()
      clear(el)
      done()
    }
  }
  return {
    enter: (el: Element, done: () => void) => animate(el, done, true),
    leave: (el: Element, done: () => void) => animate(el, done, false),
    cancel,
  }
}
