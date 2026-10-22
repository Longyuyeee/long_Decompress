import { describe, expect, it } from 'vitest'
import { APP_NAVIGATION, TOOL_NAVIGATION, navigationPathForShortcut } from '../navigation'

describe('product navigation shortcuts', () => {
  it('has unique keyboard combinations and routes every displayed shortcut', () => {
    const all = [...APP_NAVIGATION, ...TOOL_NAVIGATION]
    expect(APP_NAVIGATION).toHaveLength(8)
    expect(new Set(all.map(item => item.shortcut)).size).toBe(all.length)
    for (const item of all) {
      expect(navigationPathForShortcut(new KeyboardEvent('keydown', { key: item.key, ctrlKey: true, shiftKey: Boolean(item.shift) }))).toBe(item.path)
    }
  })

  it('leaves typing, handled events, IME and other modifier combinations alone', () => {
    const editor = document.createElement('div')
    editor.setAttribute('contenteditable', 'true')
    const child = document.createElement('span'); editor.appendChild(child); document.body.appendChild(editor)
    const typed = new KeyboardEvent('keydown', { key: 'l', ctrlKey: true, shiftKey: true, bubbles: true })
    child.dispatchEvent(typed)
    expect(navigationPathForShortcut(typed)).toBeUndefined()
    editor.remove()
    for (const options of [{ altKey: true }, { isComposing: true }, { shiftKey: false }]) {
      expect(navigationPathForShortcut(new KeyboardEvent('keydown', { key: 'l', ctrlKey: true, shiftKey: true, ...options }))).toBeUndefined()
    }
    const handled = new KeyboardEvent('keydown', { key: 'l', ctrlKey: true, shiftKey: true, cancelable: true })
    handled.preventDefault()
    expect(navigationPathForShortcut(handled)).toBeUndefined()
  })
})
