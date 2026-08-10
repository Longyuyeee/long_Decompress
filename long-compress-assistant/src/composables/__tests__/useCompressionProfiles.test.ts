import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { CompressionProfile, CreateProfileRequest, PasswordStrategy } from '@/types'
import { useCompressionProfiles } from '@/composables/useCompressionProfiles'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

const rawProfile = (passwordStrategy: unknown = 'none') => ({
  id: 'profile-1',
  name: 'Daily ZIP',
  icon: '📦',
  description: 'daily archive',
  config: {
    format: 'zip',
    level: 7,
    password: null,
    split_archive: true,
    split_size: 128,
    keep_structure: false,
    delete_after: true,
    verify_after: true,
    create_solid_archive: true,
    filename_template: '{name}-{date}',
    extra_params: { threads: '4' },
  },
  auto_apply: {
    enabled: true,
    mode: 'pattern',
    file_patterns: ['*.log'],
    size_range: [1, 512],
  },
  password_strategy: passwordStrategy,
  stats: {
    use_count: 9,
    success_count: 8,
    failure_count: 1,
    total_files_processed: 42,
    total_bytes_processed: 4096,
  },
  created_at: 100,
  last_used_at: 200,
})

const profile = (passwordStrategy: PasswordStrategy = { type: 'none' }): CompressionProfile => ({
  id: 'profile-1',
  name: 'Daily ZIP',
  icon: '📦',
  description: 'daily archive',
  config: {
    format: 'zip',
    level: 7,
    password: null,
    splitArchive: true,
    splitSize: 128,
    keepStructure: false,
    deleteAfter: true,
    verifyAfter: true,
    createSolidArchive: true,
    filenameTemplate: '{name}-{date}',
    extraParams: { threads: '4' },
  },
  autoApply: {
    enabled: true,
    mode: 'pattern' as never,
    filePatterns: ['*.log'],
    sizeRange: [1, 512],
  },
  passwordStrategy,
  stats: {
    useCount: 9,
    successCount: 8,
    failureCount: 1,
    totalFilesProcessed: 42,
    totalBytesProcessed: 4096,
  },
  createdAt: 100,
  lastUsedAt: 200,
})

