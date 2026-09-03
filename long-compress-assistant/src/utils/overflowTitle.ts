const isOverflowCandidate = (element: HTMLElement) => {
  const style = window.getComputedStyle(element)
  return element.classList.contains('truncate') || style.textOverflow === 'ellipsis'
}

const findOverflowElement = (target: EventTarget | null, root: Document | HTMLElement) => {
  let element = target instanceof HTMLElement ? target : null
  while (element) {
    if (isOverflowCandidate(element) && element.scrollWidth > element.clientWidth + 1) return element
    if (element === root || element.parentElement === null) return null
    element = element.parentElement
  }
  return null
}

export const installOverflowTitles = (root: Document | HTMLElement = document) => {
  const exposeFullText = (event: Event) => {
    const element = findOverflowElement(event.target, root)
    if (!element || (element.hasAttribute('title') && element.dataset.autoOverflowTitle !== 'true')) return
    const text = element.textContent?.replace(/\s+/g, ' ').trim()
    if (text) {
      element.title = text
      element.dataset.autoOverflowTitle = 'true'
    }
  }

  root.addEventListener('mouseover', exposeFullText)
  root.addEventListener('focusin', exposeFullText)
  return () => {
    root.removeEventListener('mouseover', exposeFullText)
    root.removeEventListener('focusin', exposeFullText)
  }
}
