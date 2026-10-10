import { expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { invoke } from '@tauri-apps/api/tauri'
import CompletedExtractionOutput from '../CompletedExtractionOutput.vue'

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn() }))

it('does not open a different result when the directory check returns after switching records', async () => {
  let finish!: (value: string) => void
  vi.mocked(invoke).mockReset().mockReturnValueOnce(new Promise(resolve => { finish = resolve }))
  const wrapper = mount(CompletedExtractionOutput, { props: { path: 'D:/first' } })
  await wrapper.get('button').trigger('click')
  await wrapper.setProps({ path: 'D:/second' })
  finish('occupied')
  await flushPromises()
  expect(invoke).not.toHaveBeenCalledWith('open_in_explorer', expect.anything())
  expect(wrapper.get('button').attributes('disabled')).toBeUndefined()
  wrapper.unmount()
})

it('opens the recorded directory and explains a moved result without launching Explorer', async () => {
  vi.mocked(invoke).mockReset().mockResolvedValueOnce('occupied').mockResolvedValueOnce(undefined)
  const wrapper = mount(CompletedExtractionOutput, { props: { path: 'D:/output/trip' } })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  expect(invoke).toHaveBeenCalledWith('open_in_explorer', { path: 'D:/output/trip' })
  vi.mocked(invoke).mockClear().mockResolvedValueOnce('missing')
  await wrapper.get('button').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('已被移动或删除')
  expect(invoke).not.toHaveBeenCalledWith('open_in_explorer', expect.anything())
  wrapper.unmount()
})

it('keeps an open failure visible in the dialog and allows retry', async () => {
  vi.mocked(invoke).mockReset().mockResolvedValueOnce('empty').mockRejectedValueOnce(new Error('Explorer failed'))
  const wrapper = mount(CompletedExtractionOutput, { props: { path: 'D:/output' } })
  await wrapper.get('button').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('无法打开输出目录')
  vi.mocked(invoke).mockResolvedValueOnce('empty').mockResolvedValueOnce(undefined)
  await wrapper.get('button').trigger('click')
  await flushPromises()
  expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  wrapper.unmount()
})
