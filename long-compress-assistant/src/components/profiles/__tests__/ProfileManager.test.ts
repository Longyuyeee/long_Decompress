import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import ProfileManager from '../ProfileManager.vue'

const mocks = vi.hoisted(() => ({
  loadAllProfiles: vi.fn(),
  addProfile: vi.fn(),
  modifyProfile: vi.fn(),
  removeProfile: vi.fn(),
  setError: vi.fn(),
  setSuccess: vi.fn(),
}))

vi.mock('@/stores/compressionProfile', () => ({
  useCompressionProfileStore: () => ({
    sortedProfiles: [],
    loading: false,
    loadAllProfiles: mocks.loadAllProfiles,
    addProfile: mocks.addProfile,
    modifyProfile: mocks.modifyProfile,
    removeProfile: mocks.removeProfile,
  }),
}))

vi.mock('@/stores/app', () => ({
  useAppStore: () => ({
    t: (key: string) => ({
      'profiles.add_new': '新建配置组',
      'profiles.edit': '编辑配置组',
      'profiles.name_required': '请输入配置组名称',
      'profiles.save_success': '配置组保存成功',
      'common.success': '操作成功',
    }[key] || key),
    setError: mocks.setError,
    setSuccess: mocks.setSuccess,
  }),
}))

const mountManager = () => mount(ProfileManager, {
  global: {
    stubs: {
      Teleport: true,
      Transition: false,
    },
  },
})

describe('ProfileManager', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.loadAllProfiles.mockResolvedValue(undefined)
    mocks.addProfile.mockResolvedValue('profile-id')
  })

  it('validates the name and saves a capability-aligned profile', async () => {
    const wrapper = mountManager()
    await flushPromises()

    const createButton = wrapper.findAll('button').find(button => button.text().includes('新建配置组'))
    expect(createButton).toBeTruthy()
    await createButton!.trigger('click')

    const form = wrapper.find('form')
    expect(form.exists()).toBe(true)
    await form.trigger('submit')
    expect(wrapper.get('[role="alert"]').text()).toContain('请输入配置组名称')
    expect(mocks.addProfile).not.toHaveBeenCalled()

    await wrapper.get('input[placeholder="例如：日常 ZIP、最大压缩"]').setValue('日常 TAR')
    expect(wrapper.find('input[type="password"]').exists()).toBe(true)
    await wrapper.get('select').setValue('rar')
    expect(wrapper.find('input[type="password"]').exists()).toBe(false)

    await wrapper.get('select').setValue('tar')
    expect(wrapper.text()).not.toContain('将压缩包拆分为多个文件')
    expect(wrapper.text()).not.toContain('提高同类文件压缩率')

    await form.trigger('submit')
    await flushPromises()

    expect(mocks.addProfile).toHaveBeenCalledWith(expect.objectContaining({
      name: '日常 TAR',
      config: expect.objectContaining({
        format: 'tar',
        splitArchive: false,
        createSolidArchive: false,
      }),
    }))
    expect(mocks.setSuccess).toHaveBeenCalledWith('配置组保存成功')
  })
})
