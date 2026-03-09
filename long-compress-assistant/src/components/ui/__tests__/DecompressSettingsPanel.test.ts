import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import DecompressSettingsPanel from '../DecompressSettingsPanel.vue'
import type { DecompressSettings } from '../DecompressSettingsPanel.vue'

// 模拟Tauri命令
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    selectDirectory: vi.fn().mockResolvedValue('/test/output/path')
  })
}))

describe('DecompressSettingsPanel组件', () => {
  it('渲染正确', () => {
    const wrapper = mount(DecompressSettingsPanel)

    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.decompress-settings-panel').exists()).toBe(true)

    // 检查主要部分是否存�?
    expect(wrapper.find('label:contains("输出目录")').exists()).toBe(true)
    expect(wrapper.find('label:contains("解压密码设置")').exists()).toBe(true)
    expect(wrapper.find('label:contains("解压选项")').exists()).toBe(true)
    expect(wrapper.find('label:contains("快速预�?)').exists()).toBe(true)
  })

  it('使用默认设置', () => {
    const wrapper = mount(DecompressSettingsPanel)

    // 检查默认�?
    const outputPathInput = wrapper.find('input[placeholder="选择解压输出目录"]')
    expect(outputPathInput.exists()).toBe(true)
    expect((outputPathInput.element as HTMLInputElement).value).toBe('')

    // 检查密码输�?
    const passwordInput = wrapper.find('input[placeholder="输入解压密码"]')
    expect(passwordInput.exists()).toBe(true)
    expect((passwordInput.element as HTMLInputElement).value).toBe('')
    expect(passwordInput.attributes('type')).toBe('password')

    // 检查解压选项
    const keepStructureCheckbox = wrapper.find('input[type="checkbox"]:checked')
    expect(keepStructureCheckbox.exists()).toBe(true)

    // 检查覆盖策�?
    const askRadio = wrapper.find('input[value="ask"]:checked')
    expect(askRadio.exists()).toBe(true)
  })

  it('接受外部设置', () => {
    const testSettings: DecompressSettings = {
      outputPath: '/custom/output/path',
      password: 'test123',
      options: {
        keepStructure: false,
        overwriteStrategy: 'overwrite',
        deleteAfter: true,
        preserveTimestamps: false,
        skipCorrupted: true,
        extractOnlyNewer: true,
        createSubdirectory: true,
        fileFilter: '*.txt,*.jpg'
      },
      passwordOptions: {
        rememberForSession: true,
        autoTryCommon: true,
        maxAttempts: 5
      }
    }

    const wrapper = mount(DecompressSettingsPanel, {
      props: {
        modelValue: testSettings
      }
    })

    // 检查输出路�?
    const outputPathInput = wrapper.find('input[placeholder="选择解压输出目录"]')
    expect((outputPathInput.element as HTMLInputElement).value).toBe('/custom/output/path')

    // 检查密�?
    const passwordInput = wrapper.find('input[placeholder="输入解压密码"]')
    expect((passwordInput.element as HTMLInputElement).value).toBe('test123')

    // 检查解压选项
    const keepStructureCheckbox = wrapper.find('input[type="checkbox"]')
    expect((keepStructureCheckbox.element as HTMLInputElement).checked).toBe(false)

    // 检查覆盖策�?
    const overwriteRadio = wrapper.find('input[value="overwrite"]:checked')
    expect(overwriteRadio.exists()).toBe(true)

    // 检查文件过滤器
    const fileFilterInput = wrapper.find('input[placeholder="例如: *.txt, *.jpg, document*"]')
    expect((fileFilterInput.element as HTMLInputElement).value).toBe('*.txt,*.jpg')
  })

  it('切换密码显示/隐藏', async () => {
    const wrapper = mount(DecompressSettingsPanel)

    const passwordInput = wrapper.find('input[placeholder="输入解压密码"]')
    const toggleButton = wrapper.find('button[aria-label="显示密码"]')

    // 初始状态为密码类型
    expect(passwordInput.attributes('type')).toBe('password')
    expect(toggleButton.find('i').classes()).toContain('pi-eye')

    // 点击切换按钮
    await toggleButton.trigger('click')

    // 应该变为文本类型
    expect(passwordInput.attributes('type')).toBe('text')
    expect(toggleButton.find('i').classes()).toContain('pi-eye-slash')
    expect(toggleButton.attributes('aria-label')).toBe('隐藏密码')

    // 再次点击切换回来
    await toggleButton.trigger('click')
    expect(passwordInput.attributes('type')).toBe('password')
    expect(toggleButton.find('i').classes()).toContain('pi-eye')
    expect(toggleButton.attributes('aria-label')).toBe('显示密码')
  })

  it('应用预设', async () => {
    const wrapper = mount(DecompressSettingsPanel)

    // 获取快速解压按�?
    const quickPresetButton = wrapper.find('button:contains("快速解�?)')
    expect(quickPresetButton.exists()).toBe(true)

    // 点击快速解压预�?
    await quickPresetButton.trigger('click')

    // 检查设置是否更�?
    const overwriteRadio = wrapper.find('input[value="overwrite"]:checked')
    expect(overwriteRadio.exists()).toBe(true)

    // 检查skipCorrupted应该为true
    const skipCorruptedCheckbox = wrapper.find('input[type="checkbox"]')
    // 需要找到正确的checkbox，这里简化处�?
    expect(wrapper.vm.settings.options.overwriteStrategy).toBe('overwrite')
    expect(wrapper.vm.settings.options.skipCorrupted).toBe(true)
  })

  it('重置设置', async () => {
    // 模拟confirm返回true
    window.confirm = vi.fn().mockReturnValue(true)

    const wrapper = mount(DecompressSettingsPanel)

    // 修改一些设�?
    await wrapper.find('input[placeholder="输入解压密码"]').setValue('test123')
    await wrapper.find('input[value="overwrite"]').setValue(true)

    // 点击重置按钮
    const resetButton = wrapper.find('button:contains("重置为默�?)')
    await resetButton.trigger('click')

    // 检查confirm是否被调�?
    expect(window.confirm).toHaveBeenCalledWith('确定要重置所有解压设置为默认值吗�?)

    // 检查设置是否重置为默认�?
    expect((wrapper.find('input[placeholder="输入解压密码"]').element as HTMLInputElement).value).toBe('')
    expect(wrapper.find('input[value="ask"]:checked').exists()).toBe(true)
  })

  it('处理禁用状�?, () => {
    const wrapper = mount(DecompressSettingsPanel, {
      props: {
        isProcessing: true
      }
    })

    // 检查所有输入和按钮是否被禁�?
    const inputs = wrapper.findAll('input')
    inputs.forEach(input => {
      expect(input.attributes('disabled')).toBeDefined()
    })

    const buttons = wrapper.findAll('button')
    buttons.forEach(button => {
      expect(button.attributes('disabled')).toBeDefined()
    })
  })

  it('触发设置变化事件', async () => {
    const wrapper = mount(DecompressSettingsPanel)

    // 修改输出路径
    await wrapper.find('input[placeholder="选择解压输出目录"]').setValue('/new/path')

    // 检查事件是否触�?
    expect(wrapper.emitted('settings-change')).toBeTruthy()
    expect(wrapper.emitted('settings-change')?.[0]?.[0]).toHaveProperty('outputPath', '/new/path')
  })

  it('触发v-model更新', async () => {
    const wrapper = mount(DecompressSettingsPanel)

    // 修改密码
    await wrapper.find('input[placeholder="输入解压密码"]').setValue('newpassword')

    // 检查update:modelValue事件是否触发
    expect(wrapper.emitted('update:modelValue')).toBeTruthy()
    const emittedValue = wrapper.emitted('update:modelValue')?.[0]?.[0] as DecompressSettings
    expect(emittedValue.password).toBe('newpassword')
  })

  describe('密码选项', () => {
    it('显示密码选项当有密码�?, async () => {
      const wrapper = mount(DecompressSettingsPanel)

      // 初始状态不应该显示密码选项
      expect(wrapper.find('label:contains("密码尝试设置")').exists()).toBe(false)

      // 输入密码
      await wrapper.find('input[placeholder="输入解压密码"]').setValue('test123')

      // 现在应该显示密码选项
      expect(wrapper.find('label:contains("密码尝试设置")').exists()).toBe(true)
    })

    it('显示最大尝试次数当启用自动尝试�?, async () => {
      const wrapper = mount(DecompressSettingsPanel)

      // 输入密码
      await wrapper.find('input[placeholder="输入解压密码"]').setValue('test123')

      // 启用自动尝试常见密码
      const autoTryCheckbox = wrapper.find('input[type="checkbox"]')
      await autoTryCheckbox.setValue(true)

      // 应该显示最大尝试次数滑�?
      expect(wrapper.find('label:contains("最大尝试次�?)').exists()).toBe(true)
      expect(wrapper.find('input[type="range"]').exists()).toBe(true)
    })
  })

  describe('文件过滤�?, () => {
    it('显示文件过滤器输�?, () => {
      const wrapper = mount(DecompressSettingsPanel)

      const fileFilterInput = wrapper.find('input[placeholder="例如: *.txt, *.jpg, document*"]')
      expect(fileFilterInput.exists()).toBe(true)
    })

    it('可以设置文件过滤�?, async () => {
      const wrapper = mount(DecompressSettingsPanel)

      const fileFilterInput = wrapper.find('input[placeholder="例如: *.txt, *.jpg, document*"]')
      await fileFilterInput.setValue('*.pdf,*.docx')

      expect((fileFilterInput.element as HTMLInputElement).value).toBe('*.pdf,*.docx')
      expect(wrapper.vm.settings.options.fileFilter).toBe('*.pdf,*.docx')
    })
  })

  describe('暴露的方�?, () => {
    it('暴露getSettings方法', () => {
      const wrapper = mount(DecompressSettingsPanel)

      expect(wrapper.vm.getSettings).toBeDefined()
      expect(typeof wrapper.vm.getSettings).toBe('function')

      const settings = wrapper.vm.getSettings()
      expect(settings).toHaveProperty('outputPath')
      expect(settings).toHaveProperty('password')
      expect(settings).toHaveProperty('options')
    })

    it('暴露resetSettings方法', () => {
      const wrapper = mount(DecompressSettingsPanel)

      expect(wrapper.vm.resetSettings).toBeDefined()
      expect(typeof wrapper.vm.resetSettings).toBe('function')
    })

    it('暴露validateSettings方法', () => {
      const wrapper = mount(DecompressSettingsPanel)

      expect(wrapper.vm.validateSettings).toBeDefined()
      expect(typeof wrapper.vm.validateSettings).toBe('function')

      const validation = wrapper.vm.validateSettings()
      expect(validation).toHaveProperty('valid')
      expect(validation.valid).toBe(true)
    })
  })

  describe('样式和类�?, () => {
    it('有正确的容器类名', () => {
      const wrapper = mount(DecompressSettingsPanel)

      expect(wrapper.find('.decompress-settings-panel').exists()).toBe(true)
      expect(wrapper.find('.decompress-settings-panel').classes()).toContain('space-y-4')
    })

    it('输入框有正确的类�?, () => {
      const wrapper = mount(DecompressSettingsPanel)

      const input = wrapper.find('input[placeholder="选择解压输出目录"]')
      expect(input.classes()).toContain('glass-input')
    })

    it('按钮有正确的类名', () => {
      const wrapper = mount(DecompressSettingsPanel)

      const button = wrapper.find('button:contains("快速解�?)')
      expect(button.classes()).toContain('glass-button')
    })
  })
})
