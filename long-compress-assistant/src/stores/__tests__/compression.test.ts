import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useCompressionStore } from '../compression'
import { createMockFile, createMockZipFile } from '../../../tests/setup'

// Mock Tauri API
vi.mock('@tauri-apps/api', () => ({
  invoke: vi.fn(),
}))

describe('Compression Store', () => {
  let store: any

  beforeEach(() => {
    setActivePinia(createPinia())
    store = useCompressionStore()
  })

  it('initializes with default state', () => {
    expect(store.selectedFiles).toEqual([])
    expect(store.compressionOptions).toEqual({
      password: '',
      compressionLevel: 6,
      createSubdirectories: true,
    })
    expect(store.isCompressing).toBe(false)
    expect(store.compressionHistory).toEqual([])
  })

  it('adds files to selection', () => {
    const mockFile = createMockFile('test.txt', 1024)
    store.addFile(mockFile)

    expect(store.selectedFiles).toHaveLength(1)
    expect(store.selectedFiles[0].name).toBe('test.txt')
  })

  it('removes files from selection', () => {
    const file1 = createMockFile('file1.txt', 1024)
    const file2 = createMockFile('file2.txt', 2048)

    store.addFile(file1)
    store.addFile(file2)
    expect(store.selectedFiles).toHaveLength(2)

    store.removeFile(0)
    expect(store.selectedFiles).toHaveLength(1)
    expect(store.selectedFiles[0].name).toBe('file2.txt')
  })

  it('clears all files', () => {
    const file1 = createMockFile('file1.txt', 1024)
    const file2 = createMockFile('file2.txt', 2048)

    store.addFile(file1)
    store.addFile(file2)
    expect(store.selectedFiles).toHaveLength(2)

    store.clearFiles()
    expect(store.selectedFiles).toHaveLength(0)
  })

  it('updates compression options', () => {
    const newOptions = {
      password: 'secret123',
      compressionLevel: 9,
      createSubdirectories: false,
    }

    store.updateOptions(newOptions)

    expect(store.compressionOptions).toEqual(newOptions)
  })

  it('calculates total file size', () => {
    const file1 = createMockFile('file1.txt', 1024)
    const file2 = createMockFile('file2.txt', 2048)
    const file3 = createMockFile('file3.txt', 3072)

    store.addFile(file1)
    store.addFile(file2)
    store.addFile(file3)

    expect(store.totalFileSize).toBe(6144) // 1024 + 2048 + 3072
  })

  it('formats file size for display', () => {
    expect(store.formatFileSize(1024)).toBe('1 KB')
    expect(store.formatFileSize(1048576)).toBe('1 MB')
    expect(store.formatFileSize(1073741824)).toBe('1 GB')
    expect(store.formatFileSize(500)).toBe('500 B')
  })

  it('validates compression options', () => {
    // 测试有效选项
    store.updateOptions({
      password: 'valid123',
      compressionLevel: 6,
      createSubdirectories: true,
    })

    expect(store.isValidOptions).toBe(true)

    // 测试无效压缩级别
    store.updateOptions({
      password: '',
      compressionLevel: 12, // 超出范围
      createSubdirectories: true,
    })

    expect(store.isValidOptions).toBe(false)
  })

  it('handles compression process', async () => {
    const mockFile = createMockFile('test.txt', 1024)
    store.addFile(mockFile)

    // Mock Tauri invoke response
    const mockInvoke = vi.fn().mockResolvedValue('压缩成功: output.zip')
    vi.mocked(require('@tauri-apps/api').invoke).mockImplementation(mockInvoke)

    await store.compressFiles('output.zip')

    expect(store.isCompressing).toBe(false)
    expect(store.compressionHistory).toHaveLength(1)
    expect(store.compressionHistory[0].status).toBe('success')
  })

  it('handles compression failure', async () => {
    const mockFile = createMockFile('test.txt', 1024)
    store.addFile(mockFile)

    // Mock Tauri invoke error
    const mockInvoke = vi.fn().mockRejectedValue(new Error('压缩失败'))
    vi.mocked(require('@tauri-apps/api').invoke).mockImplementation(mockInvoke)

    await store.compressFiles('output.zip')

    expect(store.isCompressing).toBe(false)
    expect(store.compressionHistory).toHaveLength(1)
    expect(store.compressionHistory[0].status).toBe('error')
  })

  it('handles extraction process', async () => {
    const mockZipFile = createMockZipFile()

    // Mock Tauri invoke response
    const mockInvoke = vi.fn().mockResolvedValue('解压成功: /output/path')
    vi.mocked(require('@tauri-apps/api').invoke).mockImplementation(mockInvoke)

    const result = await store.extractFile('test.zip', '/output/path', 'password123')

    expect(result).toBe('解压成功: /output/path')
    expect(store.compressionHistory).toHaveLength(1)
  })

  it('filters files by type', () => {
    const files = [
      createMockFile('test.txt', 1024, 'text/plain'),
      createMockFile('test.zip', 2048, 'application/zip'),
      createMockFile('test.pdf', 3072, 'application/pdf'),
    ]

    files.forEach(file => store.addFile(file))

    const zipFiles = store.getFilesByType('application/zip')
    expect(zipFiles).toHaveLength(1)
    expect(zipFiles[0].name).toBe('test.zip')

    const textFiles = store.getFilesByType('text/plain')
    expect(textFiles).toHaveLength(1)
    expect(textFiles[0].name).toBe('test.txt')
  })

  it('manages compression history', () => {
    const historyItem = {
      id: '1',
      timestamp: new Date().toISOString(),
      operation: 'compress',
      files: ['test.txt'],
      output: 'output.zip',
      status: 'success',
      size: 1024,
    }

    store.addToHistory(historyItem)
    expect(store.compressionHistory).toHaveLength(1)
    expect(store.compressionHistory[0].id).toBe('1')

    store.clearHistory()
    expect(store.compressionHistory).toHaveLength(0)
  })

  it('persists state to localStorage', () => {
    const mockFile = createMockFile('test.txt', 1024)
    store.addFile(mockFile)

    store.updateOptions({
      password: 'test123',
      compressionLevel: 8,
      createSubdirectories: false,
    })

    // 验证状态可以被序列化
    const serialized = JSON.stringify(store.$state)
    expect(serialized).toContain('test.txt')
    expect(serialized).toContain('test123')
  })
})