import { afterEach, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import ExtractionOutputPreview from '../ExtractionOutputPreview.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@/stores/app', () => ({ useAppStore: () => ({ settings: { conflictPolicy: 'rename' }, setError: vi.fn() }) }))
afterEach(() => vi.useRealTimers())

it('改变目录后不显示旧预览，并使用当前子目录选择重新查询', async () => {
  vi.useFakeTimers()
  let oldReply!: (path: string) => void
  mocks.invoke.mockImplementationOnce(() => new Promise(resolve => { oldReply = resolve }))
  const wrapper = mount(ExtractionOutputPreview, {
    props: { source: 'C:/照片.tar.gz', output: 'D:/旧目录', subfolder: true },
    global: { plugins: [createPinia()] },
  })
  await vi.advanceTimersByTimeAsync(150)
  await wrapper.setProps({ output: 'E:/新目录', subfolder: false })
  mocks.invoke.mockResolvedValueOnce('E:/新目录')
  await vi.advanceTimersByTimeAsync(150)
  oldReply('D:/旧目录/照片')
  await flushPromises()
  expect(wrapper.get('[data-testid="extraction-output-path"]').text()).toBe('E:/新目录')
  expect(wrapper.text()).toContain('不另建同名子目录')
  expect(mocks.invoke).toHaveBeenCalledWith('preview_extraction_output', {
    filePath: 'C:/照片.tar.gz', outputPath: 'E:/新目录', createSubdirectory: false,
  })
  await wrapper.setProps({ output: '' })
  expect(wrapper.find('[data-testid="extraction-output-path"]').exists()).toBe(false)
  wrapper.unmount()
})

it('更换目录后忽略旧目录检查，显示已有内容和共用提示并提供更换入口', async () => {
  vi.useFakeTimers()
  let oldState!: (state: string) => void
  mocks.invoke.mockReset()
  mocks.invoke.mockImplementation(async (command, args) => {
    if (command === 'preview_extraction_output') return args.outputPath
    if (args.path === 'D:/old') return new Promise(resolve => { oldState = resolve })
    return 'occupied'
  })
  const wrapper = mount(ExtractionOutputPreview, {
    props: { source: 'C:/photos.zip', output: 'D:/old', subfolder: false, sharedCount: 2 },
    global: { plugins: [createPinia()] },
  })
  await vi.advanceTimersByTimeAsync(150)
  await wrapper.setProps({ output: 'E:/new' })
  await vi.advanceTimersByTimeAsync(150)
  oldState('file')
  await flushPromises()
  expect(wrapper.text()).toContain('目录已有内容')
  expect(wrapper.text()).not.toContain('此位置是文件')
  expect(wrapper.text()).toContain('2 项任务共用此目录')
  await wrapper.findAll('button').find(button => button.text() === '更换目录')!.trigger('click')
  expect(wrapper.emitted('change-output')).toHaveLength(1)
  wrapper.unmount()
})
