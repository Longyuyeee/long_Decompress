import { test, expect } from '@playwright/test'

test.describe('文件上传功能', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('应该能够通过拖放上传文件', async ({ page }) => {
    // 准备测试文件
    const filePath = 'tests/data/test_files/small.txt'

    // 模拟拖放操作
    const dropZone = page.locator('.drop-zone')
    await expect(dropZone).toBeVisible()

    // 创建数据转移对象
    const dataTransfer = await page.evaluateHandle(() => {
      const dt = new DataTransfer()
      const file = new File(['test content'], 'test.txt', { type: 'text/plain' })
      dt.items.add(file)
      return dt
    })

    // 触发拖放事件
    await dropZone.dispatchEvent('drop', { dataTransfer })

    // 验证文件被接受
    await expect(page.locator('.file-list')).toBeVisible()
    await expect(page.locator('.file-item')).toHaveCount(1)
    await expect(page.locator('.file-name')).toContainText('test.txt')
  })

  test('应该能够通过文件选择器上传文件', async ({ page }) => {
    // 准备测试文件
    const filePath = 'tests/data/test_files/small.txt'

    // 点击文件选择按钮
    const fileInput = page.locator('input[type="file"]')
    await expect(fileInput).toBeVisible()

    // 上传文件
    await fileInput.setInputFiles(filePath)

    // 验证文件被接受
    await expect(page.locator('.file-list')).toBeVisible()
    await expect(page.locator('.file-item')).toHaveCount(1)
    await expect(page.locator('.file-name')).toContainText('small.txt')
  })

  test('应该能够上传多个文件', async ({ page }) => {
    // 准备多个测试文件
    const filePaths = [
      'tests/data/test_files/small.txt',
      'tests/data/test_files/medium.pdf',
      'tests/data/test_files/large.zip'
    ]

    // 设置多文件上传
    const fileInput = page.locator('input[type="file"]')
    await fileInput.setInputFiles(filePaths)

    // 验证所有文件被接受
    await expect(page.locator('.file-list')).toBeVisible()
    await expect(page.locator('.file-item')).toHaveCount(3)

    // 验证文件信息显示
    await expect(page.locator('.file-name')).toContainText(['small.txt', 'medium.pdf', 'large.zip'])
  })

  test('应该显示文件大小信息', async ({ page }) => {
    const filePath = 'tests/data/test_files/small.txt'

    await page.locator('input[type="file"]').setInputFiles(filePath)

    // 验证文件大小显示
    await expect(page.locator('.file-size')).toBeVisible()
    await expect(page.locator('.file-size')).toContainText(/KB|MB|GB/)
  })

  test('应该能够移除已选择的文件', async ({ page }) => {
    const filePath = 'tests/data/test_files/small.txt'

    await page.locator('input[type="file"]').setInputFiles(filePath)
    await expect(page.locator('.file-item')).toHaveCount(1)

    // 点击移除按钮
    await page.locator('.remove-file').click()

    // 验证文件被移除
    await expect(page.locator('.file-item')).toHaveCount(0)
    await expect(page.locator('.empty-state')).toBeVisible()
  })

  test('应该能够清空所有文件', async ({ page }) => {
    // 上传多个文件
    const filePaths = [
      'tests/data/test_files/small.txt',
      'tests/data/test_files/medium.pdf'
    ]

    await page.locator('input[type="file"]').setInputFiles(filePaths)
    await expect(page.locator('.file-item')).toHaveCount(2)

    // 点击清空按钮
    await page.locator('.clear-all').click()

    // 验证所有文件被清除
    await expect(page.locator('.file-item')).toHaveCount(0)
    await expect(page.locator('.empty-state')).toBeVisible()
  })

  test('应该验证文件类型', async ({ page }) => {
    // 设置只接受特定类型
    await page.locator('.file-type-filter').selectOption('.txt,.pdf')

    // 尝试上传不支持的类型
    const invalidFile = 'tests/data/test_files/test.exe'

    await page.locator('input[type="file"]').setInputFiles(invalidFile)

    // 验证错误消息显示
    await expect(page.locator('.error-message')).toBeVisible()
    await expect(page.locator('.error-message')).toContainText('不支持的文件类型')
  })

  test('应该验证文件大小限制', async ({ page }) => {
    // 设置文件大小限制为1MB
    await page.locator('.max-size-input').fill('1')

    // 尝试上传大文件
    const largeFile = 'tests/data/test_files/large.zip'

    await page.locator('input[type="file"]').setInputFiles(largeFile)

    // 验证错误消息显示
    await expect(page.locator('.error-message')).toBeVisible()
    await expect(page.locator('.error-message')).toContainText('文件大小超过限制')
  })

  test('应该显示上传进度', async ({ page }) => {
    const filePath = 'tests/data/test_files/medium.pdf'

    await page.locator('input[type="file"]').setInputFiles(filePath)

    // 验证进度条显示
    await expect(page.locator('.progress-bar')).toBeVisible()

    // 等待上传完成
    await expect(page.locator('.progress-bar')).toBeHidden({ timeout: 10000 })

    // 验证上传完成状态
    await expect(page.locator('.upload-complete')).toBeVisible()
  })

  test('应该处理上传错误', async ({ page }) => {
    // 模拟上传错误
    await page.route('**/upload', route => {
      route.fulfill({
        status: 500,
        body: JSON.stringify({ error: '上传失败' })
      })
    })

    const filePath = 'tests/data/test_files/small.txt'
    await page.locator('input[type="file"]').setInputFiles(filePath)

    // 验证错误处理
    await expect(page.locator('.error-message')).toBeVisible()
    await expect(page.locator('.error-message')).toContainText('上传失败')
  })

  test('应该支持批量上传', async ({ page }) => {
    // 启用批量上传模式
    await page.locator('.batch-mode-toggle').click()

    // 上传文件夹
    const folderPath = 'tests/data/test_files/'

    // 注意：Playwright目前不支持直接上传文件夹
    // 这里我们上传文件夹内的所有文件
    const filePaths = [
      'tests/data/test_files/small.txt',
      'tests/data/test_files/medium.pdf',
      'tests/data/test_files/large.zip'
    ]

    await page.locator('input[type="file"]').setInputFiles(filePaths)

    // 验证批量上传状态
    await expect(page.locator('.batch-status')).toBeVisible()
    await expect(page.locator('.total-files')).toContainText('3')
  })

  test('应该保持文件选择状态', async ({ page }) => {
    const filePath = 'tests/data/test_files/small.txt'

    // 上传文件
    await page.locator('input[type="file"]').setInputFiles(filePath)

    // 导航到其他页面
    await page.locator('.nav-settings').click()
    await expect(page).toHaveURL(/.*settings/)

    // 返回文件上传页面
    await page.locator('.nav-upload').click()
    await expect(page).toHaveURL(/.*upload/)

    // 验证文件选择状态保持
    await expect(page.locator('.file-item')).toHaveCount(1)
  })
})

