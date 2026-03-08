# 测试框架使用示例

本文档展示如何使用新建的测试框架编写各种类型的测试。

## 单元测试示例

### Vue组件测试

```typescript
// src/components/__tests__/ExampleComponent.test.ts
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ExampleComponent from '../ExampleComponent.vue'
import { createMockFile, mockTauriInvoke } from '../../tests/fixtures/test_helpers'

describe('ExampleComponent', () => {
  it('渲染正确', () => {
    const wrapper = mount(ExampleComponent, {
      props: { title: '测试标题' }
    })

    expect(wrapper.exists()).toBe(true)
    expect(wrapper.text()).toContain('测试标题')
  })

  it('处理用户交互', async () => {
    const wrapper = mount(ExampleComponent)

    await wrapper.find('button').trigger('click')

    expect(wrapper.emitted('click')).toBeTruthy()
  })

  it('处理文件上传', async () => {
    const mockFile = createMockFile('test.txt', 1024)
    const wrapper = mount(ExampleComponent)

    const fileInput = wrapper.find('input[type="file"]')
    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile]
    })

    await fileInput.trigger('change')

    expect(wrapper.vm.selectedFile).toBe(mockFile)
  })
})
```

### 工具函数测试

```typescript
// src/utils/__tests__/formatUtils.test.ts
import { describe, it, expect } from 'vitest'
import { formatFileSize, formatDuration } from '../formatUtils'

describe('formatUtils', () => {
  describe('formatFileSize', () => {
    it('格式化字节', () => {
      expect(formatFileSize(500)).toBe('500 B')
    })

    it('格式化千字节', () => {
      expect(formatFileSize(1500)).toBe('1.5 KB')
    })

    it('格式化兆字节', () => {
      expect(formatFileSize(1500000)).toBe('1.43 MB')
    })

    it('格式化千兆字节', () => {
      expect(formatFileSize(1500000000)).toBe('1.4 GB')
    })
  })

  describe('formatDuration', () => {
    it('格式化秒数', () => {
      expect(formatDuration(45)).toBe('45秒')
    })

    it('格式化分钟', () => {
      expect(formatDuration(125)).toBe('2分5秒')
    })

    it('格式化小时', () => {
      expect(formatDuration(3665)).toBe('1小时1分5秒')
    })
  })
})
```

## 集成测试示例

```typescript
// tests/integration/file_management.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import App from '../src/App.vue'
import {
  createMockFile,
  mockTauriInvoke,
  mockFileSystem,
  wait
} from '../fixtures/test_helpers'

describe('文件管理集成测试', () => {
  let mockInvoke: any
  let fileSystem: any

  beforeEach(() => {
    mockInvoke = mockTauriInvoke(true, { success: true })
    fileSystem = mockFileSystem()

    vi.mocked(require('@tauri-apps/api').invoke).mockImplementation(mockInvoke)
    vi.mocked(require('@tauri-apps/api').fs).mockImplementation(fileSystem)
  })

  it('完整的文件上传和处理流程', async () => {
    const wrapper = mount(App)

    // 1. 选择文件
    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('document.pdf', 2048, 'application/pdf')

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile]
    })
    await fileInput.trigger('change')

    // 2. 设置压缩选项
    await wrapper.find('input[name="password"]').setValue('secret123')
    await wrapper.find('select[name="compression-level"]').setValue('9')

    // 3. 开始压缩
    mockInvoke.mockResolvedValue({
      success: true,
      outputPath: '/output/compressed.zip',
      size: 1024
    })

    await wrapper.find('button.compress-button').trigger('click')

    // 4. 验证结果
    expect(mockInvoke).toHaveBeenCalledWith(
      'compress_files',
      expect.any(Array),
      expect.objectContaining({
        password: 'secret123',
        compressionLevel: 9
      })
    )

    // 5. 验证UI更新
    await wait(100)
    expect(wrapper.text()).toContain('压缩完成')
    expect(wrapper.text()).toContain('compressed.zip')
  })

  it('处理压缩错误', async () => {
    const wrapper = mount(App)

    // 模拟压缩失败
    mockInvoke.mockRejectedValue(new Error('磁盘空间不足'))

    const fileInput = wrapper.find('input[type="file"]')
    const mockFile = createMockFile('large.iso', 1024 * 1024 * 1024) // 1GB

    Object.defineProperty(fileInput.element, 'files', {
      value: [mockFile]
    })
    await fileInput.trigger('change')

    await wrapper.find('button.compress-button').trigger('click')

    // 验证错误处理
    await wait(100)
    expect(wrapper.text()).toContain('错误')
    expect(wrapper.text()).toContain('磁盘空间不足')
  })
})
```

