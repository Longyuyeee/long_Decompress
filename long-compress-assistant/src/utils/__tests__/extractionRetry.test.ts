import { beforeEach, describe, expect, it, vi } from 'vitest'
import { validateExtractionRetry } from '../extractionRetry'
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
beforeEach(() => vi.resetAllMocks())

describe('forum ZIP retry source resolution', () => {
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