test.describe('文件上传性能', () => {
  test('应该快速处理小文件上传', async ({ page }) => {
    await page.goto('/')

    const startTime = Date.now()
    const filePath = 'tests/data/test_files/small.txt'

    await page.locator('input[type="file"]').setInputFiles(filePath)

    const endTime = Date.now()
    const duration = endTime - startTime

    // 验证小文件上传在1秒内完成
    expect(duration).toBeLessThan(1000)

    await expect(page.locator('.file-item')).toBeVisible()
  })

  test('应该有效处理大文件上传', async ({ page }) => {
    await page.goto('/')

    const startTime = Date.now()
    const filePath = 'tests/data/test_files/large.zip'

    await page.locator('input[type="file"]').setInputFiles(filePath)

    const endTime = Date.now()
    const duration = endTime - startTime

    // 验证大文件上传在合理时间内完成
    expect(duration).toBeLessThan(5000)

    await expect(page.locator('.file-item')).toBeVisible()
  })

  test('应该有效管理内存使用', async ({ page }) => {
    await page.goto('/')

    // 上传多个大文件
    const filePaths = Array(10).fill('tests/data/test_files/medium.pdf')

    const memoryBefore = await page.evaluate(() => performance.memory?.usedJSHeapSize || 0)

    await page.locator('input[type="file"]').setInputFiles(filePaths)

    const memoryAfter = await page.evaluate(() => performance.memory?.usedJSHeapSize || 0)
    const memoryIncrease = memoryAfter - memoryBefore

    // 验证内存增长在合理范围内（小于100MB）
    expect(memoryIncrease).toBeLessThan(100 * 1024 * 1024)
  })
})

test.describe('文件上传可访问性', () => {
  test('应该支持键盘导航', async ({ page }) => {
    await page.goto('/')

    // 使用Tab键导航到文件输入
    await page.keyboard.press('Tab')
    await expect(page.locator('input[type="file"]')).toBeFocused()

    // 使用空格键触发文件选择
    await page.keyboard.press(' ')

    // 验证文件选择对话框（模拟）
    await expect(page.locator('.file-dialog')).toBeVisible()
  })

  test('应该提供适当的ARIA标签', async ({ page }) => {
    await page.goto('/')

    // 验证文件输入有ARIA标签
    const fileInput = page.locator('input[type="file"]')
    await expect(fileInput).toHaveAttribute('aria-label', /文件选择|file selection/i)

    // 验证拖放区域有ARIA标签
    const dropZone = page.locator('.drop-zone')
    await expect(dropZone).toHaveAttribute('aria-label', /拖放区域|drop zone/i)

    // 验证移除按钮有ARIA标签
    await page.locator('input[type="file"]').setInputFiles('tests/data/test_files/small.txt')
    const removeButton = page.locator('.remove-file').first()
    await expect(removeButton).toHaveAttribute('aria-label', /移除文件|remove file/i)
  })

  test('应该提供屏幕阅读器支持', async ({ page }) => {
    await page.goto('/')

    // 验证状态消息
    await page.locator('input[type="file"]').setInputFiles('tests/data/test_files/small.txt')

    const statusRegion = page.locator('[role="status"]')
    await expect(statusRegion).toBeVisible()
    await expect(statusRegion).toContainText(/文件已选择|file selected/i)
  })
})