## E2E测试示例

```typescript
// tests/e2e/specs/compression_workflow.spec.ts
import { test, expect } from '@playwright/test'

test.describe('压缩工作流', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('完整的压缩流程', async ({ page }) => {
    // 1. 选择文件
    await page.locator('input[type="file"]').setInputFiles('tests/data/test_files/small.txt')

    await expect(page.locator('.file-item')).toHaveCount(1)
    await expect(page.locator('.file-name')).toContainText('small.txt')

    // 2. 设置密码
    await page.locator('input[name="password"]').fill('mysecretpassword')

    // 3. 设置压缩级别
    await page.locator('select[name="compression-level"]').selectOption('9')

    // 4. 开始压缩
    await page.locator('button.compress-button').click()

    // 5. 验证压缩进度
    await expect(page.locator('.progress-bar')).toBeVisible()

    // 6. 等待压缩完成
    await expect(page.locator('.progress-bar')).toBeHidden({ timeout: 10000 })

    // 7. 验证结果
    await expect(page.locator('.success-message')).toBeVisible()
    await expect(page.locator('.success-message')).toContainText('压缩完成')

    // 8. 验证历史记录
    await page.locator('.nav-history').click()
    await expect(page.locator('.history-item')).toHaveCount(1)
    await expect(page.locator('.history-item')).toContainText('small.txt')
  })

  test('批量文件压缩', async ({ page }) => {
    const filePaths = [
      'tests/data/test_files/small.txt',
      'tests/data/test_files/medium.pdf',
      'tests/data/test_files/large.zip'
    ]

    // 上传多个文件
    await page.locator('input[type="file"]').setInputFiles(filePaths)

    await expect(page.locator('.file-item')).toHaveCount(3)

    // 启用批量模式
    await page.locator('.batch-toggle').click()

    // 开始批量压缩
    await page.locator('button.batch-compress').click()

    // 验证批量进度
    await expect(page.locator('.batch-progress')).toBeVisible()
    await expect(page.locator('.progress-text')).toContainText('处理中: 3个文件')

    // 等待完成
    await expect(page.locator('.batch-progress')).toBeHidden({ timeout: 15000 })

    // 验证结果
    await expect(page.locator('.batch-result')).toBeVisible()
    await expect(page.locator('.processed-count')).toContainText('3')
  })
})
```

## 性能测试示例

```typescript
// tests/performance/memory_usage.test.ts
import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import {
  createMockFileList,
  mockCompressionService,
  wait
} from '../fixtures/test_helpers'

describe('内存使用性能测试', () => {
  let compressionService: any

  beforeAll(() => {
    compressionService = mockCompressionService()
  })

  afterAll(() => {
    vi.clearAllMocks()
  })

  it('压缩大文件时内存使用稳定', async () => {
    const largeFile = createMockFile('large.bin', 500 * 1024 * 1024) // 500MB

    // 记录初始内存
    const memoryBefore = process.memoryUsage()

    // 执行压缩
    await compressionService.compress([largeFile], {
      compressionLevel: 6,
      password: null
    })

    // 记录压缩后内存
    const memoryAfter = process.memoryUsage()
    const memoryIncrease = memoryAfter.heapUsed - memoryBefore.heapUsed

    // 验证内存增长在合理范围内（小于文件大小的2倍）
    expect(memoryIncrease).toBeLessThan(largeFile.size * 2)

    // 验证内存最终会释放
    await wait(1000) // 等待GC
    const memoryFinal = process.memoryUsage()
    const memoryFinalIncrease = memoryFinal.heapUsed - memoryBefore.heapUsed

    expect(memoryFinalIncrease).toBeLessThan(memoryIncrease * 0.5) // 至少释放一半
  })

  it('批量压缩时内存使用线性增长', async () => {
    const files = createMockFileList(10, 'batch', 10 * 1024 * 1024) // 10个10MB文件

    const memoryUsage: number[] = []

    // 逐个压缩并记录内存
    for (let i = 0; i < files.length; i++) {
      memoryUsage.push(process.memoryUsage().heapUsed)

      await compressionService.compress([files[i]], {
        compressionLevel: 6,
        password: null
      })

      await wait(100) // 等待一下
    }

    // 计算内存增长趋势
    const memoryIncreases = memoryUsage.slice(1).map((mem, idx) => mem - memoryUsage[idx])
    const averageIncrease = memoryIncreases.reduce((a, b) => a + b) / memoryIncreases.length

    // 验证内存增长相对稳定（标准差小于平均值的20%）
    const variance = memoryIncreases.reduce((a, b) => a + Math.pow(b - averageIncrease, 2), 0) / memoryIncreases.length
    const stdDev = Math.sqrt(variance)

    expect(stdDev).toBeLessThan(averageIncrease * 0.2)
  })
})
```

