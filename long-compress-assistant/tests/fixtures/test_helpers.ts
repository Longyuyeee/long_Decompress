/**
 * 测试辅助函数库
 * 提供通用的测试工具函数和模拟数据
 */

import { vi } from 'vitest'
import { File } from '@tauri-apps/api/fs'

/**
 * 创建模拟文件对象
 * @param name 文件名
 * @param size 文件大小（字节）
 * @param type MIME类型
 * @returns 模拟的File对象
 */
export const createMockFile = (name: string, size: number, type = 'text/plain'): File => {
  const file = new File([''], name, { type })
  Object.defineProperty(file, 'size', { value: size })
  Object.defineProperty(file, 'lastModified', { value: Date.now() })
  return file
}

/**
 * 创建模拟压缩文件
 * @param name 文件名
 * @param size 文件大小
 * @param format 压缩格式
 * @returns 模拟的压缩文件
 */
export const createMockArchiveFile = (
  name: string,
  size: number,
  format: 'zip' | 'rar' | '7z' | 'tar' | 'gz' = 'zip'
): File => {
  const mimeTypes = {
    zip: 'application/zip',
    rar: 'application/x-rar-compressed',
    '7z': 'application/x-7z-compressed',
    tar: 'application/x-tar',
    gz: 'application/gzip',
  }

  return createMockFile(name, size, mimeTypes[format])
}

/**
 * 创建模拟文件列表
 * @param count 文件数量
 * @param baseName 基础文件名
 * @param baseSize 基础文件大小
 * @returns 文件数组
 */
export const createMockFileList = (
  count: number,
  baseName = 'file',
  baseSize = 1024
): File[] => {
  return Array.from({ length: count }, (_, i) =>
    createMockFile(`${baseName}_${i + 1}.txt`, baseSize * (i + 1))
  )
}

/**
 * 模拟Tauri API调用
 * @param success 是否成功
 * @param data 返回数据
 * @param delay 延迟时间（毫秒）
 * @returns 模拟的invoke函数
 */
export const mockTauriInvoke = (
  success = true,
  data: any = null,
  delay = 100
) => {
  return vi.fn().mockImplementation(() => {
    return new Promise((resolve, reject) => {
      setTimeout(() => {
        if (success) {
          resolve(data)
        } else {
          reject(new Error('模拟API错误'))
        }
      }, delay)
    })
  })
}

/**
 * 模拟文件系统操作
 */
export const mockFileSystem = () => {
  const files = new Map<string, string>()

  return {
    readTextFile: vi.fn().mockImplementation((path: string) => {
      if (files.has(path)) {
        return Promise.resolve(files.get(path))
      }
      return Promise.reject(new Error(`文件不存在: ${path}`))
    }),

    writeTextFile: vi.fn().mockImplementation((path: string, content: string) => {
      files.set(path, content)
      return Promise.resolve()
    }),

    readDir: vi.fn().mockResolvedValue([]),

    removeFile: vi.fn().mockResolvedValue(undefined),

    exists: vi.fn().mockResolvedValue(true),

    // 测试辅助方法
    _setFile: (path: string, content: string) => {
      files.set(path, content)
    },

    _clearFiles: () => {
      files.clear()
    },

    _getFileCount: () => files.size,
  }
}

/**
 * 模拟数据库操作
 */
export const mockDatabase = () => {
  const data = new Map<string, any>()
  let idCounter = 1

  return {
    execute: vi.fn().mockResolvedValue({ rowsAffected: 1 }),

    select: vi.fn().mockImplementation((query: string, params: any[]) => {
      // 简单模拟查询
      if (query.includes('SELECT')) {
        return Promise.resolve(Array.from(data.values()))
      }
      return Promise.resolve([])
    }),

    insert: vi.fn().mockImplementation((table: string, record: any) => {
      const id = `mock_${idCounter++}`
      const recordWithId = { ...record, id }
      data.set(id, recordWithId)
      return Promise.resolve({ lastInsertId: id })
    }),

    update: vi.fn().mockImplementation((table: string, record: any) => {
      if (record.id && data.has(record.id)) {
        data.set(record.id, { ...data.get(record.id), ...record })
        return Promise.resolve({ rowsAffected: 1 })
      }
      return Promise.resolve({ rowsAffected: 0 })
    }),

    delete: vi.fn().mockImplementation((table: string, id: string) => {
      if (data.has(id)) {
        data.delete(id)
        return Promise.resolve({ rowsAffected: 1 })
      }
      return Promise.resolve({ rowsAffected: 0 })
    }),

    // 测试辅助方法
    _setData: (id: string, record: any) => {
      data.set(id, record)
    },

    _getData: (id: string) => data.get(id),

    _clearData: () => {
      data.clear()
      idCounter = 1
    },

    _getDataCount: () => data.size,
  }
}

