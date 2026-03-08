import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import FileUpload from '../FileUpload.vue'
import { createMockFile } from '../../../tests/setup'

// Mock Tauri API
vi.mock('@tauri-apps/api', () => ({
  invoke: vi.fn(),
}))

describe('FileUpload Component', () => {
  let wrapper: any

  beforeEach(() => {
    wrapper = mount(FileUpload)
  })

  it('renders correctly', () => {
    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('input[type="file"]').exists()).toBe(true)
    expect(wrapper.find('button').exists()).toBe(true)
  })

  it('accepts file selection', async () => {
    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('test.txt', 1024)

    // 模拟文件选择
    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })

    await fileInput.trigger('change')

    // 验证文件被接受
    expect(wrapper.vm.selectedFiles).toHaveLength(1)
  })

  it('rejects files that are too large', async () => {
    const fileInput = wrapper.find('input[type="file"]')
    const largeFile = createMockFile('large.txt', 1024 * 1024 * 101) // 101MB

    Object.defineProperty(fileInput.element, 'files', {
      value: [largeFile],
    })

    await fileInput.trigger('change')

    // 验证大文件被拒绝
    expect(wrapper.vm.selectedFiles).toHaveLength(0)
    // 应该显示错误消息
    expect(wrapper.text()).toContain('文件大小超过限制')
  })

  it('handles multiple file selection', async () => {
    const fileInput = wrapper.find('input[type="file"]')
    const files = [
      createMockFile('file1.txt', 1024),
      createMockFile('file2.txt', 2048),
      createMockFile('file3.txt', 3072),
    ]

    Object.defineProperty(fileInput.element, 'multiple', { value: true })
    Object.defineProperty(fileInput.element, 'files', {
      value: files,
    })

    await fileInput.trigger('change')

    expect(wrapper.vm.selectedFiles).toHaveLength(3)
  })

  it('clears selected files', async () => {
    // 先选择文件
    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('test.txt', 1024)

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })

    await fileInput.trigger('change')
    expect(wrapper.vm.selectedFiles).toHaveLength(1)

    // 清除文件
    await wrapper.find('.clear-button').trigger('click')
    expect(wrapper.vm.selectedFiles).toHaveLength(0)
  })

  it('emits upload event with files', async () => {
    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('test.txt', 1024)

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })

    await fileInput.trigger('change')
    await wrapper.find('button').trigger('click')

    // 验证emit事件
    expect(wrapper.emitted('upload')).toBeTruthy()
    expect(wrapper.emitted('upload')[0][0]).toHaveLength(1)
  })

  it('shows file information', async () => {
    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('document.pdf', 2048, 'application/pdf')

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })

    await fileInput.trigger('change')

    // 验证文件信息显示
    expect(wrapper.text()).toContain('document.pdf')
    expect(wrapper.text()).toContain('2 KB')
  })

  it('handles drag and drop', async () => {
    const dropZone = wrapper.find('.drop-zone')
    const mockFile = createMockFile('dropped.txt', 1024)

    const dataTransfer = {
      files: [mockFile],
      items: [
        {
          kind: 'file',
          getAsFile: () => mockFile,
        },
      ],
      types: ['Files'],
    }

    // 模拟拖放事件
    const dropEvent = new Event('drop')
    Object.defineProperty(dropEvent, 'dataTransfer', { value: dataTransfer })

    await dropZone.element.dispatchEvent(dropEvent)

    expect(wrapper.vm.selectedFiles).toHaveLength(1)
  })

  it('validates file types', async () => {
    const fileInput = wrapper.find('input[type="file"]')

    // 设置只接受特定类型
    await wrapper.setProps({
      accept: '.txt,.pdf',
    })

    const validFile = createMockFile('test.txt', 1024, 'text/plain')
    const invalidFile = createMockFile('test.exe', 1024, 'application/x-msdownload')

    Object.defineProperty(fileInput.element, 'files', {
      value: [validFile, invalidFile],
    })

    await fileInput.trigger('change')

    // 只应该接受有效文件
    expect(wrapper.vm.selectedFiles).toHaveLength(1)
    expect(wrapper.vm.selectedFiles[0].name).toBe('test.txt')
  })
})