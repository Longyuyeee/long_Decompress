import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import SavedLocationPicker from '../SavedLocationPicker.vue'
import { useSavedLocations } from '@/composables/useSavedLocations'
const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/tauri', () => ({ invoke }))
async function choose(wrapper: ReturnType<typeof mount>) {
  await wrapper.get('[data-testid="saved-output-select"]').trigger('click')
  ;(document.querySelector('[role="menuitem"]') as HTMLButtonElement).click()
}

describe('saved output locations', () => {
  beforeEach(() => {
    localStorage.clear()
    invoke.mockReset().mockResolvedValue('empty')
    useSavedLocations().toggleBookmark('D:/资料')
  })
  it('shows useful empty groups and does not open when disabled', async () => {
    localStorage.clear()
    const wrapper = mount(SavedLocationPicker, { props: { disabled: true } })
    await wrapper.get('[data-testid="saved-output-select"]').trigger('click')
    expect(document.querySelector('[role="menu"]')).toBeNull()
    await wrapper.setProps({ disabled: false })
    await wrapper.get('[data-testid="saved-output-select"]').trigger('click')
    expect(document.querySelector('[role="menu"]')?.textContent).toContain('点击星标，收藏当前目录')
    expect(document.querySelector('[role="menu"]')?.textContent).toContain('使用过的目录会显示在这里')
    wrapper.unmount()
    expect(document.querySelector('[role="menu"]')).toBeNull()
  })
  it('checks an existing bookmark and remembers it before changing the output', async () => {
    const wrapper = mount(SavedLocationPicker, { props: { current: 'C:/old' } })
    await choose(wrapper)
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('inspect_extraction_destination', { path: 'D:/资料' })
    expect(wrapper.emitted('select')).toEqual([['D:/资料']])
    expect(useSavedLocations().recent.value).toEqual(['D:/资料'])
    wrapper.unmount()
  })
  it.each(['missing', 'unavailable', 'file'])('preserves my current output if the location is %s', async state => {
    invoke.mockResolvedValue(state)
    const wrapper = mount(SavedLocationPicker, { props: { current: 'C:/old' } })
    await choose(wrapper)
    await flushPromises()
    expect(wrapper.emitted('select')).toBeUndefined()
    expect(wrapper.get('[role="status"]').text()).not.toBe('')
    expect(useSavedLocations().recent.value).toEqual([])
    wrapper.unmount()
  })
  it.each(['change', 'disable', 'unmount'])('drops a delayed directory response after %s', async action => {
    let finish!: (state: string) => void
    invoke.mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const wrapper = mount(SavedLocationPicker, { props: { current: 'C:/old' } })
    await choose(wrapper)
    if (action === 'change') await wrapper.setProps({ current: 'C:/new' })
    if (action === 'disable') await wrapper.setProps({ disabled: true })
    if (action === 'unmount') wrapper.unmount()
    finish('empty')
    await flushPromises()
    expect(wrapper.emitted('select')).toBeUndefined()
    if (action !== 'unmount') wrapper.unmount()
  })
  it('refreshes locations from another mounted picker without losing its bookmarks', async () => {
    const wrapper = mount(SavedLocationPicker, { props: { current: 'E:/输出' } })
    useSavedLocations().toggleBookmark('F:/other')
    await wrapper.get('[data-testid="saved-output-select"]').trigger('click')
    expect(document.querySelector('[role="menu"]')?.textContent).toContain('F:/other')
    await wrapper.get('[data-testid="bookmark-output"]').trigger('click')
    await flushPromises()
    expect(useSavedLocations().bookmarks.value).toEqual(['D:/资料', 'F:/other', 'E:/输出'])
    wrapper.unmount()
  })
})
