/**
 * 前后端集成性能测试
 * 测试完整的压缩解压工作流程、任务管理和进度显示
 */

import { describe, it, expect, beforeAll, afterAll, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import App from '../../src/App.vue'
import { createMockFile, wait, measurePerformance } from '../fixtures/test_helpers'

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

// 性能监控工具
class PerformanceMonitor {
  private metrics: Map<string, any[]> = new Map()

  startMeasurement(name: string) {
    const startTime = performance.now()
    const startMemory = process.memoryUsage().heapUsed

    return {
      name,
      startTime,
      startMemory,
      end: () => {
        const endTime = performance.now()
        const endMemory = process.memoryUsage().heapUsed
        const duration = endTime - startTime
        const memoryUsed = endMemory - startMemory

        const metric = {
          duration,
          memoryUsed,
          timestamp: new Date().toISOString(),
        }

        if (!this.metrics.has(name)) {
          this.metrics.set(name, [])
        }
        this.metrics.get(name)!.push(metric)

        return metric
      }
    }
  }

  getMetrics(name: string) {
    return this.metrics.get(name) || []
  }

  getSummary() {
    const summary: Record<string, any> = {}

    for (const [name, metrics] of this.metrics.entries()) {
      if (metrics.length === 0) continue

      const durations = metrics.map(m => m.duration)
      const memoryUsages = metrics.map(m => m.memoryUsed)

      summary[name] = {
        count: metrics.length,
        avgDuration: durations.reduce((a, b) => a + b, 0) / durations.length,
        minDuration: Math.min(...durations),
        maxDuration: Math.max(...durations),
        avgMemory: memoryUsages.reduce((a, b) => a + b, 0) / memoryUsages.length,
        minMemory: Math.min(...memoryUsages),
        maxMemory: Math.max(...memoryUsages),
      }
    }

    return summary
  }

  clear() {
    this.metrics.clear()
  }
}

describe('前后端集成性能测试', () => {
  let wrapper: any
  let mockInvoke: any
  let performanceMonitor: PerformanceMonitor

  beforeAll(() => {
    performanceMonitor = new PerformanceMonitor()
  })

  beforeEach(() => {
    setActivePinia(createPinia())

    mockInvoke = vi.fn()
    vi.mocked(require('@tauri-apps/api').invoke).mockImplementation(mockInvoke)

    wrapper = mount(App)
    performanceMonitor.clear()
  })

  afterAll(() => {
    // 输出性能测试摘要
    const summary = performanceMonitor.getSummary()
    console.log('\n=== 性能测试摘要 ===')
    console.log(JSON.stringify(summary, null, 2))
  })

  describe('大文件压缩性能', () => {
    it('应该在合理时间内压缩大文件', async () => {
      const measurement = performanceMonitor.startMeasurement('large_file_compression')

      // 模拟大文件选择
      const fileInput = wrapper.find('input[type="file"]')
      const largeFile = createMockFile('large_data.bin', 100 * 1024 * 1024) // 100MB

      Object.defineProperty(fileInput.element, 'files', {
        value: [largeFile],
      })
      await fileInput.trigger('change')

      // 设置压缩选项
      const compressionLevelSelect = wrapper.find('select[name="compression-level"]')
      await compressionLevelSelect.setValue('6')

      // 模拟压缩过程
      mockInvoke.mockImplementation(async (command: string, args: any[]) => {
        if (command === 'compress_file') {
          // 模拟压缩时间
          await wait(2000) // 2秒压缩时间
          return {
            success: true,
            outputPath: '/test/output/large_data.zip',
            duration: 2000,
            originalSize: 100 * 1024 * 1024,
            compressedSize: 60 * 1024 * 1024,
            compressionRatio: 0.6,
          }
        }
        return null
      })

      // 触发压缩
      const compressButton = wrapper.find('button.compress-button')
      await compressButton.trigger('click')

      // 等待压缩完成
      await wait(2500)

      const metric = measurement.end()

      // 验证性能指标
      expect(metric.duration).toBeLessThan(5000) // 总时间应小于5秒
      expect(metric.memoryUsed).toBeLessThan(200 * 1024 * 1024) // 内存使用应小于200MB

      // 验证压缩结果
      expect(mockInvoke).toHaveBeenCalledWith('compress_file', expect.any(Array))
      const store = useCompressionStore()
      expect(store.compressionHistory).toHaveLength(1)
      expect(store.compressionHistory[0].status).toBe('success')
    })

    it('应该实时显示压缩进度', async () => {
      const measurement = performanceMonitor.startMeasurement('compression_progress')

      // 模拟带进度更新的压缩
      let progressCallback: any = null
      mockInvoke.mockImplementation(async (command: string, args: any[]) => {
        if (command === 'compress_file_with_progress') {
          // 模拟进度更新
          for (let i = 0; i <= 100; i += 10) {
            await wait(100)
            if (progressCallback) {
              progressCallback({
                currentFile: 'large_data.bin',
                progress: i,
                bytesProcessed: (i / 100) * 100 * 1024 * 1024,
                totalBytes: 100 * 1024 * 1024,
              })
            }
          }
          return { success: true, outputPath: '/test/output/progress_test.zip' }
        }
        return null
      })

      // 设置进度监听
      vi.mocked(require('@tauri-apps/api').window.appWindow.listen)
        .mockImplementation((event: string, callback: any) => {
          if (event === 'compression_progress') {
            progressCallback = callback
          }
        })

      // 触发压缩
      const fileInput = wrapper.find('input[type="file"]')
      const testFile = createMockFile('progress_test.bin', 50 * 1024 * 1024)

      Object.defineProperty(fileInput.element, 'files', {
        value: [testFile],
      })
      await fileInput.trigger('change')

      const compressButton = wrapper.find('button.compress-button')
      await compressButton.trigger('click')

      // 验证进度显示
      await wait(500)
      expect(wrapper.text()).toContain('压缩中')
      expect(wrapper.find('.progress-bar').exists()).toBe(true)

      // 等待完成
      await wait(1500)

      const metric = measurement.end()
      expect(metric.duration).toBeLessThan(3000) // 进度更新不应显著增加总时间
    })
  })

  describe('并发任务处理性能', () => {
    it('应该高效处理多个并发压缩任务', async () => {
      const measurement = performanceMonitor.startMeasurement('concurrent_compression')

      const fileCount = 5
      const fileSize = 20 * 1024 * 1024 // 20MB每个

      // 模拟多个文件选择
      const fileInput = wrapper.find('input[type="file"]')
      const files = Array(fileCount).fill(0).map((_, i) =>
        createMockFile(`concurrent_${i}.bin`, fileSize)
      )

      Object.defineProperty(fileInput.element, 'files', {
        value: files,
      })
      await fileInput.trigger('change')

      // 模拟并发压缩
      let completedTasks = 0
      mockInvoke.mockImplementation(async (command: string, args: any[]) => {
        if (command === 'compress_file') {
          // 每个任务模拟1秒压缩时间
          await wait(1000)
          completedTasks++
          return {
            success: true,
            outputPath: `/test/output/concurrent_${completedTasks}.zip`,
            duration: 1000,
          }
        }
        return null
      })

      // 触发压缩
      const startTime = performance.now()
      const compressButton = wrapper.find('button.compress-button')
      await compressButton.trigger('click')

      // 等待所有任务完成
      while (completedTasks < fileCount) {
        await wait(100)
      }

      const totalTime = performance.now() - startTime
      const metric = measurement.end()

      // 验证并发性能
      // 理想情况下，5个1秒的任务并发执行应该接近1秒
      expect(totalTime).toBeLessThan(3000) // 应明显小于5秒
      expect(metric.duration).toBeLessThan(3500) // 包括测试开销

      // 验证所有任务完成
      const store = useCompressionStore()
      expect(store.compressionHistory).toHaveLength(fileCount)
      expect(store.compressionHistory.every((h: any) => h.status === 'success')).toBe(true)
    })

    it('应该合理管理并发任务资源', async () => {
      const measurement = performanceMonitor.startMeasurement('concurrent_resource_management')

      // 模拟资源监控
      const resourceUsage: any[] = []
      const monitorInterval = setInterval(() => {
        resourceUsage.push({
          timestamp: Date.now(),
          memory: process.memoryUsage().heapUsed,
        })
      }, 100)

      // 执行并发任务
      const fileInput = wrapper.find('input[type="file"]')
      const files = Array(10).fill(0).map((_, i) =>
        createMockFile(`resource_test_${i}.bin`, 10 * 1024 * 1024)
      )

      Object.defineProperty(fileInput.element, 'files', {
        value: files,
      })
      await fileInput.trigger('change')

      mockInvoke.mockImplementation(async (command: string) => {
        if (command === 'compress_file') {
          await wait(500)
          return { success: true }
        }
        return null
      })

      const compressButton = wrapper.find('button.compress-button')
      await compressButton.trigger('click')

      // 等待任务完成
      await wait(3000)

      clearInterval(monitorInterval)
      const metric = measurement.end()

      // 分析资源使用
      const memoryValues = resourceUsage.map(r => r.memory)
      const maxMemory = Math.max(...memoryValues)
      const minMemory = Math.min(...memoryValues)
      const memoryRange = maxMemory - minMemory

      // 内存使用应该相对稳定
      expect(memoryRange).toBeLessThan(100 * 1024 * 1024) // 内存波动应小于100MB
      expect(metric.memoryUsed).toBeLessThan(150 * 1024 * 1024) // 总内存增长应有限
    })
  })

  describe('解压性能测试', () => {
    it('应该高效解压大ZIP文件', async () => {
      const measurement = performanceMonitor.startMeasurement('large_zip_extraction')

      // 模拟ZIP文件选择
      const fileInput = wrapper.find('input[type="file"]')
      const zipFile = createMockFile('large_archive.zip', 200 * 1024 * 1024, 'application/zip')

      Object.defineProperty(fileInput.element, 'files', {
        value: [zipFile],
      })
      await fileInput.trigger('change')

      // 切换到解压模式
      const extractTab = wrapper.find('[data-testid="extract-tab"]')
      await extractTab.trigger('click')

      // 模拟解压过程
      mockInvoke.mockImplementation(async (command: string, archivePath: string) => {
        if (command === 'extract_file') {
          // 模拟解压时间（与文件大小相关）
          const size = 200 * 1024 * 1024
          const simulatedTime = size / (50 * 1024 * 1024) * 1000 // 假设50MB/秒
          await wait(simulatedTime)
          return {
            success: true,
            outputDir: '/test/extracted/large_archive',
            extractedFiles: 100,
            totalSize: size,
            duration: simulatedTime,
          }
        }
        return null
      })

      // 触发解压
      const extractButton = wrapper.find('button.extract-button')
      await extractButton.trigger('click')

      // 等待解压完成
      await wait(5000)

      const metric = measurement.end()

      // 验证性能指标
      expect(metric.duration).toBeLessThan(10000) // 解压时间应合理
      expect(mockInvoke).toHaveBeenCalledWith('extract_file', expect.any(String), expect.any(String))

      const store = useCompressionStore()
      expect(store.compressionHistory).toHaveLength(1)
      expect(store.compressionHistory[0].operation).toBe('extract')
    })

    it('应该支持带密码的加密ZIP解压', async () => {
      const measurement = performanceMonitor.startMeasurement('encrypted_zip_extraction')

      // 选择加密ZIP文件
      const fileInput = wrapper.find('input[type="file"]')
      const encryptedZip = createMockFile('encrypted.zip', 50 * 1024 * 1024, 'application/zip')

      Object.defineProperty(fileInput.element, 'files', {
        value: [encryptedZip],
      })
      await fileInput.trigger('change')

      // 输入密码
      const passwordInput = wrapper.find('input[type="password"]')
      await passwordInput.setValue('securepassword123')

      // 模拟加密解压（应该比普通解压稍慢）
      let extractionStartTime: number
      mockInvoke.mockImplementation(async (command: string) => {
        if (command === 'extract_file') {
          extractionStartTime = performance.now()
          // 加密解压模拟更长时间
          await wait(1500)
          return {
            success: true,
            outputDir: '/test/extracted/encrypted',
            duration: 1500,
          }
        }
        return null
      })

      const extractButton = wrapper.find('button.extract-button')
      await extractButton.trigger('click')

      await wait(2000)
      const metric = measurement.end()

      // 验证加密解压
      expect(metric.duration).toBeGreaterThan(1000) // 加密解压应有合理时间
      expect(mockInvoke).toHaveBeenCalledWith(
        'extract_file',
        expect.any(String),
        expect.any(String),
        'securepassword123'
      )
    })
  })

  describe('内存使用效率', () => {
    it('应该在长时间运行中保持内存稳定', async () => {
      const measurement = performanceMonitor.startMeasurement('memory_stability_long_run')

      // 模拟长时间运行（多个压缩/解压循环）
      const cycles = 10
      const memoryReadings: number[] = []

      mockInvoke.mockImplementation(async (command: string) => {
        if (command === 'compress_file' || command === 'extract_file') {
          // 记录内存使用
          memoryReadings.push(process.memoryUsage().heapUsed)
          await wait(300)
          return { success: true }
        }
        return null
      })

      for (let i = 0; i < cycles; i++) {
        // 交替进行压缩和解压
        if (i % 2 === 0) {
          // 压缩
          const fileInput = wrapper.find('input[type="file"]')
          const file = createMockFile(`cycle_${i}.bin`, 5 * 1024 * 1024)

          Object.defineProperty(fileInput.element, 'files', {
            value: [file],
          })
          await fileInput.trigger('change')

          const compressButton = wrapper.find('button.compress-button')
          await compressButton.trigger('click')
        } else {
          // 解压
          const fileInput = wrapper.find('input[type="file"]')
          const zipFile = createMockFile(`cycle_${i}.zip`, 5 * 1024 * 1024, 'application/zip')

          Object.defineProperty(fileInput.element, 'files', {
            value: [zipFile],
          })
          await fileInput.trigger('change')

          const extractTab = wrapper.find('[data-testid="extract-tab"]')
          await extractTab.trigger('click')

          const extractButton = wrapper.find('button.extract-button')
          await extractButton.trigger('click')
        }

        await wait(500)
      }

      // 等待所有操作完成
      await wait(2000)
      const metric = measurement.end()

      // 分析内存稳定性
      if (memoryReadings.length > 1) {
        const initialMemory = memoryReadings[0]
        const finalMemory = memoryReadings[memoryReadings.length - 1]
        const memoryIncrease = finalMemory - initialMemory

        // 内存增长应该很小
        expect(memoryIncrease).toBeLessThan(50 * 1024 * 1024) // 增长应小于50MB

        // 计算内存波动
        const memoryChanges = []
        for (let i = 1; i < memoryReadings.length; i++) {
          memoryChanges.push(Math.abs(memoryReadings[i] - memoryReadings[i - 1]))
        }
        const avgChange = memoryChanges.reduce((a, b) => a + b, 0) / memoryChanges.length

        expect(avgChange).toBeLessThan(10 * 1024 * 1024) // 平均波动应小于10MB
      }
    })

    it('应该正确处理内存压力场景', async () => {
      const measurement = performanceMonitor.startMeasurement('memory_pressure_handling')

      // 模拟内存压力（大文件处理）
      const largeFiles = Array(3).fill(0).map((_, i) =>
        createMockFile(`memory_pressure_${i}.bin`, 100 * 1024 * 1024)
      )

      const fileInput = wrapper.find('input[type="file"]')
      Object.defineProperty(fileInput.element, 'files', {
        value: largeFiles,
      })
      await fileInput.trigger('change')

      // 模拟内存监控
      let peakMemory = 0
      const originalInvoke = mockInvoke
      mockInvoke.mockImplementation(async (...args: any[]) => {
        const currentMemory = process.memoryUsage().heapUsed
        peakMemory = Math.max(peakMemory, currentMemory)

        // 模拟压缩需要内存
        await wait(1000)
        return {
          success: true,
          memoryUsed: currentMemory,
        }
      })

      const compressButton = wrapper.find('button.compress-button')
      await compressButton.trigger('click')

      await wait(4000)
      const metric = measurement.end()

      // 验证内存使用在合理范围内
      expect(peakMemory).toBeLessThan(500 * 1024 * 1024) // 峰值内存应小于500MB
      expect(metric.memoryUsed).toBeLessThan(300 * 1024 * 1024) // 内存增长应有限
    })
  })

  describe('错误处理和恢复性能', () => {
    it('应该快速处理压缩失败并恢复', async () => {
      const measurement = performanceMonitor.startMeasurement('error_recovery_performance')

      // 第一次压缩失败
      mockInvoke.mockImplementationOnce(async () => {
        await wait(500)
        throw new Error('压缩失败: 磁盘空间不足')
      }).mockImplementationOnce(async () => {
        await wait(1000)
        return { success: true, outputPath: '/test/output/recovery.zip' }
      })

      const fileInput = wrapper.find('input[type="file"]')
      const testFile = createMockFile('recovery_test.bin', 10 * 1024 * 1024)

      Object.defineProperty(fileInput.element, 'files', {
        value: [testFile],
      })
      await fileInput.trigger('change')

      const compressButton = wrapper.find('button.compress-button')

      // 第一次尝试（应该失败）
      await compressButton.trigger('click')
      await wait(1000)

      // 验证错误显示
      expect(wrapper.text()).toContain('错误')
      expect(wrapper.text()).toContain('磁盘空间不足')

      // 第二次尝试（应该成功）
      await compressButton.trigger('click')
      await wait(1500)

      const metric = measurement.end()

      // 验证恢复性能
      expect(metric.duration).toBeLessThan(3000) // 包括错误处理和重试
      expect(mockInvoke).toHaveBeenCalledTimes(2)

      const store = useCompressionStore()
      expect(store.compressionHistory).toHaveLength(2)
      expect(store.compressionHistory[0].status).toBe('error')
      expect(store.compressionHistory[1].status).toBe('success')
    })

    it('应该优雅处理取消操作', async () => {
      const measurement = performanceMonitor.startMeasurement('cancel_operation_performance')

      // 模拟可取消的长时间操作
      let shouldCancel = false
      mockInvoke.mockImplementation(async () => {
        for (let i = 0; i < 10; i++) {
          if (shouldCancel) {
            throw new Error('操作已取消')
          }
          await wait(200)
        }
        return { success: true }
      })

      const fileInput = wrapper.find('input[type="file"]')
      const largeFile = createMockFile('cancel_test.bin', 50 * 1024 * 1024)

      Object.defineProperty(fileInput.element, 'files', {
        value: [largeFile],
      })
      await fileInput.trigger('change')

      const compressButton = wrapper.find('button.compress-button')
      await compressButton.trigger('click')

      // 等待一段时间后取消
      await wait(500)
      shouldCancel = true

      // 触发取消
      const cancelButton = wrapper.find('button.cancel-button')
      if (cancelButton.exists()) {
        await cancelButton.trigger('click')
      }

      await wait(1000)
      const metric = measurement.end()

      // 验证取消性能
      expect(metric.duration).toBeLessThan(2000) // 取消应快速响应
      expect(wrapper.text()).toContain('已取消')
    })
  })

  describe('用户界面响应性能', () => {
    it('应该保持UI在操作期间的响应性', async () => {
      const measurement = performanceMonitor.startMeasurement('ui_responsiveness')

      // 模拟长时间操作，但UI应保持响应
      let uiUpdates = 0
      const uiUpdateListener = vi.fn(() => {
        uiUpdates++
      })

      // 监听UI更新
      wrapper.vm.$watch(() => wrapper.vm.someReactiveProperty, uiUpdateListener)

      mockInvoke.mockImplementation(async () => {
        // 长时间操作，但定期更新进度
        for (let i = 0; i <= 100; i += 20) {
          await wait(200)
          // 触发UI更新
          wrapper.vm.someReactiveProperty = i
        }
        return { success: true }
      })

      const fileInput = wrapper.find('input[type="file"]')
      const testFile = createMockFile('ui_test.bin', 30 * 1024 * 1024)

      Object.defineProperty(fileInput.element, 'files', {
        value: [testFile],
      })
      await fileInput.trigger('change')

      const compressButton = wrapper.find('button.compress-button')
      await compressButton.trigger('click')

      // 在操作期间尝试与UI交互
      await wait(300)
      const otherButton = wrapper.find('button.some-other-button')
      if (otherButton.exists()) {
        await otherButton.trigger('click')
        // UI应该响应
        expect(wrapper.text()).not.toContain('无响应')
      }

      await wait(1500)
      const metric = measurement.end()

      // 验证UI响应性
      expect(uiUpdates).toBeGreaterThan(0) // UI应该有更新
      expect(metric.duration).toBeLessThan(3000)

      // UI应该没有冻结迹象
      expect(wrapper.text()).not.toContain('冻结')
      expect(wrapper.text()).not.toContain('无响应')
    })

    it('应该快速更新任务列表和状态', async () => {
      const measurement = performanceMonitor.startMeasurement('task_list_performance')

      const taskCount = 20
      const store = useCompressionStore()

      // 快速添加多个任务
      const startTime = performance.now()
      for (let i = 0; i < taskCount; i++) {
        store.addToHistory({
          id: `task_${i}`,
          timestamp: new Date().toISOString(),
          operation: i % 2 === 0 ? 'compress' : 'extract',
          files: [`file_${i}.txt`],
          status: 'pending',
          size: 1024 * 1024,
        })

        // 快速更新状态
        await wait(10)
        store.updateTaskStatus(`task_${i}`, 'processing')
        await wait(10)
        store.updateTaskStatus(`task_${i}`, 'completed')
      }
      const endTime = performance.now()

      const metric = measurement.end()

      // 验证任务列表性能
      expect(endTime - startTime).toBeLessThan(1000) // 添加20个任务应很快
      expect(store.compressionHistory).toHaveLength(taskCount)

      // UI应该能处理这么多任务
      await wrapper.vm.$nextTick()
      expect(wrapper.findAll('.task-item').length).toBe(taskCount)

      // 滚动性能（如果实现）
      const taskList = wrapper.find('.task-list')
      if (taskList.exists()) {
        // 模拟滚动
        taskList.element.scrollTop = 1000
        await wrapper.vm.$nextTick()
        // 应该没有明显延迟
      }
    })
  })
})

// 辅助函数和模拟store
function useCompressionStore() {
  // 模拟压缩store
  return {
    selectedFiles: [],
    compressionOptions: {
      password: '',
      compressionLevel: 6,
      createSubdirectories: true,
    },
    isCompressing: false,
    isExtracting: false,
    compressionHistory: [] as any[],
    addFile: vi.fn(),
    removeFile: vi.fn(),
    clearFiles: vi.fn(),
    updateOptions: vi.fn(),
    compressFiles: vi.fn(),
    extractFile: vi.fn(),
    addToHistory: vi.fn((entry: any) => {
      // 模拟添加历史记录
      const store = useCompressionStore()
      store.compressionHistory.push(entry)
    }),
    updateTaskStatus: vi.fn((taskId: string, status: string) => {
      // 模拟更新任务状态
      const store = useCompressionStore()
      const task = store.compressionHistory.find((h: any) => h.id === taskId)
      if (task) {
        task.status = status
      }
    }),
    get totalFileSize() { return 0 },
    formatFileSize: vi.fn((size: number) => `${(size / 1024 / 1024).toFixed(1)} MB`),
    get isValidOptions() { return true },
    getFilesByType: vi.fn(),
    clearHistory: vi.fn(),
    get canCompress() { return true },
  }
}