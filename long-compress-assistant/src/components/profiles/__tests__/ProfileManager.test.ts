import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import ProfileManager from '../ProfileManager.vue'

const mocks = vi.hoisted(() => ({
  profiles: [] as Record<string, unknown>[],
  loadAllProfiles: vi.fn(),
  addProfile: vi.fn(),
  modifyProfile: vi.fn(),
  removeProfile: vi.fn(),
  setError: vi.fn(),
  setSuccess: vi.fn(),
  open: vi.fn(),
  save: vi.fn(),
  exportTaskTemplate: vi.fn(),
  previewTaskTemplate: vi.fn(),
  importTaskTemplate: vi.fn(),
  planTaskTemplateDraft: vi.fn(),
  addTemplateDraft: vi.fn(),
}))

vi.mock('@tauri-apps/api/dialog', () => ({ open: mocks.open, save: mocks.save }))

vi.mock('@/composables/useCompressionProfiles', () => ({
  useCompressionProfiles: () => ({
    exportTaskTemplate: mocks.exportTaskTemplate,
    previewTaskTemplate: mocks.previewTaskTemplate,
    importTaskTemplate: mocks.importTaskTemplate,
    planTaskTemplateDraft: mocks.planTaskTemplateDraft,
  }),
}))

vi.mock('@/stores/compression', () => ({
  useCompressionStore: () => ({ addTemplateDraft: mocks.addTemplateDraft }),
}))