describe('useCompressionProfiles', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('normalizes snake_case profiles and all password strategy variants', async () => {
    mocks.invoke.mockResolvedValueOnce([
      rawProfile('none'),
      rawProfile('fixed'),
      rawProfile({ from_vault: { category_id: 'private' } }),
      rawProfile({ auto_generate: { length: 24, save_to_vault: true } }),
    ])

    const result = await useCompressionProfiles().getAllProfiles()

    expect(mocks.invoke).toHaveBeenCalledWith('get_compression_profiles')
    expect(result[0]).toEqual(profile({ type: 'none' }))
    expect(result[1].passwordStrategy).toEqual({ type: 'fixed' })
    expect(result[2].passwordStrategy).toEqual({ type: 'from_vault', categoryId: 'private' })
    expect(result[3].passwordStrategy).toEqual({
      type: 'auto_generate',
      length: 24,
      saveToVault: true,
    })
  })

  it('applies safe defaults to partial camelCase profiles', async () => {
    mocks.invoke.mockResolvedValueOnce({
      id: 'profile-2',
      name: 'Minimal',
      icon: 'M',
      description: '',
      config: { format: '7z', level: 5 },
      autoApply: {},
      passwordStrategy: { type: 'auto_generate' },
    })

    const result = await useCompressionProfiles().getProfileById('profile-2')

    expect(mocks.invoke).toHaveBeenCalledWith('get_compression_profile', { id: 'profile-2' })
    expect(result).toMatchObject({
      config: {
        password: null,
        splitArchive: false,
        splitSize: null,
        keepStructure: true,
        deleteAfter: false,
        verifyAfter: true,
        createSolidArchive: false,
        filenameTemplate: null,
        extraParams: {},
      },
      autoApply: {
        enabled: false,
        mode: 'none',
        filePatterns: [],
        sizeRange: null,
      },
      passwordStrategy: { type: 'auto_generate', length: 16, saveToVault: false },
      stats: {
        useCount: 0,
        successCount: 0,
        failureCount: 0,
        totalFilesProcessed: 0,
        totalBytesProcessed: 0,
      },
      createdAt: 0,
      lastUsedAt: null,
    })

    mocks.invoke.mockResolvedValueOnce(null)
    await expect(useCompressionProfiles().getProfileById('missing')).resolves.toBeNull()
  })

  it('creates, updates, deletes, records, and suggests profiles with backend-safe payloads', async () => {
    const api = useCompressionProfiles()
    const createRequest: CreateProfileRequest = {
      name: 'New',
      icon: 'N',
      description: '',
      config: profile().config,
    }

    mocks.invoke.mockResolvedValueOnce('new-id')
    await expect(api.createProfile(createRequest)).resolves.toBe('new-id')
    expect(mocks.invoke).toHaveBeenLastCalledWith('create_compression_profile', {
      profile: createRequest,
    })

    const strategies: Array<[PasswordStrategy, unknown]> = [
      [{ type: 'none' }, 'none'],
      [{ type: 'fixed' }, 'fixed'],
      [{ type: 'from_vault', categoryId: 'secure' }, { from_vault: { category_id: 'secure' } }],
      [
        { type: 'auto_generate', length: 20, saveToVault: true },
        { auto_generate: { length: 20, save_to_vault: true } },
      ],
    ]
    for (const [strategy, backendStrategy] of strategies) {
      mocks.invoke.mockResolvedValueOnce(undefined)
      await api.updateProfile(profile(strategy))
      expect(mocks.invoke).toHaveBeenLastCalledWith(
        'update_compression_profile',
        expect.objectContaining({
          id: 'profile-1',
          profile: expect.objectContaining({
            password_strategy: backendStrategy,
            config: expect.objectContaining({
              split_archive: true,
              keep_structure: false,
              delete_after: true,
              verify_after: true,
            }),
            auto_apply: expect.objectContaining({
              file_patterns: ['*.log'],
              size_range: [1, 512],
            }),
          }),
        }),
      )
    }

    mocks.invoke.mockResolvedValueOnce(undefined)
    await api.deleteProfile('profile-1')
    expect(mocks.invoke).toHaveBeenLastCalledWith('delete_compression_profile', {
      id: 'profile-1',
    })

    mocks.invoke.mockResolvedValueOnce(undefined)
    await api.recordProfileUsage({
      profile_id: 'profile-1',
      success: true,
      files_count: 3,
      bytes_processed: 2048,
    })
    expect(mocks.invoke).toHaveBeenLastCalledWith('apply_compression_profile', {
      profileId: 'profile-1',
      success: true,
      filesCount: 3,
      bytesProcessed: 2048,
    })

    mocks.invoke.mockResolvedValueOnce(rawProfile({ from_vault: { category_id: 'suggested' } }))
    const suggestion = await api.suggestProfile({
      file_path: 'C:/input/sample.log',
      file_size: 2048,
    })
    expect(mocks.invoke).toHaveBeenLastCalledWith('suggest_compression_profile', {
      filePath: 'C:/input/sample.log',
      fileSize: 2048,
    })
    expect(suggestion?.passwordStrategy).toEqual({
      type: 'from_vault',
      categoryId: 'suggested',
    })

    mocks.invoke.mockResolvedValueOnce(null)
    await expect(api.suggestProfile({ file_path: 'C:/none', file_size: 0 })).resolves.toBeNull()
  })

  it('normalizes every command failure and keeps initialization backward compatible', async () => {
    const error = new Error('backend exploded')
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    const consoleWarn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const api = useCompressionProfiles()
    const calls = [
      () => api.getAllProfiles(),
      () => api.getProfileById('profile-1'),
      () => api.createProfile({
        name: 'New',
        icon: 'N',
        description: '',
        config: profile().config,
      }),
      () => api.updateProfile(profile()),
      () => api.deleteProfile('profile-1'),
      () => api.recordProfileUsage({
        profile_id: 'profile-1',
        success: false,
        files_count: 1,
        bytes_processed: 0,
      }),
      () => api.suggestProfile({ file_path: 'C:/input', file_size: 1 }),
    ]

    for (const call of calls) {
      mocks.invoke.mockRejectedValueOnce(error)
      await expect(call()).rejects.toThrow('backend exploded')
    }
    expect(consoleError).toHaveBeenCalledTimes(calls.length)

    await expect(api.initializeDefaultProfiles()).resolves.toBeUndefined()
    expect(consoleWarn).toHaveBeenCalledOnce()
  })
})