/**
 * 模拟压缩服务
 */
export const mockCompressionService = () => {
  return {
    compress: vi.fn().mockImplementation((files: File[], options: any) => {
      return Promise.resolve({
        success: true,
        outputPath: '/mock/output/compressed.zip',
        size: 1024,
        duration: 1000,
      })
    }),

    extract: vi.fn().mockImplementation((archivePath: string, outputDir: string, password?: string) => {
      return Promise.resolve({
        success: true,
        outputPath: outputDir,
        extractedFiles: ['file1.txt', 'file2.txt'],
        size: 2048,
        duration: 500,
      })
    }),

    listContents: vi.fn().mockResolvedValue([
      { name: 'file1.txt', size: 512, compressedSize: 256 },
      { name: 'file2.txt', size: 1024, compressedSize: 512 },
    ]),

    testArchive: vi.fn().mockResolvedValue({ valid: true, errors: [] }),

    getSupportedFormats: vi.fn().mockReturnValue(['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz']),
  }
}

/**
 * 模拟密码管理器
 */
export const mockPasswordManager = () => {
  const passwords = new Map<string, string[]>()

  return {
    addPassword: vi.fn().mockImplementation((category: string, password: string) => {
      if (!passwords.has(category)) {
        passwords.set(category, [])
      }
      passwords.get(category)!.push(password)
      return Promise.resolve()
    }),

    getPasswords: vi.fn().mockImplementation((category: string) => {
      return Promise.resolve(passwords.get(category) || [])
    }),

    removePassword: vi.fn().mockImplementation((category: string, password: string) => {
      if (passwords.has(category)) {
        const list = passwords.get(category)!
        const index = list.indexOf(password)
        if (index > -1) {
          list.splice(index, 1)
        }
      }
      return Promise.resolve()
    }),

    clearPasswords: vi.fn().mockImplementation((category: string) => {
      passwords.delete(category)
      return Promise.resolve()
    }),

    hasPassword: vi.fn().mockImplementation((category: string, password: string) => {
      const list = passwords.get(category) || []
      return Promise.resolve(list.includes(password))
    }),

    // 测试辅助方法
    _setPasswords: (category: string, passwordList: string[]) => {
      passwords.set(category, [...passwordList])
    },

    _getPasswordCount: (category: string) => passwords.get(category)?.length || 0,

    _clearAll: () => {
      passwords.clear()
    },
  }
}

/**
 * 等待函数（用于异步测试）
 * @param ms 等待时间（毫秒）
 */
export const wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

/**
 * 模拟用户事件
 */
export const userEvent = {
  click: vi.fn(),
  type: vi.fn(),
  clear: vi.fn(),
  selectOptions: vi.fn(),
  deselectOptions: vi.fn(),
  upload: vi.fn(),
  dragAndDrop: vi.fn(),
  keyboard: vi.fn(),
}

/**
 * 生成测试ID
 * @param prefix ID前缀
 * @returns 唯一的测试ID
 */
export const generateTestId = (prefix = 'test') => {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
}

/**
 * 验证文件对象
 * @param file 文件对象
 * @param expectedName 预期文件名
 * @param expectedSize 预期文件大小
 * @param expectedType 预期文件类型
 */
export const validateFile = (
  file: File,
  expectedName?: string,
  expectedSize?: number,
  expectedType?: string
) => {
  if (expectedName) {
    expect(file.name).toBe(expectedName)
  }

  if (expectedSize !== undefined) {
    expect(file.size).toBe(expectedSize)
  }

  if (expectedType) {
    expect(file.type).toBe(expectedType)
  }

  expect(file).toBeInstanceOf(File)
  expect(file.size).toBeGreaterThanOrEqual(0)
  expect(file.name).toBeTruthy()
}

/**
 * 清理测试环境
 */
export const cleanupTestEnvironment = () => {
  vi.clearAllMocks()
  vi.resetAllMocks()
  vi.restoreAllMocks()
}

export default {
  createMockFile,
  createMockArchiveFile,
  createMockFileList,
  mockTauriInvoke,
  mockFileSystem,
  mockDatabase,
  mockCompressionService,
  mockPasswordManager,
  wait,
  userEvent,
  generateTestId,
  validateFile,
  cleanupTestEnvironment,
}