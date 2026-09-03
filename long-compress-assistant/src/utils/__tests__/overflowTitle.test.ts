import { afterEach, describe, expect, it } from 'vitest'
import { installOverflowTitles } from '../overflowTitle'

const setDimensions = (element: HTMLElement, clientWidth: number, scrollWidth: number) => {
  Object.defineProperty(element, 'clientWidth', { configurable: true, value: clientWidth })
  Object.defineProperty(element, 'scrollWidth', { configurable: true, value: scrollWidth })
}

describe('overflow titles', () => {
  afterEach(() => { document.body.innerHTML = '' })

  it('adds the full rendered text to an actually truncated element on hover', () => {
    document.body.innerHTML = '<span class="truncate">很长的压缩包完整名称.rar</span>'
    const element = document.querySelector('span')!
    setDimensions(element, 80, 240)
    const uninstall = installOverflowTitles(document)

    element.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }))

    expect(element.title).toBe('很长的压缩包完整名称.rar')
    expect(element.dataset.autoOverflowTitle).toBe('true')

    element.textContent = '更新后的完整名称.7z'
    element.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }))
    expect(element.title).toBe('更新后的完整名称.7z')
    uninstall()
  })

  it('preserves explicit titles and ignores text that is not truncated', () => {
    document.body.innerHTML = '<span class="truncate" title="精确路径">显示文本</span><span class="truncate">短文本</span>'
    const [explicit, fitting] = [...document.querySelectorAll('span')]
    setDimensions(explicit, 40, 120)
    setDimensions(fitting, 120, 80)
    const uninstall = installOverflowTitles(document)

    explicit.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }))
    fitting.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }))

    expect(explicit.title).toBe('精确路径')
    expect(fitting.hasAttribute('title')).toBe(false)
    uninstall()
  })
})
