import { config } from '@vue/test-utils'
import { vi } from 'vitest'

// 全局测试配置
config.global.stubs = {
  Transition: false,
  TransitionGroup: false,
}

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
    appLocalDataDir: vi.fn(() => Promise.resolve('/test/app/local/data')),
  },
  fs: {
    readDir: vi.fn(),
    readTextFile: vi.fn(),
    writeTextFile: vi.fn(),
  },
}))

// 全局测试工具函数
export const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

// 模拟文件对象
export const createMockFile = (name: string, size: number, type = 'text/plain') => {
  const file = new File([''], name, { type })
  Object.defineProperty(file, 'size', { value: size })
  return file
}

// 模拟压缩文件
export const createMockZipFile = () => {
  const file = new File([''], 'test.zip', { type: 'application/zip' })
  Object.defineProperty(file, 'size', { value: 1024 })
  return file
}