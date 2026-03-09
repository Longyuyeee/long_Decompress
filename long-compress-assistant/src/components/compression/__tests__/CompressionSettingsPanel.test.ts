import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createTestingPinia } from '@pinia/testing'
import CompressionSettingsPanel from '../CompressionSettingsPanel.vue'
import type { CompressionOptions } from '@/stores/compression'

// Mock Tauri commands
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    selectDirectory: vi.fn().mockResolvedValue('/test/output/path')
  })
}))

describe('CompressionSettingsPanel', () => {
  const defaultOptions: CompressionOptions = {
    format: 'zip',
    level: 6,
    password: '',
    filename: '',
    splitArchive: false,
    splitSize: '1024',
    keepStructure: true,
    deleteAfter: false,
    createSolidArchive: false
  }

  const createWrapper = (props = {}) => {
    return mount(CompressionSettingsPanel, {
      global: {
        plugins: [createTestingPinia()],
        stubs: {
          'i': true // Stub icon components
        }
      },
      props: {
        modelValue: defaultOptions,
        outputPath: '/test/path',
        ...props
      }
    })
  }

  it('renders correctly with default props', () => {
    const wrapper = createWrapper()

    expect(wrapper.find('h2').text()).toBe('压缩配置')
    expect(wrapper.find('input[type="text"][placeholder="选择压缩文件保存路径"]').exists()).toBe(true)
    expect(wrapper.find('input[type="text"][placeholder="压缩文件名（可选）"]').exists()).toBe(true)
  })

  it('displays all compression format options', () => {
    const wrapper = createWrapper()
    const formatButtons = wrapper.findAll('button[aria-label^="选择"]')

    expect(formatButtons.length).toBeGreaterThan(0)
    expect(formatButtons[0].text()).toContain('ZIP')
  })

  it('emits format-changed event when format is selected', async () => {
    const wrapper = createWrapper()

    // Find and click the 7z format button
    const formatButtons = wrapper.findAll('button[aria-label^="选择"]')
    const sevenZipButton = formatButtons.find(btn => btn.text().includes('7-Zip'))

    if (sevenZipButton) {
      await sevenZipButton.trigger('click')

      expect(wrapper.emitted('format-changed')).toBeTruthy()
      expect(wrapper.emitted('format-changed')![0]).toEqual(['7z'])
    }
  })

  it('updates compression level when slider is moved', async () => {
    const wrapper = createWrapper()
    const levelSlider = wrapper.find('input[type="range"]')

    await levelSlider.setValue(9)

    expect(wrapper.emitted('update:modelValue')).toBeTruthy()
    const emittedValue = wrapper.emitted('update:modelValue')![0][0] as CompressionOptions
    expect(emittedValue.level).toBe(9)
  })

  it('shows password fields when password is entered', async () => {
    const wrapper = createWrapper()

    // Initially password confirm field should not be visible
    expect(wrapper.find('input[placeholder="确认密码"]').exists()).toBe(false)

    // Set password
    const passwordInput = wrapper.find('input[placeholder="设置压缩密码"]')
    await passwordInput.setValue('test123')

    // Now confirm password field should be visible
    expect(wrapper.find('input[placeholder="确认密码"]').exists()).toBe(true)
  })

  it('shows password mismatch error when passwords do not match', async () => {
    const wrapper = createWrapper()

    // Set password and different confirm password
    const passwordInput = wrapper.find('input[placeholder="设置压缩密码"]')
    await passwordInput.setValue('test123')

    const confirmInput = wrapper.find('input[placeholder="确认密码"]')
    await confirmInput.setValue('different')

    // Error message should be visible
    expect(wrapper.text()).toContain('两次输入的密码不一�?)
  })

  it('toggles password visibility when eye icon is clicked', async () => {
    const wrapper = createWrapper()

    const passwordInput = wrapper.find('input[placeholder="设置压缩密码"]')
    const toggleButton = wrapper.find('button[aria-label="显示密码"]')

    // Initially password should be hidden (type="password")
    expect(passwordInput.attributes('type')).toBe('password')

    // Click to show password
    await toggleButton.trigger('click')

    // Now password should be visible (type="text")
    expect(passwordInput.attributes('type')).toBe('text')
    expect(toggleButton.attributes('aria-label')).toBe('隐藏密码')
  })

  it('disables unsupported options based on selected format', async () => {
    const wrapper = createWrapper()

    // Initially with ZIP format, split archive should be enabled
    const splitArchiveCheckbox = wrapper.find('input[type="checkbox"]:not(:checked)')
    expect(splitArchiveCheckbox.attributes('disabled')).toBeUndefined()

    // Change to TAR format (doesn't support split archive)
    await wrapper.setProps({ modelValue: { ...defaultOptions, format: 'tar' } })

    // Split archive checkbox should be disabled
    const updatedCheckbox = wrapper.find('input[type="checkbox"]:not(:checked)')
    expect(updatedCheckbox.attributes('disabled')).toBe('')
  })

  it('emits output-path-changed event when output path is selected', async () => {
    const wrapper = createWrapper()

    // Mock the selectDirectory function
    const selectButton = wrapper.find('button[aria-label="选择输出路径"]')
    await selectButton.trigger('click')

    // Wait for async operation
    await wrapper.vm.$nextTick()

    expect(wrapper.emitted('update:outputPath')).toBeTruthy()
    expect(wrapper.emitted('update:outputPath')![0]).toEqual(['/test/output/path'])
  })

  it('shows correct file extension for selected format', async () => {
    const wrapper = createWrapper()

    // Default format is ZIP, should show .zip
    expect(wrapper.find('span').text()).toContain('.zip')

    // Change to 7z format
    await wrapper.setProps({ modelValue: { ...defaultOptions, format: '7z' } })

    // Should show .7z
    expect(wrapper.find('span').text()).toContain('.7z')
  })

  it('validates password confirmation correctly', async () => {
    const wrapper = createWrapper()

    // Get the component instance to call validate method
    const component = wrapper.vm as any

    // Test with matching passwords
    wrapper.vm.compressionOptions.password = 'test123'
    wrapper.vm.confirmPassword = 'test123'

    let validation = component.validate()
    expect(validation.valid).toBe(true)

    // Test with non-matching passwords
    wrapper.vm.confirmPassword = 'different'

    validation = component.validate()
    expect(validation.valid).toBe(false)
    expect(validation.error).toBe('两次输入的密码不一�?)
  })

  it('exposes getOptions and getOutputPath methods', async () => {
    const wrapper = createWrapper()
    const component = wrapper.vm as any

    const options = component.getOptions()
    expect(options.format).toBe('zip')
    expect(options.level).toBe(6)

    const outputPath = component.getOutputPath()
    expect(outputPath).toBe('/test/path')
  })
})
