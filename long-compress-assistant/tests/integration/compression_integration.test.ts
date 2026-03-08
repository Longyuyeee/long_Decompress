import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import App from '../src/App.vue'
import { createMockFile } from '../tests/setup'

// Mock Tauri API
vi.mock('@tauri-apps/api', () => ({
  invoke: vi.fn(),
  window: {
    appWindow: {
      listen: vi.fn(),
      emit: vi.fn(),
    },
  },
  path: {
    appDataDir: vi.fn(() => Promise.resolve('/test/app/data')),
  },
  fs: {
    readDir: vi.fn(),
    readTextFile: vi.fn(),
    writeTextFile: vi.fn(),
  },
}))

describe('Compression Integration Tests', () => {
  let wrapper: any
  let mockInvoke: any

  beforeEach(() => {
    setActivePinia(createPinia())

    mockInvoke = vi.fn()
    vi.mocked(require('@tauri-apps/api').invoke).mockImplementation(mockInvoke)

    wrapper = mount(App)
  })

  it('integrates file selection with compression store', async () => {
    // 模拟文件选择组件
    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('test.txt', 1024)

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })

    await fileInput.trigger('change')

    // 验证文件被添加到store
    const store = useCompressionStore()
    expect(store.selectedFiles).toHaveLength(1)
  })

  it('integrates compression options with UI', async () => {
    // 查找压缩选项输入
    const passwordInput = wrapper.find('input[type="password"]')
    const compressionLevelSelect = wrapper.find('select[name="compression-level"]')
    const subdirectoriesCheckbox = wrapper.find('input[type="checkbox"]')

    // 更新选项
    await passwordInput.setValue('secret123')
    await compressionLevelSelect.setValue('9')
    await subdirectoriesCheckbox.setChecked(false)

    // 验证store更新
    const store = useCompressionStore()
    expect(store.compressionOptions.password).toBe('secret123')
    expect(store.compressionOptions.compressionLevel).toBe(9)
    expect(store.compressionOptions.createSubdirectories).toBe(false)
  })

  it('completes full compression workflow', async () => {
    // 1. 选择文件
    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('test.txt', 1024)

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })
    await fileInput.trigger('change')

    // 2. 设置选项
    const passwordInput = wrapper.find('input[type="password"]')
    await passwordInput.setValue('test123')

    // 3. 触发压缩
    mockInvoke.mockResolvedValue('压缩成功: output.zip')

    const compressButton = wrapper.find('button.compress-button')
    await compressButton.trigger('click')

    // 4. 验证结果
    expect(mockInvoke).toHaveBeenCalledWith('compress_file', expect.any(Array))

    const store = useCompressionStore()
    expect(store.isCompressing).toBe(false)
    expect(store.compressionHistory).toHaveLength(1)
  })

  it('handles extraction workflow', async () => {
    // 模拟选择压缩文件
    const fileInput = wrapper.find('input[type="file"]')
    const mockZipFile = createMockFile('archive.zip', 2048, 'application/zip')

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockZipFile],
    })
    await fileInput.trigger('change')

    // 设置密码
    const passwordInput = wrapper.find('input[type="password"]')
    await passwordInput.setValue('extract123')

    // 触发解压
    mockInvoke.mockResolvedValue('解压成功: /output/path')

    const extractButton = wrapper.find('button.extract-button')
    await extractButton.trigger('click')

    // 验证解压调用
    expect(mockInvoke).toHaveBeenCalledWith(
      'extract_file',
      expect.stringContaining('archive.zip'),
      expect.any(String),
      'extract123'
    )
  })

  it('displays compression progress', async () => {
    // 开始压缩
    const store = useCompressionStore()
    store.isCompressing = true

    await wrapper.vm.$nextTick()

    // 验证进度显示
    expect(wrapper.text()).toContain('压缩中')
    expect(wrapper.find('.progress-bar').exists()).toBe(true)

    // 完成压缩
    store.isCompressing = false
    await wrapper.vm.$nextTick()

    expect(wrapper.text()).not.toContain('压缩中')
  })

  it('shows compression history', async () => {
    const store = useCompressionStore()

    // 添加历史记录
    store.compressionHistory = [
      {
        id: '1',
        timestamp: new Date().toISOString(),
        operation: 'compress',
        files: ['file1.txt'],
        output: 'output.zip',
        status: 'success',
        size: 1024,
      },
      {
        id: '2',
        timestamp: new Date().toISOString(),
        operation: 'extract',
        files: ['archive.zip'],
        output: '/extracted',
        status: 'success',
        size: 2048,
      },
    ]

    await wrapper.vm.$nextTick()

    // 验证历史记录显示
    expect(wrapper.findAll('.history-item')).toHaveLength(2)
    expect(wrapper.text()).toContain('file1.txt')
    expect(wrapper.text()).toContain('archive.zip')
  })

  it('handles errors in compression workflow', async () => {
    // 模拟压缩失败
    mockInvoke.mockRejectedValue(new Error('压缩失败: 磁盘空间不足'))

    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('large.txt', 1024 * 1024 * 50) // 50MB

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile],
    })
    await fileInput.trigger('change')

    const compressButton = wrapper.find('button.compress-button')
    await compressButton.trigger('click')

    // 验证错误处理
    await wrapper.vm.$nextTick()

    const store = useCompressionStore()
    expect(store.compressionHistory[0].status).toBe('error')
    expect(wrapper.text()).toContain('错误')
  })

  it('validates file size before compression', async () => {
    const store = useCompressionStore()

    // 添加超大文件
    const largeFile = createMockFile('huge.iso', 1024 * 1024 * 1024 * 5) // 5GB
    store.addFile(largeFile)

    // 尝试压缩
    const canCompress = store.canCompress

    // 应该拒绝超大文件
    expect(canCompress).toBe(false)
    expect(wrapper.text()).toContain('文件太大')
  })

  it('integrates with system information', async () => {
    // Mock系统信息
    mockInvoke.mockResolvedValue({
      totalMemory: 8589934592, // 8GB
      usedMemory: 4294967296, // 4GB
      diskSpace: 500000000000, // 500GB
      cpuUsage: 45.5,
    })

    // 触发系统信息获取
    const systemInfoButton = wrapper.find('button.system-info')
    await systemInfoButton.trigger('click')

    // 验证系统信息显示
    await wrapper.vm.$nextTick()
    expect(wrapper.text()).toContain('内存')
    expect(wrapper.text()).toContain('磁盘')
  })
})

// 辅助函数
function useCompressionStore() {
  // 这里应该导入实际的store
  // 为了测试，我们返回一个模拟对象
  return {
    selectedFiles: [],
    compressionOptions: {
      password: '',
      compressionLevel: 6,
      createSubdirectories: true,
    },
    isCompressing: false,
    compressionHistory: [],
    addFile: vi.fn(),
    removeFile: vi.fn(),
    clearFiles: vi.fn(),
    updateOptions: vi.fn(),
    get totalFileSize() { return 0 },
    formatFileSize: vi.fn(),
    get isValidOptions() { return true },
    compressFiles: vi.fn(),
    extractFile: vi.fn(),
    getFilesByType: vi.fn(),
    addToHistory: vi.fn(),
    clearHistory: vi.fn(),
    get canCompress() { return true },
  }
}