## 测试数据生成示例

```bash
# 使用脚本生成测试数据
./scripts/generate_test_files.sh

# 生成的文件包括：
# - 各种大小的文本文件 (1KB, 10KB, 100KB, 1MB)
# - 代码文件示例 (.rs, .js, .ts)
# - JSON配置文件
# - 二进制测试文件
# - 压缩文件样本 (.zip, .tar.gz)
# - 大文件 (10MB, 50MB, 100MB)
# - 嵌套目录结构
# - 特殊字符文件名

# 手动创建特定测试文件
echo "测试内容" > tests/data/test_files/custom_test.txt
dd if=/dev/urandom of=tests/data/test_files/random_1m.bin bs=1M count=1
```

## CI/CD集成示例

### 本地运行CI测试
```bash
# 运行所有测试（模拟CI流程）
./scripts/run_all_tests.sh

# 输出包括：
# - 单元测试结果和覆盖率
# - 集成测试结果
# - E2E测试结果
# - 性能测试结果
# - 代码质量检查
# - 安全扫描
```

### GitHub Actions工作流
```yaml
# .github/workflows/tests.yml 中的关键部分
name: Test Suite
on: [push, pull_request]

jobs:
  test-frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm ci
      - run: npm run test:unit
      - run: npm run test:unit:coverage
      - uses: codecov/codecov-action@v3

  test-rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cd src-tauri && cargo test --verbose

  test-e2e:
    runs-on: ubuntu-latest
    needs: [test-frontend, test-rust]
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm ci
      - run: npx playwright install --with-deps
      - run: npm run test:e2e
```

## 最佳实践提示

### 1. 测试组织
- 按功能模块组织测试文件
- 使用描述性的测试套件名称
- 保持测试文件与源代码文件结构一致

### 2. 测试数据
- 使用测试辅助函数创建模拟数据
- 避免硬编码的测试数据
- 为边界情况创建专门的测试数据

### 3. 测试维护
- 定期更新测试以适应代码变化
- 删除过时或重复的测试
- 监控测试执行时间和资源使用

### 4. 团队协作
- 在PR中包括相关测试
- 代码审查时检查测试质量
- 分享测试技巧和最佳实践

## 故障排除

### 常见问题
1. **测试失败**：检查测试环境配置
2. **覆盖率低**：检查未覆盖的代码路径
3. **性能问题**：优化测试执行时间
4. **环境差异**：确保测试环境一致

### 调试技巧
```typescript
// 在测试中添加调试输出
console.log('调试信息:', variable)

// 使用Vitest的调试模式
npm run test:unit -- --reporter=verbose

// 运行单个测试
npm run test:unit -- --run "测试名称"

// 使用Playwright的调试模式
npm run test:e2e:ui
```

## 总结

新建的测试框架提供了完整的测试基础设施，支持：
- ✅ 单元测试、集成测试、E2E测试、性能测试
- ✅ 丰富的测试辅助工具和模拟函数
- ✅ 完整的CI/CD集成
- ✅ 多平台测试支持
- ✅ 详细的测试报告和覆盖率分析

开发团队可以基于这个框架快速编写高质量的测试，确保代码质量和项目稳定性。