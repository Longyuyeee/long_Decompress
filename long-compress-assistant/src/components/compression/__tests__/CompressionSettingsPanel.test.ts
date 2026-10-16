import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import CompressionSettingsPanel from '../CompressionSettingsPanel.vue'
import type { CompressionOptions } from '@/stores/compression'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: (...args: unknown[]) => invoke(...args) }))

const vaultEntry = {
  id: 'vault-1',
  name: '常用归档密码',
  password: 'Archive!2026',
  notes: '项目文件',
  tags: ['archive'],
  category: 'Work',
  created_at: '2026-09-01T00:00:00Z',
  updated_at: '2026-09-01T00:00:00Z',
  last_used: null,
  favorite: true,
  use_count: 3,
  usage_history: {},
}

const settings = (overrides: Partial<CompressionOptions> = {}): CompressionOptions => ({
  format: 'zip',
  level: 6,
  password: '',
  filename: 'archive',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  verifyAfter: true,
  createSolidArchive: false,
  ...overrides,
})

const mountPanel = (modelValue: CompressionOptions, allowSplitArchive = true) => mount(CompressionSettingsPanel, {
  props: { modelValue, allowSplitArchive },
  global: {
    plugins: [createPinia()],
    stubs: {
      PasswordGeneratorDialog: true,
      ProfileSelector: true,
      ProfileManager: true,
      Teleport: true,
      Transition: false,
    },
  },
})

