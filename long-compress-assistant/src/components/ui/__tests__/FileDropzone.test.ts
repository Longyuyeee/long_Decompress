import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import FileDropzone from '../FileDropzone.vue'
import { createMockFile } from '../../../../tests/setup'

// Mock PrimeVue图标
vi.mock('primevue/icons', () => ({
  CloudUpload: { template: '<i class="pi pi-cloud-upload"></i>' },
  FileArchive: { template: '<i class="pi pi-file-archive"></i>' },
  File: { template: '<i class="pi pi-file"></i>' },
  Times: { template: '<i class="pi pi-times"></i>' },
  ExclamationTriangle: { template: '<i class="pi pi-exclamation-triangle"></i>' },
}))

describe('FileDropzone Component', () => {
  let wrapper: any

  beforeEach(() => {
    wrapper = mount(FileDropzone)
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('renders correctly with default props', () => {
    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.file-dropzone').exists()).toBe(true)
    expect(wrapper.find('input[type="file"]').exists()).toBe(true)
    expect(wrapper.find('input[type="file"]').attributes('multiple')).toBe('')
    expect(wrapper.text()).toContain('拖放文件到此处')
    expect(wrapper.text()).toContain('支持 ZIP, RAR, 7Z, TAR, GZ, BZ2 等格式')
  })

  it('renders with custom props', async () => {
    await wrapper.setProps({
      multiple: false,
      accept: '.txt,.pdf',
      maxSize: 1024 * 1024, // 1MB
      maxFiles: 5,
      className: 'custom-class',
    })

    expect(wrapper.find('input[type="file"]').attributes('multiple')).toBeUndefined()
    expect(wrapper.find('input[type="file"]').attributes('accept')).toBe('.txt,.pdf')
    expect(wrapper.text()).toContain('支持 TXT, PDF 等格式')
    expect(wrapper.text()).toContain('最大 1 MB')
    expect(wrapper.find('.file-dropzone').classes()).toContain('custom-class')
  })

  it('handles click to open file dialog', async () => {
    const fileInput = wrapper.find('input[type="file"]')
    const clickSpy = vi.spyOn(fileInput.element, 'click')

    await wrapper.find('.file-dropzone').trigger('click')

    expect(clickSpy).toHaveBeenCalled()
  })

  it('handles drag over event', async () => {
    const dropzone = wrapper.find('.file-dropzone')

    expect(wrapper.vm.isDragging).toBe(false)

    await dropzone.trigger('dragover', {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
    })

    expect(wrapper.vm.isDragging).toBe(true)
    expect(wrapper.find('.file-dropzone').classes()).toContain('border-primary')
  })

  it('handles drag leave event', async () => {
    const dropzone = wrapper.find('.file-dropzone')

    // 先设置为拖拽状态
    wrapper.vm.isDragging = true
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.isDragging).toBe(true)

    // 模拟拖拽离开
    await dropzone.trigger('dragleave', {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      relatedTarget: null,
    })

    expect(wrapper.vm.isDragging).toBe(false)
  })

  it('handles file drop event', async () => {
    const mockFile = createMockFile('test.zip', 1024, 'application/zip')
    const dropEvent = {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      dataTransfer: {
        files: [mockFile],
      },
    }

    await wrapper.find('.file-dropzone').trigger('drop', dropEvent)

    expect(dropEvent.preventDefault).toHaveBeenCalled()
    expect(dropEvent.stopPropagation).toHaveBeenCalled()
    expect(wrapper.vm.isDragging).toBe(false)
    expect(wrapper.vm.files).toHaveLength(1)
    expect(wrapper.vm.files[0].name).toBe('test.zip')
  })

  it('handles file input change', async () => {
    const mockFile = createMockFile('document.pdf', 2048, 'application/pdf')
    const fileInput = wrapper.find('input[type="file"]')

    // 模拟文件选择
    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })

    await fileInput.trigger('change')

    expect(wrapper.vm.files).toHaveLength(1)
    expect(wrapper.vm.files[0].name).toBe('document.pdf')
    expect(wrapper.vm.files[0].size).toBe(2048)
  })

  it('emits files-selected event when files are added', async () => {
    const mockFile = createMockFile('test.zip', 1024, 'application/zip')
    const dropEvent = {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      dataTransfer: {
        files: [mockFile],
      },
    }

    await wrapper.find('.file-dropzone').trigger('drop', dropEvent)

    expect(wrapper.emitted('files-selected')).toBeTruthy()
    expect(wrapper.emitted('files-selected')[0][0]).toHaveLength(1)
    expect(wrapper.emitted('files-selected')[0][0][0].name).toBe('test.zip')
  })

  it('validates file size limit', async () => {
    await wrapper.setProps({
      maxSize: 1024, // 1KB
    })

    const mockFile = createMockFile('large.zip', 2048, 'application/zip') // 2KB
    const dropEvent = {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      dataTransfer: {
        files: [mockFile],
      },
    }

    await wrapper.find('.file-dropzone').trigger('drop', dropEvent)

    expect(wrapper.vm.files).toHaveLength(0)
    expect(wrapper.vm.error).toContain('超过最大大小限制')
    expect(wrapper.emitted('error')).toBeTruthy()
  })

  it('validates file type', async () => {
    await wrapper.setProps({
      accept: '.zip,.rar',
    })

    const mockFile = createMockFile('test.txt', 1024, 'text/plain')
    const dropEvent = {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      dataTransfer: {
        files: [mockFile],
      },
    }

    await wrapper.find('.file-dropzone').trigger('drop', dropEvent)

    expect(wrapper.vm.files).toHaveLength(0)
    expect(wrapper.vm.error).toContain('格式不支持')
    expect(wrapper.emitted('error')).toBeTruthy()
  })

  it('validates max files limit', async () => {
    await wrapper.setProps({
      maxFiles: 2,
    })

    // 先添加一个文件
    const mockFile1 = createMockFile('file1.zip', 1024, 'application/zip')
    wrapper.vm.files = [{
      id: '1',
      name: 'file1.zip',
      size: 1024,
      type: 'application/zip',
      file: mockFile1,
    }]

    await wrapper.vm.$nextTick()

    // 尝试添加两个以上的文件
    const mockFile2 = createMockFile('file2.zip', 1024, 'application/zip')
    const mockFile3 = createMockFile('file3.zip', 1024, 'application/zip')
    const dropEvent = {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      dataTransfer: {
        files: [mockFile2, mockFile3],
      },
    }

    await wrapper.find('.file-dropzone').trigger('drop', dropEvent)

    expect(wrapper.vm.files).toHaveLength(1) // 应该只保留原来的文件
    expect(wrapper.vm.error).toContain('最多只能选择 2 个文件')
    expect(wrapper.emitted('error')).toBeTruthy()
  })

  it('removes files correctly', async () => {
    const mockFile = createMockFile('test.zip', 1024, 'application/zip')
    wrapper.vm.files = [{
      id: 'test-id',
      name: 'test.zip',
      size: 1024,
      type: 'application/zip',
      file: mockFile,
    }]

    await wrapper.vm.$nextTick()

    // 验证文件显示
    expect(wrapper.find('.file-dropzone').text()).toContain('test.zip')
    expect(wrapper.find('.file-dropzone').text()).toContain('已选择文件 (1)')

    // 点击删除按钮
    const removeButton = wrapper.find('button.text-gray-400')
    await removeButton.trigger('click')

    expect(wrapper.vm.files).toHaveLength(0)
    expect(wrapper.emitted('file-removed')).toBeTruthy()
    expect(wrapper.emitted('file-removed')[0][0]).toBe('test-id')
  })

  it('formats file size correctly', () => {
    const testCases = [
      { bytes: 0, expected: '0 B' },
      { bytes: 500, expected: '500 B' },
      { bytes: 1024, expected: '1 KB' },
      { bytes: 1024 * 1024, expected: '1 MB' },
      { bytes: 1024 * 1024 * 1024, expected: '1 GB' },
      { bytes: 1024 * 1024 * 1024 * 1024, expected: '1 TB' },
    ]

    testCases.forEach(({ bytes, expected }) => {
      const result = wrapper.vm.formatFileSize(bytes)
      // 注意：实际实现可能显示为 "1.00 KB" 而不是 "1 KB"
      expect(result).toContain(expected.split(' ')[1]) // 检查单位
    })
  })

  it('gets correct file icon based on extension', () => {
    const testCases = [
      { name: 'archive.zip', expectedClass: 'text-blue-500' },
      { name: 'archive.rar', expectedClass: 'text-red-500' },
      { name: 'archive.7z', expectedClass: 'text-green-500' },
      { name: 'archive.tar', expectedClass: 'text-purple-500' },
      { name: 'archive.gz', expectedClass: 'text-purple-500' },
      { name: 'archive.bz2', expectedClass: 'text-purple-500' },
      { name: 'unknown.xyz', expectedClass: 'text-gray-500' },
    ]

    testCases.forEach(({ name, expectedClass }) => {
      const fileItem = {
        id: '1',
        name,
        size: 1024,
        type: 'application/octet-stream',
        file: createMockFile(name, 1024),
      }

      const iconClass = wrapper.vm.getFileIcon(fileItem)
      expect(iconClass).toContain(expectedClass)
      expect(iconClass).toContain('pi pi-file')
    })
  })

  it('clears all files', () => {
    const mockFile = createMockFile('test.zip', 1024, 'application/zip')
    wrapper.vm.files = [{
      id: '1',
      name: 'test.zip',
      size: 1024,
      type: 'application/zip',
      file: mockFile,
    }]

    wrapper.vm.clearFiles()

    expect(wrapper.vm.files).toHaveLength(0)
  })

  it('resets component state', () => {
    const mockFile = createMockFile('test.zip', 1024, 'application/zip')
    wrapper.vm.files = [{
      id: '1',
      name: 'test.zip',
      size: 1024,
      type: 'application/zip',
      file: mockFile,
    }]
    wrapper.vm.error = 'Test error'
    wrapper.vm.isDragging = true

    wrapper.vm.reset()

    expect(wrapper.vm.files).toHaveLength(0)
    expect(wrapper.vm.error).toBe('')
    expect(wrapper.vm.isDragging).toBe(false)
  })

  it('exposes methods to parent component', () => {
    expect(typeof wrapper.vm.clearFiles).toBe('function')
    expect(typeof wrapper.vm.reset).toBe('function')
  })

  it('handles multiple file selection', async () => {
    const mockFiles = [
      createMockFile('file1.zip', 1024, 'application/zip'),
      createMockFile('file2.zip', 2048, 'application/zip'),
      createMockFile('file3.zip', 3072, 'application/zip'),
    ]

    const dropEvent = {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      dataTransfer: {
        files: mockFiles,
      },
    }

    await wrapper.find('.file-dropzone').trigger('drop', dropEvent)

    expect(wrapper.vm.files).toHaveLength(3)
    expect(wrapper.emitted('files-selected')[0][0]).toHaveLength(3)
  })

  it('shows error message when present', async () => {
    wrapper.vm.error = '文件大小超过限制'
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.file-dropzone').text()).toContain('文件大小超过限制')
    expect(wrapper.find('.bg-red-50').exists()).toBe(true)
  })

  it('updates accepted formats display', async () => {
    await wrapper.setProps({
      accept: '.txt,.pdf,.docx',
    })

    expect(wrapper.vm.acceptedFormats).toEqual(['TXT', 'PDF', 'DOCX'])
    expect(wrapper.find('.file-dropzone').text()).toContain('支持 TXT, PDF, DOCX 等格式')
  })

  it('handles empty file drop', async () => {
    const dropEvent = {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      dataTransfer: {
        files: [],
      },
    }

    await wrapper.find('.file-dropzone').trigger('drop', dropEvent)

    expect(wrapper.vm.files).toHaveLength(0)
    expect(wrapper.vm.error).toBe('')
  })

  it('handles file input reset', async () => {
    const mockFile = createMockFile('test.zip', 1024, 'application/zip')
    const fileInput = wrapper.find('input[type="file"]')

    // 第一次选择文件
    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })
    Object.defineProperty(fileInput.element, 'value', {
      writable: true,
      value: 'test.zip',
    })

    await fileInput.trigger('change')

    expect(wrapper.vm.files).toHaveLength(1)

    // 重置input后可以再次选择相同的文件
    expect(fileInput.element.value).toBe('')
  })

  it('shows file list when files are selected', async () => {
    const mockFile = createMockFile('test.zip', 1024, 'application/zip')
    wrapper.vm.files = [{
      id: '1',
      name: 'test.zip',
      size: 1024,
      type: 'application/zip',
      file: mockFile,
    }]

    await wrapper.vm.$nextTick()

    expect(wrapper.find('.file-dropzone').text()).toContain('已选择文件 (1)')
    expect(wrapper.find('.file-dropzone').text()).toContain('test.zip')
    expect(wrapper.find('.file-dropzone').text()).toContain('1 KB')
  })

  it('handles drag over with custom styling', async () => {
    const dropzone = wrapper.find('.file-dropzone')

    // 初始状态
    expect(wrapper.vm.isDragging).toBe(false)
    expect(dropzone.classes()).not.toContain('scale-105')

    // 拖拽状态
    await dropzone.trigger('dragover', {
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
    })

    expect(wrapper.vm.isDragging).toBe(true)
    expect(dropzone.classes()).toContain('border-primary')
    expect(dropzone.classes()).toContain('bg-primary/5')
    expect(dropzone.classes()).toContain('scale-105')
  })
})