vi.mock('@/stores/compressionProfile', () => ({
  useCompressionProfileStore: () => ({
    sortedProfiles: mocks.profiles,
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
    mocks.profiles.splice(0)
    mocks.loadAllProfiles.mockResolvedValue(undefined)
    mocks.addProfile.mockResolvedValue('profile-id')
  })

  it('previews a bounded template and imports only after explicit confirmation', async () => {
    mocks.open.mockResolvedValue('C:/templates/daily.longtask.json')
    mocks.previewTaskTemplate.mockResolvedValue({
      template: {
        schema: 'long-decompress-task-template',
        version: 1,
        name: '日志归档',
        icon: '📦',
        description: '归档日志文件',
        sourceRules: { mode: 'pattern', includePatterns: ['*.log'], excludePatterns: ['*.tmp'], sizeRangeMib: null },
        targetRule: { mode: 'choose_at_runtime', filenameTemplate: '{name}-{date}' },
        compression: {
          format: '7z',
          level: 7,
          splitArchive: false,
          splitSizeMib: null,
          keepStructure: true,
          verifyAfter: true,
          createSolidArchive: true,
        },
        passwordStrategy: { mode: 'prompt_at_runtime' },
        exportNotes: ['固定密码已替换为执行时询问'],
      },
      warnings: ['自动匹配规则导入后默认保持关闭'],
      contentSha256: 'a'.repeat(64),
    })
    mocks.importTaskTemplate.mockResolvedValue('imported-profile')

    const wrapper = mountManager()
    await flushPromises()
    await wrapper.get('[data-testid="import-task-template"]').trigger('click')
    await flushPromises()

    expect(mocks.previewTaskTemplate).toHaveBeenCalledWith('C:/templates/daily.longtask.json')
    const preview = wrapper.get('[data-testid="task-template-preview"]')
    expect(wrapper.text()).toContain('确认后只创建配置组')
    expect(preview.text()).toContain('固定密码、删除源文件和额外引擎参数')
    expect(preview.text()).toContain('执行时询问密码')
    expect(preview.text()).toContain('自动匹配规则导入后默认保持关闭')
    expect(mocks.importTaskTemplate).not.toHaveBeenCalled()

    await wrapper.get('[data-testid="confirm-task-template-import"]').trigger('click')
    await flushPromises()

    expect(mocks.importTaskTemplate).toHaveBeenCalledWith(
      'C:/templates/daily.longtask.json',
      'a'.repeat(64),
    )
    expect(mocks.loadAllProfiles).toHaveBeenCalledTimes(2)
    expect(mocks.setSuccess).toHaveBeenCalledWith('任务模板已导入为配置组，尚未执行任何压缩任务')
  })

  it('creates an explicit safe draft after previewing accepted and excluded sources', async () => {
    mocks.profiles.push({
      id: 'logs', name: '日志归档', icon: '📦', description: '',
      config: {
        format: '7z', level: 7, password: 'never-copy', splitArchive: false, splitSize: null,
        keepStructure: true, deleteAfter: true, verifyAfter: true,
        createSolidArchive: true, filenameTemplate: '{name}-{date}', extraParams: {},
      },
      stats: { useCount: 0, totalBytesProcessed: 0 }, lastUsedAt: null,
    })
    mocks.open.mockResolvedValue(['C:/logs/keep.log', 'C:/logs/skip.tmp'])
    mocks.planTaskTemplateDraft.mockResolvedValue({
      profileId: 'logs', profileName: '日志归档',
      accepted: [{ path: 'C:/logs/keep.log', name: 'keep.log', size: 12, isDirectory: false }],
      excluded: [{
        candidate: { path: 'C:/logs/skip.tmp', name: 'skip.tmp', size: 3, isDirectory: false },
        reason: '命中排除规则',
      }],
      warnings: ['该计划只会创建压缩草稿，不会启动任务'],
    })
    mocks.addTemplateDraft.mockReturnValue({ id: 'draft-1', addedCount: 1, skippedCount: 0 })

    const wrapper = mountManager()
    await flushPromises()
    await wrapper.get('[data-testid="create-template-draft-logs"]').trigger('click')
    await flushPromises()

    expect(mocks.planTaskTemplateDraft).toHaveBeenCalledWith('logs', [
      'C:/logs/keep.log', 'C:/logs/skip.tmp',
    ])
    expect(wrapper.get('[data-testid="template-draft-plan"]').text()).toContain('命中排除规则')
    expect(mocks.addTemplateDraft).not.toHaveBeenCalled()

    await wrapper.get('[data-testid="confirm-template-draft"]').trigger('click')
    expect(mocks.addTemplateDraft).toHaveBeenCalledWith(
      [expect.objectContaining({ path: 'C:/logs/keep.log' })],
      '日志归档',
      expect.objectContaining({
        format: '7z', password: '', deleteAfter: false, verifyAfter: true,
      }),
    )
    expect(wrapper.emitted('draftCreated')).toHaveLength(1)
    expect(mocks.setSuccess).toHaveBeenCalledWith(expect.stringContaining('尚未开始压缩'))
  })

  it('exports a profile through a user-selected file without exposing a task execution action', async () => {
    mocks.profiles.push({
      id: 'daily',
      name: '日常/ZIP',
      icon: '📦',
      description: '',
      config: {
        format: 'zip', level: 6, password: null, splitArchive: false, splitSize: null,
        keepStructure: true, deleteAfter: false, verifyAfter: true,
        createSolidArchive: false, filenameTemplate: null, extraParams: {},
      },
      stats: { useCount: 0, totalBytesProcessed: 0 },
      lastUsedAt: null,
    })
    mocks.save.mockResolvedValue('C:/templates/daily.longtask.json')
    mocks.exportTaskTemplate.mockResolvedValue({})

    const wrapper = mountManager()
    await flushPromises()
    await wrapper.get('[data-testid="export-task-template-daily"]').trigger('click')
    await flushPromises()

    expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({
      defaultPath: '日常-ZIP.longtask.json',
    }))
    expect(mocks.exportTaskTemplate).toHaveBeenCalledWith(
      'daily',
      'C:/templates/daily.longtask.json',
    )
    expect(mocks.setSuccess).toHaveBeenCalledWith(expect.stringContaining('不包含固定密码'))
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
    expect(wrapper.find('input[type="password"]').exists()).toBe(true)

    await wrapper.get('select').setValue('tar')
    expect(wrapper.text()).not.toContain('将压缩包拆分为多个文件')
    expect(wrapper.text()).not.toContain('提高同类文件压缩率')

    const deleteSource = wrapper.findAll('label').find(label => label.text().includes('完成后删除源文件'))
    const verifyArchive = wrapper.findAll('label').find(label => label.text().includes('压缩完成后校验'))
    expect(deleteSource).toBeTruthy()
    expect(verifyArchive).toBeTruthy()
    await deleteSource!.get('input').setValue(true)
    expect((verifyArchive!.get('input').element as HTMLInputElement).checked).toBe(true)
    expect(verifyArchive!.get('input').attributes('disabled')).toBeDefined()

    await wrapper.get('[data-testid="source-rule-mode"]').setValue('pattern')
    await wrapper.get('[data-testid="include-patterns"]').setValue('*.log\nreport-*.csv')
    await wrapper.get('[data-testid="exclude-patterns"]').setValue('*.tmp, *.bak')

    await form.trigger('submit')
    await flushPromises()

    expect(mocks.addProfile).toHaveBeenCalledWith(expect.objectContaining({
      name: '日常 TAR',
      config: expect.objectContaining({
        format: 'tar',
        splitArchive: false,
        verifyAfter: true,
        createSolidArchive: false,
      }),
      autoApply: {
        enabled: false,
        mode: 'pattern',
        filePatterns: ['*.log', 'report-*.csv'],
        excludePatterns: ['*.tmp', '*.bak'],
        sizeRange: null,
      },
    }))
    expect(mocks.setSuccess).toHaveBeenCalledWith('配置组保存成功')
  })

  it('preserves an existing auto-apply enabled state while editing', async () => {
    mocks.profiles.push({
      id: 'existing', name: '现有配置', icon: '📦', description: '',
      config: {
        format: 'zip', level: 6, password: null, splitArchive: false, splitSize: null,
        keepStructure: true, deleteAfter: false, verifyAfter: true,
        createSolidArchive: false, filenameTemplate: null, extraParams: {},
      },
      autoApply: {
        enabled: true, mode: 'pattern', filePatterns: ['*.log'],
        excludePatterns: ['*.tmp'], sizeRange: null,
      },
      passwordStrategy: { type: 'none' },
      stats: {
        useCount: 0, successCount: 0, failureCount: 0,
        totalFilesProcessed: 0, totalBytesProcessed: 0,
      },
      createdAt: 0,
      lastUsedAt: null,
    })

    const wrapper = mountManager()
    await flushPromises()
    await wrapper.get('button[title="编辑配置组"]').trigger('click')
    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(mocks.modifyProfile).toHaveBeenCalledWith(expect.objectContaining({
      id: 'existing',
      autoApply: expect.objectContaining({
        enabled: true,
        mode: 'pattern',
        filePatterns: ['*.log'],
        excludePatterns: ['*.tmp'],
      }),
    }))
  })
})
