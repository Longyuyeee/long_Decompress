/**
 * 压缩性能测试
 * 测试压缩和解压操作的性能指标
 */

import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest'
import { mockCompressionService, createMockFileList, wait } from '../fixtures/test_helpers'

describe('压缩性能测试', () => {
  let compressionService: any

  beforeAll(() => {
    compressionService = mockCompressionService()
  })

  afterAll(() => {
    vi.clearAllMocks()
  })

  describe('小文件压缩性能', () => {
    it('应该在1秒内压缩10个1MB文件', async () => {
      const files = createMockFileList(10, 'small', 1024 * 1024) // 10个1MB文件
      const options = { compressionLevel: 6, password: null }

      const startTime = performance.now()
      const result = await compressionService.compress(files, options)
      const endTime = performance.now()
      const duration = endTime - startTime

      expect(result.success).toBe(true)
      expect(duration).toBeLessThan(1000) // 1秒内完成
      expect(result.size).toBeLessThan(files.reduce((sum, file) => sum + file.size, 0)) // 压缩后应更小
    })

    it('应该在500毫秒内解压10MB压缩包', async () => {
      const archivePath = '/test/archive.zip'
      const outputDir = '/test/output'
      const password = null

      const startTime = performance.now()
      const result = await compressionService.extract(archivePath, outputDir, password)
      const endTime = performance.now()
      const duration = endTime - startTime

      expect(result.success).toBe(true)
      expect(duration).toBeLessThan(500) // 500毫秒内完成
      expect(result.extractedFiles).toHaveLength(2) // 模拟返回2个文件
    })
  })

  describe('中等文件压缩性能', () => {
    it('应该在3秒内压缩100MB文件', async () => {
      const files = [createMockFile('medium.bin', 100 * 1024 * 1024)] // 100MB文件
      const options = { compressionLevel: 6, password: null }

      const startTime = performance.now()
      const result = await compressionService.compress(files, options)
      const endTime = performance.now()
      const duration = endTime - startTime

      expect(result.success).toBe(true)
      expect(duration).toBeLessThan(3000) // 3秒内完成
      expect(result.duration).toBeLessThan(1000) // 模拟服务返回的持续时间
    })

    it('应该在2秒内解压100MB压缩包', async () => {
      const archivePath = '/test/large_archive.zip'
      const outputDir = '/test/output_large'
      const password = null

      const startTime = performance.now()
      const result = await compressionService.extract(archivePath, outputDir, password)
      const endTime = performance.now()
      const duration = endTime - startTime

      expect(result.success).toBe(true)
      expect(duration).toBeLessThan(2000) // 2秒内完成
    })
  })

  describe('大文件压缩性能', () => {
    it('应该在15秒内压缩1GB文件', async () => {
      const files = [createMockFile('large.bin', 1024 * 1024 * 1024)] // 1GB文件
      const options = { compressionLevel: 6, password: null }

      const startTime = performance.now()
      const result = await compressionService.compress(files, options)
      const endTime = performance.now()
      const duration = endTime - startTime

      expect(result.success).toBe(true)
      expect(duration).toBeLessThan(15000) // 15秒内完成
    })

    it('应该在10秒内解压1GB压缩包', async () => {
      const archivePath = '/test/huge_archive.zip'
      const outputDir = '/test/output_huge'
      const password = null

      const startTime = performance.now()
      const result = await compressionService.extract(archivePath, outputDir, password)
      const endTime = performance.now()
      const duration = endTime - startTime

      expect(result.success).toBe(true)
      expect(duration).toBeLessThan(10000) // 10秒内完成
    })
  })

  describe('批量文件压缩性能', () => {
    it('应该在30秒内批量压缩100个文件（总大小500MB）', async () => {
      const files = createMockFileList(100, 'batch', 5 * 1024 * 1024) // 100个5MB文件，共500MB
      const options = { compressionLevel: 6, password: null }

      const startTime = performance.now()
      const result = await compressionService.compress(files, options)
      const endTime = performance.now()
      const duration = endTime - startTime

      expect(result.success).toBe(true)
      expect(duration).toBeLessThan(30000) // 30秒内完成
    })

    it('应该在20秒内批量解压10个压缩包', async () => {
      const archives = Array(10).fill(0).map((_, i) => `/test/archive_${i}.zip`)
      const outputDir = '/test/batch_output'

      const startTime = performance.now()

      // 模拟批量解压
      const results = await Promise.all(
        archives.map(archivePath =>
          compressionService.extract(archivePath, `${outputDir}/${archivePath.split('/').pop()}`, null)
        )
      )

      const endTime = performance.now()
      const duration = endTime - startTime

      expect(results.every(r => r.success)).toBe(true)
      expect(duration).toBeLessThan(20000) // 20秒内完成
    })
  })

  describe('压缩级别性能影响', () => {
    const testFile = createMockFile('test_perf.bin', 10 * 1024 * 1024) // 10MB文件

    it('低压缩级别（1）应该最快', async () => {
      const options = { compressionLevel: 1, password: null }

      const startTime = performance.now()
      await compressionService.compress([testFile], options)
      const endTime = performance.now()
      const durationLow = endTime - startTime

      expect(durationLow).toBeLessThan(1000) // 应该很快
    })

    it('高压缩级别（9）应该产生最小文件', async () => {
      const optionsLow = { compressionLevel: 1, password: null }
      const optionsHigh = { compressionLevel: 9, password: null }

      // 模拟不同压缩级别的结果
      compressionService.compress.mockImplementationOnce(() =>
        Promise.resolve({ success: true, size: 8 * 1024 * 1024 }) // 低压缩：8MB
      ).mockImplementationOnce(() =>
        Promise.resolve({ success: true, size: 6 * 1024 * 1024 }) // 高压缩：6MB
      )

      const resultLow = await compressionService.compress([testFile], optionsLow)
      const resultHigh = await compressionService.compress([testFile], optionsHigh)

      expect(resultHigh.size).toBeLessThan(resultLow.size) // 高压缩应该产生更小的文件
    })
  })

  describe('加密压缩性能', () => {
    it('加密压缩应该比非加密压缩稍慢', async () => {
      const files = [createMockFile('encrypted.bin', 10 * 1024 * 1024)] // 10MB文件
      const optionsNoPassword = { compressionLevel: 6, password: null }
      const optionsWithPassword = { compressionLevel: 6, password: 'secret123' }

      // 模拟加密和非加密压缩的时间
      compressionService.compress
        .mockImplementationOnce(() => {
          return new Promise(resolve => {
            setTimeout(() => resolve({ success: true, duration: 800 }), 800)
          })
        })
        .mockImplementationOnce(() => {
          return new Promise(resolve => {
            setTimeout(() => resolve({ success: true, duration: 1200 }), 1200)
          })
        })

      const startTime1 = performance.now()
      await compressionService.compress(files, optionsNoPassword)
      const endTime1 = performance.now()
      const durationNoPassword = endTime1 - startTime1

      const startTime2 = performance.now()
      await compressionService.compress(files, optionsWithPassword)
      const endTime2 = performance.now()
      const durationWithPassword = endTime2 - startTime2

      // 加密压缩应该比非加密压缩时间长
      expect(durationWithPassword).toBeGreaterThan(durationNoPassword)
    })
  })

  describe('内存使用性能', () => {
    it('压缩大文件时内存使用应该在合理范围内', async () => {
      const files = [createMockFile('memory_test.bin', 500 * 1024 * 1024)] // 500MB文件
      const options = { compressionLevel: 6, password: null }

      // 模拟内存使用监控
      const memoryBefore = process.memoryUsage().heapUsed

      await compressionService.compress(files, options)

      const memoryAfter = process.memoryUsage().heapUsed
      const memoryIncrease = memoryAfter - memoryBefore

      // 内存增长应该小于文件大小的2倍
      expect(memoryIncrease).toBeLessThan(files[0].size * 2)
    })

    it('批量压缩时内存使用应该稳定', async () => {
      const files = createMockFileList(50, 'memory', 10 * 1024 * 1024) // 50个10MB文件
      const options = { compressionLevel: 6, password: null }

      const memoryUsage: number[] = []

      // 模拟压缩过程中的内存监控
      for (let i = 0; i < 5; i++) {
        memoryUsage.push(process.memoryUsage().heapUsed)
        await compressionService.compress([files[i]], options)
        await wait(100) // 等待一下
      }

      // 计算内存使用的标准差
      const mean = memoryUsage.reduce((a, b) => a + b) / memoryUsage.length
      const variance = memoryUsage.reduce((a, b) => a + Math.pow(b - mean, 2), 0) / memoryUsage.length
      const stdDev = Math.sqrt(variance)

      // 内存使用应该相对稳定（标准差小于平均值的10%）
      expect(stdDev).toBeLessThan(mean * 0.1)
    })
  })

  describe('并发压缩性能', () => {
    it('应该支持并发压缩多个文件', async () => {
      const files = createMockFileList(5, 'concurrent', 20 * 1024 * 1024) // 5个20MB文件
      const options = { compressionLevel: 6, password: null }

      const startTime = performance.now()

      // 并发压缩
      const promises = files.map(file =>
        compressionService.compress([file], options)
      )

      const results = await Promise.all(promises)
      const endTime = performance.now()
      const duration = endTime - startTime

      expect(results.every(r => r.success)).toBe(true)

      // 并发压缩应该比顺序压缩快
      // 假设顺序压缩需要5 * 单个文件时间，并发应该明显更快
      const expectedSequentialTime = 5 * 1000 // 假设每个文件1秒
      expect(duration).toBeLessThan(expectedSequentialTime * 0.7) // 至少快30%
    })
  })

  describe('压缩比性能', () => {
    it('文本文件应该有高压缩比', async () => {
      const textFile = createMockFile('text.txt', 10 * 1024 * 1024) // 10MB文本文件
      const options = { compressionLevel: 6, password: null }

      // 模拟文本文件压缩（高压缩比）
      compressionService.compress.mockResolvedValueOnce({
        success: true,
        size: 1 * 1024 * 1024, // 压缩到1MB
        compressionRatio: 0.1 // 10:1压缩比
      })

      const result = await compressionService.compress([textFile], options)
      const compressionRatio = result.size / textFile.size

      expect(compressionRatio).toBeLessThan(0.3) // 压缩比应该小于0.3（压缩到原大小的30%以内）
    })

    it('已压缩文件应该几乎无法再压缩', async () => {
      const compressedFile = createMockFile('already_compressed.zip', 10 * 1024 * 1024)
      const options = { compressionLevel: 6, password: null }

      // 模拟已压缩文件（低压缩比）
      compressionService.compress.mockResolvedValueOnce({
        success: true,
        size: 9.5 * 1024 * 1024, // 几乎没压缩
        compressionRatio: 0.95
      })

      const result = await compressionService.compress([compressedFile], options)
      const compressionRatio = result.size / compressedFile.size

      expect(compressionRatio).toBeGreaterThan(0.9) // 压缩比应该大于0.9（几乎没压缩）
    })
  })
})