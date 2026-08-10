import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  open: vi.fn(),
  save: vi.fn(),
  message: vi.fn(),
  ask: vi.fn(),
}))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/dialog', () => ({
  open: mocks.open,
  save: mocks.save,
  message: mocks.message,
  ask: mocks.ask,
}))

const decompressionOptions = {
  outputPath: 'C:/output',
  keepStructure: true,
  overwrite: false,
  deleteAfter: false,
} as const

describe('useTauriCommands', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    Object.values(mocks).forEach(mock => mock.mockReset())
    mocks.invoke.mockResolvedValue(undefined)
    mocks.message.mockResolvedValue(undefined)
    vi.spyOn(console, 'error').mockImplementation(() => undefined)
    vi.spyOn(window, 'prompt').mockReturnValue(null)
  })

  it('selects files while skipping metadata entries that cannot be read', async () => {
    mocks.open.mockResolvedValue(['C:/archives/good.zip', 'C:/archives/blocked.zip'])
    mocks.invoke.mockImplementation(async (command: string, payload: { path?: string }) => {
      if (command === 'get_file_info' && payload.path?.endsWith('good.zip')) {
        return { name: 'good.zip', size: 42, is_dir: false, modified: '2026-07-27T00:00:00Z' }
      }
      if (command === 'get_file_info') throw new Error('Access denied')
      return undefined
    })

    const result = await useTauriCommands().selectFiles()

    expect(result).toEqual([{
      path: 'C:/archives/good.zip',
      name: 'good.zip',
      size: 42,
      isDir: false,
      modified: Date.parse('2026-07-27T00:00:00Z'),
    }])
    expect(mocks.open).toHaveBeenCalledWith(expect.objectContaining({
      multiple: true,
      filters: expect.arrayContaining([
        expect.objectContaining({ extensions: expect.arrayContaining(['zip', '7z', 'rar']) }),
      ]),
    }))
    expect(mocks.message).not.toHaveBeenCalled()
  })

  it('reports file and directory picker failures without leaking exceptions', async () => {
    mocks.open.mockRejectedValueOnce(new Error('picker unavailable'))
    const commands = useTauriCommands()

    await expect(commands.selectFiles(false)).resolves.toEqual([])
    expect(mocks.message).toHaveBeenLastCalledWith(
      expect.stringContaining('picker unavailable'),
      { type: 'error' },
    )

    mocks.open.mockRejectedValueOnce('directory permission denied')
    await expect(commands.selectDirectory('C:/start')).resolves.toBeNull()
    expect(mocks.message).toHaveBeenLastCalledWith(
      expect.stringContaining('directory permission denied'),
      { type: 'error' },
    )
  })

  it('returns selected directories and save paths, including cancellation', async () => {
    const commands = useTauriCommands()
    mocks.open.mockResolvedValueOnce('C:/chosen')
    await expect(commands.selectDirectory('C:/start')).resolves.toBe('C:/chosen')
    expect(mocks.open).toHaveBeenLastCalledWith({
      directory: true,
      multiple: false,
      defaultPath: 'C:/start',
    })

    mocks.save.mockResolvedValueOnce('C:/chosen/report.json')
    await expect(commands.saveFile('report.json')).resolves.toBe('C:/chosen/report.json')
    mocks.save.mockResolvedValueOnce(null)
    await expect(commands.saveFile()).resolves.toBeNull()
  })

  it('marks a decompression task failed when the backend reports a disk error', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'extract_file') throw new Error('disk full')
      return undefined
    })
    const appStore = useAppStore()
    appStore.settings.enableBruteForce = true
    appStore.settings.bruteForceWordlists = ['C:/words.txt']
    const commands = useTauriCommands()
    const tasks = useTaskStore()

    await expect(commands.decompressFile(
      'C:/archives/demo.zip',
      decompressionOptions,
    )).rejects.toThrow('disk full')

    expect(tasks.tasks[0]).toMatchObject({
      name: 'demo.zip',
      status: 'failed',
      progress: 0,
    })
    expect(mocks.invoke).toHaveBeenCalledWith('extract_file', expect.objectContaining({
      filePath: 'C:/archives/demo.zip',
      password: null,
      options: {
        preserve_paths: true,
        overwrite_existing: false,
        delete_after: false,
        preserve_timestamps: true,
        skip_corrupted: false,
        extract_only_newer: false,
        create_subdirectory: false,
        preserve_mark_of_web: true,
        file_filter: null,
        selected_entries: [],
        conflict_policy: 'rename',
        enable_bruteforce: true,
        bruteforce_wordlists: ['C:/words.txt'],
      },
    }))
  })

  it('does not overwrite a cancelled task when a late extraction error arrives', async () => {
    const tasks = useTaskStore()
    tasks.addTask({
      id: 'cancelled-task',
      name: 'large.zip',
      type: 'decompression',
      sourceFiles: ['C:/large.zip'],
      outputPath: 'C:/output',
    })
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'extract_file') {
        tasks.updateTaskStatus('cancelled-task', 'cancelled')
        throw new Error('operation cancelled')
      }
      return undefined
    })

    await expect(useTauriCommands().decompressFile(
      'C:/large.zip',
      decompressionOptions,
      'cancelled-task',
    )).rejects.toThrow('operation cancelled')
    expect(tasks.tasks[0].status).toBe('cancelled')
  })

  it('returns per-file results when a decompression batch partially fails', async () => {
    mocks.invoke.mockImplementation(async (command: string, payload: { filePath?: string }) => {
      if (command !== 'extract_file') return undefined
      if (payload.filePath?.endsWith('denied.zip')) throw new Error('permission denied')
      if (payload.filePath?.endsWith('broken.zip')) throw 'corrupt archive'
      return `out:${payload.filePath}`
    })

    const result = await useTauriCommands().decompressFiles([
      { path: 'C:/ok.zip', options: decompressionOptions },
      { path: 'C:/denied.zip', options: decompressionOptions },
      { path: 'C:/broken.zip', options: decompressionOptions },
    ])

    expect(result).toEqual([
      { file: 'C:/ok.zip', success: true, result: 'out:C:/ok.zip' },
      { file: 'C:/denied.zip', success: false, error: 'permission denied' },
      { file: 'C:/broken.zip', success: false, error: 'corrupt archive' },
    ])
  })

  it('sorts directory listings and degrades safely when listing fails', async () => {
    mocks.invoke.mockImplementation(async (command: string, payload: { path?: string }) => {
      if (command !== 'list_files') return undefined
      if (payload.path === 'C:/protected') throw new Error('access denied')
      return [
        { path: 'C:/z.txt', name: 'z.txt', size: 1, isDir: false, modified: 0 },
        { path: 'C:/b', name: 'b', size: 0, isDir: true, modified: 0 },
        { path: 'C:/a', name: 'a', size: 0, isDir: true, modified: 0 },
        { path: 'C:/a.txt', name: 'a.txt', size: 1, isDir: false, modified: 0 },
      ]
    })
    const commands = useTauriCommands()

    await expect(commands.listDirectory('C:/')).resolves.toEqual([
      expect.objectContaining({ name: 'a', isDir: true }),
      expect.objectContaining({ name: 'b', isDir: true }),
      expect.objectContaining({ name: 'a.txt', isDir: false }),
      expect.objectContaining({ name: 'z.txt', isDir: false }),
    ])

    await expect(commands.listDirectory('C:/protected')).resolves.toEqual([])
  })

  it('classifies supported, encrypted, and unreadable archive formats', async () => {
    const commands = useTauriCommands()
    mocks.invoke.mockImplementation(async (command: string, payload: { filePath?: string }) => {
      if (command !== 'list_archive_contents') return undefined
      if (payload.filePath?.endsWith('secure.7z')) {
        throw new Error('Password required for encrypted archive')
      }
      if (payload.filePath?.endsWith('plain.bin')) throw 'unsupported header'
      return ['payload.txt']
    })
    await expect(commands.checkFileFormat('C:/archive.TAR.GZ')).resolves.toEqual({
      supported: true,
      format: 'gz',
      encrypted: false,
    })

    await expect(commands.checkFileFormat('C:/secure.7z')).resolves.toEqual({
      supported: false,
      encrypted: true,
      error: 'Password required for encrypted archive',
    })

    await expect(commands.checkFileFormat('C:/plain.bin')).resolves.toEqual({
      supported: false,
      encrypted: false,
      error: 'unsupported header',
    })
  })

  it('filters duplicate and invalid password wordlists with one warning', async () => {
    mocks.open.mockResolvedValue([
      'C:/words/good.txt',
      'C:/words/good.txt',
      'C:/words/empty.txt',
    ])
    mocks.invoke.mockImplementation(async (command: string) => command === 'validate_wordlists'
      ? [
          { path: 'C:/words/good.txt', valid: true, valid_password_count: 3 },
          { path: 'C:/words/empty.txt', valid: false, valid_password_count: 0, error: 'No passwords' },
        ]
      : undefined)

    await expect(useTauriCommands().selectWordlists()).resolves.toEqual(['C:/words/good.txt'])
    expect(mocks.invoke).toHaveBeenCalledWith('validate_wordlists', {
      paths: ['C:/words/good.txt', 'C:/words/empty.txt'],
    })
    expect(mocks.message).toHaveBeenCalledWith(
      expect.stringContaining('empty.txt: No passwords'),
      expect.objectContaining({ type: 'warning' }),
    )
  })

  it('reports password wordlist validation failures', async () => {
    mocks.open.mockResolvedValue(['C:/words/blocked.txt'])
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'validate_wordlists') throw new Error('Access denied')
      return undefined
    })

    await expect(useTauriCommands().selectWordlists()).resolves.toEqual([])
    expect(mocks.message).toHaveBeenCalledWith(
      'Failed to validate wordlists: Error: Access denied',
      { title: 'Wordlist validation', type: 'error' },
    )
  })

  it('exports an unencrypted password vault with explicit backend options', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'list_encrypted_passwords') return [{ id: 'one' }, { id: 'two' }]
      if (command === 'export_passwords_command') return true
      return undefined
    })
    mocks.ask.mockResolvedValue(false)
    mocks.save.mockResolvedValue('C:/backup/passwords.json')

    await useTauriCommands().exportPasswords()

    expect(mocks.invoke).toHaveBeenCalledWith('export_passwords_command', {
      filePath: 'C:/backup/passwords.json',
      exportPassword: null,
      encrypt: false,
      includePasswords: true,
      includeMetadata: true,
      format: 'Json',
    })
    expect(mocks.message).toHaveBeenLastCalledWith(
      expect.stringContaining('2'),
      expect.objectContaining({ type: 'info' }),
    )
  })

  it('stops encrypted export when the password prompt is cancelled', async () => {
    mocks.invoke.mockImplementation(async (command: string) =>
      command === 'list_encrypted_passwords' ? [{ id: 'one' }] : undefined)
    mocks.ask.mockResolvedValue(true)
    vi.spyOn(window, 'prompt').mockReturnValue(null)

    await useTauriCommands().exportPasswords()

    expect(mocks.save).not.toHaveBeenCalled()
    expect(mocks.invoke).not.toHaveBeenCalledWith('export_passwords_command', expect.anything())
    expect(mocks.message).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({ type: 'info' }),
    )
  })

  it('imports encrypted password data and reports backend failures', async () => {
    mocks.open.mockResolvedValue('C:/backup/passwords.json')
    mocks.ask.mockResolvedValue(true)
    vi.spyOn(window, 'prompt').mockReturnValue('import-secret')
    mocks.invoke.mockImplementation(async (command: string) =>
      command === 'import_passwords_command' ? 4 : undefined)
    const commands = useTauriCommands()

    await commands.importPasswords()
    expect(mocks.invoke).toHaveBeenCalledWith('import_passwords_command', {
      filePath: 'C:/backup/passwords.json',
      importPassword: 'import-secret',
      encrypt: true,
      format: 'Json',
    })
    expect(mocks.message).toHaveBeenLastCalledWith(
      expect.stringContaining('4'),
      expect.objectContaining({ type: 'info' }),
    )

    mocks.open.mockRejectedValueOnce({ message: 'permission denied' })
    await commands.importPasswords()
    expect(mocks.message).toHaveBeenLastCalledWith(
      expect.stringContaining('permission denied'),
      { type: 'error' },
    )
  })

  it('keeps task state truthful for successful and failed cancellation requests', async () => {
    const tasks = useTaskStore()
    tasks.addTask({
      id: 'task-ok',
      name: 'ok.zip',
      type: 'decompression',
      sourceFiles: ['ok.zip'],
      outputPath: 'output',
    })
    tasks.addTask({
      id: 'task-failed',
      name: 'failed.zip',
      type: 'decompression',
      sourceFiles: ['failed.zip'],
      outputPath: 'output',
    })
    const commands = useTauriCommands()

    await commands.cancelCompression('task-ok')
    expect(tasks.tasks[0].status).toBe('cancelled')

    mocks.invoke.mockImplementation(async (command: string, payload: { taskId?: string }) => {
      if (command === 'cancel_compression' && payload.taskId === 'task-failed') {
        throw new Error('backend unavailable')
      }
      return undefined
    })
    await commands.cancelCompression('task-failed')
    expect(tasks.tasks[1].status).toBe('pending')
  })

  it('shows cancelling until the backend confirms cleanup', async () => {
    let finishCancellation!: () => void
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'cancel_compression') {
        return new Promise<void>(resolve => { finishCancellation = resolve })
      }
      return Promise.resolve(undefined)
    })
    const tasks = useTaskStore()
    tasks.addTask({
      id: 'slow-cancel',
      name: 'large.7z',
      type: 'compression',
      sourceFiles: ['large.bin'],
      outputPath: 'large.7z',
    })
    tasks.updateTaskStatus('slow-cancel', 'compressing')

    const pending = useTauriCommands().cancelCompression('slow-cancel')
    await Promise.resolve()
    expect(tasks.tasks[0].status).toBe('cancelling')

    finishCancellation()
    await pending
    expect(tasks.tasks[0].status).toBe('cancelled')
  })

  it('forwards archive and desktop integration commands without changing payloads', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'check_rar_compression_support') {
        return { available: false, message: 'WinRAR missing' }
      }
      if (command === 'get_archive_engine_capabilities') {
        return { available: true, fullEngine: true, formats: [], message: 'ready' }
      }
      return command
    })
    const commands = useTauriCommands()

    await expect(commands.compressFiles('task', ['a.txt'], 'a.zip', { level: 6 })).resolves.toBe('compress_files')
    await expect(commands.checkRarCompressionSupport()).resolves.toEqual({
      available: false,
      message: 'WinRAR missing',
    })
    await expect(commands.getArchiveEngineCapabilities()).resolves.toEqual(
      expect.objectContaining({ available: true, fullEngine: true }),
    )
    await commands.installWinRarWithWinget()
    await commands.openRarDownloadPage()
    await commands.listArchiveContents('a.zip', 'secret')
    await commands.browseArchive('a.zip', 'secret')
    await commands.testArchiveIntegrity('a.zip')
    await commands.analyzeCompressionSources('analysis-1', ['a.txt'], 'zip', 6)
    await commands.cancelCompressionAnalysis('analysis-1')
    await commands.repairZip('a.zip')
    await commands.registerContextMenu()
    await commands.unregisterContextMenu()
    await commands.isContextMenuRegistered()

    expect(mocks.invoke).toHaveBeenCalledWith('list_archive_contents', {
      filePath: 'a.zip',
      password: 'secret',
    })
    expect(mocks.invoke).toHaveBeenCalledWith('browse_archive', {
      filePath: 'a.zip',
      password: 'secret',
    })
    expect(mocks.invoke).toHaveBeenCalledWith('test_archive_integrity', {
      filePath: 'a.zip',
      password: null,
    })
    expect(mocks.invoke).toHaveBeenCalledWith('repair_zip', { filePath: 'a.zip' })
    expect(mocks.invoke).toHaveBeenCalledWith('analyze_compression_sources', {
      analysisId: 'analysis-1',
      paths: ['a.txt'],
      format: 'zip',
      level: 6,
    })
    expect(mocks.invoke).toHaveBeenCalledWith('cancel_compression_analysis', {
      analysisId: 'analysis-1',
    })
  })

  it('degrades optional system UI commands safely when native APIs reject', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'get_system_info') throw new Error('system info unavailable')
      if (command === 'open_in_explorer') throw new Error('Explorer unavailable')
      return undefined
    })
    const commands = useTauriCommands()
    await expect(commands.getSystemInfo()).resolves.toBeNull()

    mocks.message.mockRejectedValueOnce(new Error('dialog unavailable'))
    await expect(commands.showMessage('Title', 'Body', 'warning')).resolves.toBeUndefined()

    mocks.save.mockRejectedValueOnce(new Error('save permission denied'))
    await expect(commands.saveFile()).resolves.toBeNull()

    await expect(commands.openInExplorer('C:/output')).resolves.toBeUndefined()
  })
})
