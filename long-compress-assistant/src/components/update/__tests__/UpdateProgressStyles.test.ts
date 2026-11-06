import source from '../UpdateProgress.vue?raw'
import { compileStyle, parse } from '@vue/compiler-sfc'
import { expect, it } from 'vitest'

it('compiled reduced-motion CSS centers the update indicator without moving the document', () => {
  const { descriptor } = parse(source)
  const scopeId = 'data-v-update-style-regression'
  const compiled = compileStyle({ source: descriptor.styles[0].content, filename: 'UpdateProgress.vue', id: scopeId, scoped: true })
  expect(compiled.errors).toEqual([])
  const style = document.createElement('style')
  style.textContent = compiled.code
  const track = document.createElement('div')
  const fill = document.createElement('div')
  track.className = 'update-progress-track'
  fill.className = 'indeterminate'
  for (const el of [track, fill]) el.setAttribute(scopeId, '')
  track.append(fill)
  document.head.append(style)
  document.body.append(track)
  document.documentElement.classList.add('reduce-motion')
  try {
    expect(getComputedStyle(document.documentElement).marginLeft).not.toBe('31%')
    expect(getComputedStyle(fill).marginLeft).toBe('31%')
    expect(getComputedStyle(fill).animation).toBe('none')
    document.documentElement.classList.remove('reduce-motion')
    expect(getComputedStyle(fill).marginLeft).not.toBe('31%')
  } finally {
    document.documentElement.classList.remove('reduce-motion')
    track.remove()
    style.remove()
  }
})
