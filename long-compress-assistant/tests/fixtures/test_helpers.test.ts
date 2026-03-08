/**
 * 测试辅助函数的测试
 * 验证测试工具函数正常工作
 */

import { describe, it, expect, vi } from 'vitest'
import {
  createMockFile,
  createMockArchiveFile,
  createMockFileList,
  mockTauriInvoke,
  mockFileSystem,
  mockDatabase,
  mockCompressionService,
  mockPasswordManager,
  wait,
  generateTestId,
  validateFile,
  cleanupTestEnvironment
} from './test_helpers'

describe('测试辅助函数', () => {
  afterEach(() => {
    cleanupTestEnvironment()
  })

  describe('createMockFile', () => {
    it('应该创建模拟文件对象', () => {
      const file = createMockFile('test.txt', 1024, 'text/plain')

      expect(file).toBeInstanceOf(File)
      expect(file.name).toBe('test.txt')
      expect(file.size).toBe(1024)
      expect(file.type).toBe('text/plain')
      expect(file.lastModified).toBeGreaterThan(0)
    })

    it('应该使用默认类型', () => {
      const file = createMockFile('test.txt', 1024)

      expect(file.type).toBe('text/plain')
    })
  })

  describe('createMockArchiveFile', () => {
    it('应该创建ZIP模拟文件', () => {
      const file = createMockArchiveFile('test.zip', 2048, 'zip')

      expect(file.name).toBe('test.zip')
      expect(file.size).toBe(2048)
      expect(file.type).toBe('application/zip')
    })

    it('应该创建RAR模拟文件', () => {
      const file = createMockArchiveFile('test.rar', 2048, 'rar')

      expect(file.type).toBe('application/x-rar-compressed')
    })

    it('应该使用默认格式(ZIP)', () => {
      const file = createMockArchiveFile('test.zip', 2048)

      expect(file.type).toBe('application/zip')
    })
  })

  describe('createMockFileList', () => {
    it('应该创建指定数量的文件', () => {
      const files = createMockFileList(5, 'test', 1024)

      expect(files).toHaveLength(5)
      expect(files[0].name).toBe('test_1.txt')
      expect(files[4].name).toBe('test_5.txt')
    })

    it('应该创建递增大小的文件', () => {
      const files = createMockFileList(3, 'size', 1024)

      expect(files[0].size).toBe(1024)
      expect(files[1].size).toBe(2048)
      expect(files[2].size).toBe(3072)
    })
  })

  describe('mockTauriInvoke', () => {
    it('应该模拟成功的API调用', async () => {
      const mockInvoke = mockTauriInvoke(true, { success: true, data: 'test' })

      const result = await mockInvoke('test_command', 'arg1', 'arg2')

      expect(result).toEqual({ success: true, data: 'test' })
      expect(mockInvoke).toHaveBeenCalledWith('test_command', 'arg1', 'arg2')
    })

    it('应该模拟失败的API调用', async () => {
      const mockInvoke = mockTauriInvoke(false, null)

      await expect(mockInvoke('test_command')).rejects.toThrow('模拟API错误')
    })

    it('应该模拟延迟的API调用', async () => {
      const startTime = Date.now()
      const mockInvoke = mockTauriInvoke(true, null, 100)

      await mockInvoke('test_command')
      const endTime = Date.now()
      const duration = endTime - startTime

      expect(duration).toBeGreaterThanOrEqual(90) // 大约100ms延迟
    })
  })

  describe('mockFileSystem', () => {
    it('应该模拟文件系统操作', async () => {
      const fs = mockFileSystem()

      // 设置测试文件
      fs._setFile('/test/file.txt', 'test content')

      // 测试读取
      await expect(fs.readTextFile('/test/file.txt')).resolves.toBe('test content')

      // 测试写入
      await fs.writeTextFile('/test/new.txt', 'new content')
      expect(fs.writeTextFile).toHaveBeenCalledWith('/test/new.txt', 'new content')

      // 测试文件不存在
      await expect(fs.readTextFile('/nonexistent.txt')).rejects.toThrow('文件不存在')
    })

    it('应该跟踪文件数量', () => {
      const fs = mockFileSystem()

      fs._setFile('/file1.txt', 'content1')
      fs._setFile('/file2.txt', 'content2')

      expect(fs._getFileCount()).toBe(2)

      fs._clearFiles()
      expect(fs._getFileCount()).toBe(0)
    })
  })

  describe('mockDatabase', () => {
    it('应该模拟数据库操作', async () => {
      const db = mockDatabase()

      // 测试插入
      const insertResult = await db.insert('users', { name: 'John', age: 30 })
      expect(insertResult.lastInsertId).toMatch(/^mock_\d+$/)

      // 测试查询
      const selectResult = await db.select('SELECT * FROM users', [])
      expect(Array.isArray(selectResult)).toBe(true)

      // 测试更新
      const updateResult = await db.update('users', { id: 'mock_1', name: 'Jane' })
      expect(updateResult.rowsAffected).toBe(1)

      // 测试删除
      const deleteResult = await db.delete('users', 'mock_1')
      expect(deleteResult.rowsAffected).toBe(1)
    })

    it('应该管理测试数据', () => {
      const db = mockDatabase()

      db._setData('test1', { id: 'test1', name: 'Item 1' })
      db._setData('test2', { id: 'test2', name: 'Item 2' })

      expect(db._getData('test1')).toEqual({ id: 'test1', name: 'Item 1' })
      expect(db._getDataCount()).toBe(2)

      db._clearData()
      expect(db._getDataCount()).toBe(0)
    })
  })

  describe('mockCompressionService', () => {
    it('应该模拟压缩服务', async () => {
      const service = mockCompressionService()
      const files = [createMockFile('test.txt', 1024)]

      // 测试压缩
      const compressResult = await service.compress(files, { compressionLevel: 6 })
      expect(compressResult.success).toBe(true)
      expect(compressResult.outputPath).toBe('/mock/output/compressed.zip')

      // 测试解压
      const extractResult = await service.extract('/test/archive.zip', '/output')
      expect(extractResult.success).toBe(true)
      expect(extractResult.extractedFiles).toHaveLength(2)

      // 测试列表内容
      const listResult = await service.listContents('/test/archive.zip')
      expect(listResult).toHaveLength(2)

      // 测试支持的格式
      const formats = service.getSupportedFormats()
      expect(formats).toContain('zip')
      expect(formats).toContain('rar')
    })
  })

  describe('mockPasswordManager', () => {
    it('应该模拟密码管理器', async () => {
      const manager = mockPasswordManager()

      // 测试添加密码
      await manager.addPassword('general', 'password123')
      await manager.addPassword('general', 'secret456')

      // 测试获取密码
      const passwords = await manager.getPasswords('general')
      expect(passwords).toContain('password123')
      expect(passwords).toContain('secret456')

      // 测试检查密码
      const hasPassword = await manager.hasPassword('general', 'password123')
      expect(hasPassword).toBe(true)

      // 测试移除密码
      await manager.removePassword('general', 'password123')
      const updatedPasswords = await manager.getPasswords('general')
      expect(updatedPasswords).not.toContain('password123')
      expect(updatedPasswords).toContain('secret456')

      // 测试清空密码
      await manager.clearPasswords('general')
      const emptyPasswords = await manager.getPasswords('general')
      expect(emptyPasswords).toHaveLength(0)
    })

    it('应该管理测试密码', () => {
      const manager = mockPasswordManager()

      manager._setPasswords('test', ['pass1', 'pass2', 'pass3'])
      expect(manager._getPasswordCount('test')).toBe(3)

      manager._clearAll()
      expect(manager._getPasswordCount('test')).toBe(0)
    })
  })

  describe('wait', () => {
    it('应该等待指定时间', async () => {
      const startTime = Date.now()
      await wait(100)
      const endTime = Date.now()
      const duration = endTime - startTime

      expect(duration).toBeGreaterThanOrEqual(90)
      expect(duration).toBeLessThan(150)
    })
  })

  describe('generateTestId', () => {
    it('应该生成唯一的测试ID', () => {
      const id1 = generateTestId('test')
      const id2 = generateTestId('test')

      expect(id1).toMatch(/^test_\d+_[a-z0-9]+$/)
      expect(id2).toMatch(/^test_\d+_[a-z0-9]+$/)
      expect(id1).not.toBe(id2)
    })

    it('应该使用默认前缀', () => {
      const id = generateTestId()

      expect(id).toMatch(/^test_\d+_[a-z0-9]+$/)
    })
  })

  describe('validateFile', () => {
    it('应该验证文件对象', () => {
      const file = createMockFile('test.txt', 1024, 'text/plain')

      expect(() => {
        validateFile(file, 'test.txt', 1024, 'text/plain')
      }).not.toThrow()
    })

    it('应该验证部分属性', () => {
      const file = createMockFile('test.txt', 1024)

      expect(() => {
        validateFile(file, 'test.txt')
      }).not.toThrow()

      expect(() => {
        validateFile(file, undefined, 1024)
      }).not.toThrow()

      expect(() => {
        validateFile(file, undefined, undefined, 'text/plain')
      }).not.toThrow()
    })

    it('应该验证文件实例', () => {
      const file = createMockFile('test.txt', 1024)

      expect(() => {
        validateFile(file)
      }).not.toThrow()

      // 应该对非文件对象抛出错误
      expect(() => {
        validateFile({} as any)
      }).toThrow()
    })
  })

  describe('cleanupTestEnvironment', () => {
    it('应该清理测试环境', () => {
      const mock1 = vi.fn()
      const mock2 = vi.fn()

      // 设置一些模拟
      vi.mock('test-module', () => ({ mock1 }))
      vi.mock('another-module', () => ({ mock2 }))

      // 调用清理
      cleanupTestEnvironment()

      // 验证模拟被清理
      expect(vi.isMockFunction(mock1)).toBe(false)
      expect(vi.isMockFunction(mock2)).toBe(false)
    })
  })
})