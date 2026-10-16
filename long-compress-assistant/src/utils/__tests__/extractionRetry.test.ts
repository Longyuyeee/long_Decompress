import { beforeEach, describe, expect, it, vi } from 'vitest'
import { validateExtractionRetry } from '../extractionRetry'
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
beforeEach(() => vi.resetAllMocks())

describe('forum ZIP retry source resolution', () => {
  it('uses the chosen input encoding for required integrity checks and keeps encoding failures distinct', async () => {
    mocks.invoke.mockImplementation(async (cmd, args) => cmd === 'detect_split_archive'
      ? { is_split: false, archive_format: 'ZIP' }
      : cmd === 'get_file_info' ? { name: 'old.zip', is_dir: args.path === 'D:/output', size: 100 }
      : { status: 'password_encoding', integrityTested: false })
    await expect(validateExtractionRetry({ source: 'C:/old.zip', output: 'D:/output', password: '中文', extractToSubfolder: false, zipPasswordEncoding: 'cp932' }, true, 'check-encoding', () => false)).rejects.toThrow('不代表文件损坏')
    expect(mocks.invoke).toHaveBeenCalledWith('diagnose_archive', expect.objectContaining({ zipPasswordEncoding: 'cp932' }))
  })
  const input = { source:'C:/资料.haozip02.zip', output:'D:/本次输出', password:'my-password', extractToSubfolder:true }
  it('does not retry an unconfirmed forum volume as an independent ZIP', async () => {
    mocks.invoke.mockResolvedValue({is_split:false})
    await expect(validateExtractionRetry(input,false,'retry-forum',()=>false)).rejects.toThrow('无法确认分卷组')
    expect(mocks.invoke).toHaveBeenCalledTimes(1)
  })
  it('canonicalizes a complete group without changing the selected output', async () => {
    mocks.invoke.mockImplementation(async (command,args) => command === 'detect_split_archive'
      ? {is_split:true, first_part:'C:/资料.haozip01.zip', is_complete:true, missing_parts:[]}
      : {name:'资料.haozip01.zip',is_dir:args.path===input.output,size:100})
    await expect(validateExtractionRetry(input,false,'retry-forum',()=>false)).resolves.toMatchObject({path:'C:/资料.haozip01.zip',outputPath:input.output})
    expect(input.password).toBe('my-password')
    expect(input.extractToSubfolder).toBe(true)
  })
})