describe('CompressionSettingsPanel split archive settings', () => {
  it.each(['cp932','cp936','cp950'] as const)('keeps %s through the real output encoding select without resetting password or split settings', async encoding => {
    const wrapper=mountPanel(settings({password:'test password',splitArchive:true}));await flushPromises()
    await wrapper.get('[data-testid="zip-password-encoding"]').trigger('click')
    const option=wrapper.findAll('[role="option"]').find(o=>o.text().includes(encoding.toUpperCase()))!
    expect(option).toBeDefined();await option.trigger('click');await flushPromises()
    expect(wrapper.emitted('update:modelValue')?.at(-1)?.[0]).toMatchObject({zipPasswordEncoding:encoding,password:'test password',splitArchive:true})
    wrapper.unmount()
  })
  it('keeps inherited ZIP defaults until I change its method and preserves ZIPX password/split controls', async () => {
    const wrapper = mountPanel(settings({ splitArchive: true, password: 'secret' }))
    await flushPromises()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    await wrapper.get('[data-testid="compression-zip-method"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(option => option.text().includes('Zstandard'))!.trigger('click')
    await flushPromises()
    expect(wrapper.emitted('update:modelValue')?.at(-1)?.[0]).toMatchObject({ format: 'zip', zipMethod: 'zstd', password: 'secret', splitArchive: true })
    await wrapper.setProps({ modelValue: settings({ format: 'zipx', password: 'secret', splitArchive: true, zipSplitMode: 'standard' }) })
    expect(wrapper.get('[data-testid="compression-zip-method"]').text()).toContain('XZ')
    expect(wrapper.get('[data-testid="compression-filename-extension"]').text()).toBe('.zipx')
    expect(wrapper.get('[data-testid="compression-zip-split-mode"]').text()).toContain('.zipx.001')
    expect(wrapper.find('[data-testid="zip-encryption"]').exists()).toBe(true)
    wrapper.unmount()
  })
  it('does not detach valid inherited settings merely by opening a LZIP row', async () => {
    const wrapper = mountPanel(settings({ format: 'tar.lz' }))
    await flushPromises()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    wrapper.unmount()
  })
  it.each(['lz', 'tar.lz'])('clears inherited encryption and split settings when opening %s', async format => {
    const wrapper = mountPanel(settings({ format, password: 'previous ZIP password', splitArchive: true, createSolidArchive: true, encryptFilenames: true }))
    await flushPromises()
    const changed = wrapper.emitted('update:modelValue')?.at(-1)?.[0]
    expect(changed).toMatchObject({ format, password: '', splitArchive: false, createSolidArchive: false, encryptFilenames: false })
    expect(wrapper.get('[data-testid="compression-split-toggle"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-testid="compression-password-input"]').attributes('disabled')).toBeDefined()
    wrapper.unmount()
  })
  it('keeps RAR recovery protection collapsed until selected and emits it without changing other options', async () => {
    const wrapper=mountPanel(settings({format:'rar'})); await flushPromises()
    expect(wrapper.get('[data-testid="rar-recovery-settings"]').attributes('open')).toBeUndefined()
    await wrapper.get('[data-testid="rar-recovery-percent"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(option=>option.text().includes('5%'))!.trigger('click'); await flushPromises()
    expect(wrapper.emitted('update:modelValue')?.at(-1)?.[0]).toMatchObject({format:'rar',rarRecoveryPercent:5,password:'',keepStructure:true})
    await wrapper.get('[data-testid="rar-recovery-volumes"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(option=>option.text()==='2 个恢复卷')!.trigger('click'); await flushPromises()
    expect(wrapper.emitted('update:modelValue')?.at(-1)?.[0]).toMatchObject({format:'rar',rarRecoveryVolumes:2,password:'',keepStructure:true})
    await wrapper.setProps({modelValue:settings({format:'zip'})})
    expect(wrapper.find('[data-testid="rar-recovery-settings"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('keeps legacy numeric ZIP volumes by default and exposes standard disks as an explicit choice', async () => {
    const wrapper = mountPanel(settings({ splitArchive: true }))
    await flushPromises()
    const select = wrapper.get('[data-testid="compression-zip-split-mode"]')
    expect(select.text()).toContain('.zip.001')
    await select.trigger('click')
    await wrapper.findAll('[role="option"]').find(item => item.text().includes('标准 ZIP'))!.trigger('click')
    await flushPromises()
    expect(wrapper.emitted('update:modelValue')?.at(-1)?.[0]).toMatchObject({ zipSplitMode: 'standard', splitArchive: true })
    await wrapper.setProps({ modelValue: settings({ format: '7z', splitArchive: true }) })
    expect(wrapper.find('[data-testid="compression-zip-split-mode"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('dismisses vault suggestions when the user types their own password', async () => {
    const wrapper = mountPanel(settings())
    await flushPromises()
    const input = wrapper.get('[data-testid="compression-password-input"]')
    await input.trigger('focus')
    expect(wrapper.find('.password-vault-menu').exists()).toBe(true)
    await input.setValue('my-password')
    await vi.waitFor(() => expect(wrapper.find('.password-vault-menu').exists()).toBe(false))
    wrapper.unmount()
  })
  it('defaults to AES and exposes weaker ZipCrypto only as an explicit selection', async () => {
    const wrapper = mountPanel(settings({ password: 'secret' }))
    await flushPromises()
    const select = wrapper.get('[data-testid="zip-encryption"]')
    expect(select.text()).toContain('AES-256')
    await select.trigger('click')
    const choice = wrapper.findAll('[role="option"]').find(item => item.text().includes('ZipCrypto'))
    expect(choice).toBeTruthy()
    await choice!.trigger('click'); await flushPromises()
    expect(wrapper.emitted('update:modelValue')?.at(-1)?.[0]).toMatchObject({ zipEncryption: 'zipcrypto' })
    wrapper.unmount()
  })
  it('offers opt-in filename encryption only for 7Z and clears it on format change', async () => {
    const wrapper = mountPanel(settings({ format: '7z' }))
    await flushPromises()
    const toggle = wrapper.get('[data-testid="compression-encrypt-filenames"]')
    expect((toggle.element as HTMLInputElement).checked).toBe(false)
    await toggle.setValue(true)
    expect(wrapper.emitted('update:modelValue')?.at(-1)?.[0]).toMatchObject({ encryptFilenames: true })
    await wrapper.setProps({ modelValue: settings({ format: 'zip', encryptFilenames: true }) })
    await flushPromises()
    expect(wrapper.find('[data-testid="compression-encrypt-filenames"]').exists()).toBe(false)
    wrapper.unmount()
  })
  beforeEach(() => {
    invoke.mockReset()
    invoke.mockImplementation(async (command: string) => {
      if (command === 'get_archive_engine_capabilities') {
        return {
          available: true,
          fullEngine: true,
          formats: [],
          browseExtensions: [],
          nestedExtensions: [],
          boundedPreviewFormats: [],
          imagePreviewExtensions: [],
          textPreviewExtensions: [],
          message: '',
        }
      }
      if (command === 'is_encrypted_password_service_unlocked') return true
      if (command === 'list_encrypted_passwords') return [vaultEntry]
      if (command === 'increment_encrypted_password_use_count') return { ...vaultEntry, use_count: 4 }
      return '{}'
    })
  })

  it('shows and edits split ZIP settings directly in each configuration panel', async () => {
    const wrapper = mountPanel(settings())
    await flushPromises()

    const section = wrapper.get('[data-testid="compression-split-settings"]')
    expect(section.text()).toContain('分卷压缩')
    expect(section.text()).toContain('关闭')

    await wrapper.get('[data-testid="compression-split-toggle"]').setValue(true)
    const size = wrapper.get('[data-testid="compression-split-size"]')
    expect((size.element as HTMLInputElement).value).toBe('1024')
    await size.setValue('512')

    const emitted = wrapper.emitted('update:modelValue')?.at(-1)?.[0] as CompressionOptions
    expect(emitted.splitArchive).toBe(true)
    expect(emitted.splitSize).toBe('512')
  })

  it('removes fast packing even when an old configuration enables it', async () => {
    const wrapper = mountPanel(settings({ fastPack: true }))
    expect(wrapper.find('[data-testid="compression-fast-pack"]').exists()).toBe(false)
    await wrapper.setProps({ modelValue: settings({ splitArchive: true, fastPack: true }) })
    expect(wrapper.text()).not.toContain('快速打包')
    wrapper.unmount()
  })

  it('keeps the setting visible and explains why it is unavailable', async () => {
    const wrapper = mountPanel(settings(), false)
    await flushPromises()

    const toggle = wrapper.get('[data-testid="compression-split-toggle"]')
    expect(toggle.attributes('disabled')).toBeDefined()
    await wrapper.get('button[aria-label="分卷压缩说明"]').trigger('click')
    expect(wrapper.get('[data-testid="compression-split-settings"]').text()).toContain('请先添加需要压缩的文件或文件夹')

    await wrapper.setProps({
      allowSplitArchive: true,
      modelValue: settings({ password: 'secret' }),
    })
    await flushPromises()
    expect(toggle.attributes('disabled')).toBeUndefined()
  })

  it.each(['zip', '7z'])('preserves %s split settings when entering, changing or clearing a password', async (format) => {
    const wrapper = mountPanel(settings({ format, splitArchive: true, splitSize: '128' }))
    await flushPromises()
    for (const password of ['中文密码 + !测试', 'second secret', '']) {
      await wrapper.get('[data-testid="compression-password-input"]').setValue(password)
      await flushPromises()
      const emitted = wrapper.emitted('update:modelValue')?.at(-1)?.[0] as CompressionOptions
      expect(emitted).toMatchObject({ password, splitArchive: true, splitSize: '128' })
      expect(wrapper.get('[data-testid="compression-split-toggle"]').attributes('disabled')).toBeUndefined()
    }
    await wrapper.setProps({ modelValue: settings({ format, password: 'template secret', splitArchive: true, splitSize: '256' }) })
    await flushPromises()
    expect((wrapper.get('[data-testid="compression-split-toggle"]').element as HTMLInputElement).checked).toBe(true)
    expect((wrapper.get('[data-testid="compression-split-size"]').element as HTMLInputElement).value).toBe('256')
  })

  it('opens the password vault list on focus and fills the selected password', async () => {
    const wrapper = mountPanel(settings())
    await flushPromises()

    await wrapper.get('[data-testid="compression-password-input"]').trigger('focus')
    const menu = wrapper.get('[data-testid="compression-password-vault-menu"]')
    expect(menu.text()).toContain('常用归档密码')
    expect(menu.text()).not.toContain('Archive!2026')

    await wrapper.get('[data-testid="compression-vault-password-vault-1"]').trigger('click')
    await flushPromises()
    const emitted = wrapper.emitted('update:modelValue')?.at(-1)?.[0] as CompressionOptions
    expect(emitted.password).toBe('Archive!2026')
    expect(invoke).toHaveBeenCalledWith('increment_encrypted_password_use_count', { id: 'vault-1' })
    await new Promise(resolve => window.setTimeout(resolve, 180))
    expect(wrapper.find('[data-testid="compression-password-vault-menu"]').exists()).toBe(false)
  })
})
