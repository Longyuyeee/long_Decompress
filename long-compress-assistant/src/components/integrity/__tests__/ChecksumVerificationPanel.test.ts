import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import Panel from '../ChecksumVerificationPanel.vue'
import type { ChecksumEntryStatus, ChecksumVerificationReport } from '@/types/checksum'
const mocks=vi.hoisted(()=>({open:vi.fn(),invoke:vi.fn(),copy:vi.fn()}))
vi.mock('@tauri-apps/api/dialog',()=>({open:mocks.open}))
vi.mock('@/composables/useTauriCommands',()=>({useTauriCommands:()=>({invoke:mocks.invoke})}))
const result=(statuses:ChecksumEntryStatus[],id='fixed'):ChecksumVerificationReport=>({
  checksumPath:'C:/校验.sha256',manifestId:id,valid:statuses.length>0&&statuses.every(s=>s==='matched'),message:'本次结果',
  entries:statuses.map((status,i)=>({line:i+1,fileName:`资料.zip.00${i+1}`,path:`C:/资料.zip.00${i+1}`,status,algorithm:'sha256',expectedChecksum:'a'.repeat(64),actualChecksum:status==='matched'?'a'.repeat(64):null,reason:`真实原因 ${status}`,checkedAt:status==='not_checked'?null:1000})),
})
const start=async(statuses:ChecksumEntryStatus[])=>{
  mocks.open.mockResolvedValue('C:/校验.sha256');mocks.invoke.mockResolvedValueOnce(result(statuses))
  const wrapper=mount(Panel,{global:{plugins:[createPinia()]}})
  await wrapper.get('[data-testid="checksum-choose"]').trigger('click');await flushPromises();return wrapper
}
beforeEach(()=>{vi.clearAllMocks();Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:mocks.copy}});mocks.copy.mockResolvedValue(undefined)})
describe('per-file checksum workspace',()=>{
  it('shows distinct causes and opens one inline row with equal digest columns',async()=>{
    const w=await start(['matched','missing','mismatch','unreadable'])
    expect(w.get('[data-testid="checksum-counts"]').text()).toContain('3 待处理')
    await w.findAll('[data-testid="checksum-row"]')[1].trigger('click')
    expect(w.get('[data-testid="checksum-details"]').text()).toContain('C:/资料.zip.002')
    expect(w.findAll('.checksum-digests>div')).toHaveLength(2)
    expect(w.find('[role="dialog"]').exists()).toBe(false);w.unmount()
  })
  it('requests only problem rows and preserves the earlier matched timestamp',async()=>{
    const w=await start(['matched','missing','unreadable'])
    mocks.invoke.mockResolvedValueOnce(result(['not_checked','matched','matched']))
    await w.get('[data-testid="checksum-recheck-problems"]').trigger('click');await flushPromises()
    expect(mocks.invoke).toHaveBeenLastCalledWith('verify_checksum_file',{checksumPath:'C:/校验.sha256',entryLines:[2,3]})
    expect(w.get('[data-testid="checksum-heading"]').text()).toBe('清单项目均匹配')
    expect(w.get('[data-testid="checksum-notice"]').text()).toContain('未重复读取');w.unmount()
  })
  it('uses only checked selections and never promotes other unknown rows',async()=>{
    const w=await start(['missing','missing'])
    await w.findAll('input[type="checkbox"]')[1].setValue(true)
    mocks.invoke.mockResolvedValueOnce(result(['not_checked','matched']))
    await w.get('[data-testid="checksum-recheck-selected"]').trigger('click');await flushPromises()
    expect(mocks.invoke).toHaveBeenLastCalledWith('verify_checksum_file',{checksumPath:'C:/校验.sha256',entryLines:[2]})
    expect(w.get('[data-testid="checksum-heading"]').text()).toBe('还有文件需要处理');w.unmount()
  })
  it('invalidates old matches when a manifest changes instead of merging unchecked rows',async()=>{
    const w=await start(['matched','missing'])
    mocks.invoke.mockResolvedValueOnce(result(['not_checked','matched'],'new manifest'))
    await w.get('[data-testid="checksum-recheck-problems"]').trigger('click');await flushPromises()
    expect(w.get('[data-testid="checksum-heading"]').text()).toBe('还有项目未检查')
    expect(w.get('[data-testid="checksum-notice"]').text()).toContain('旧结果已作废');w.unmount()
  })
  it('copies only downloadable problems and hands off diagnosis/queue without starting tasks',async()=>{
    const w=await start(['missing','mismatch','unreadable'])
    await w.get('[data-testid="checksum-copy-pending"]').trigger('click');await flushPromises()
    expect(mocks.copy).toHaveBeenCalledWith('资料.zip.001\n资料.zip.002')
    await w.findAll('[data-testid="checksum-row"]')[1].trigger('click');await w.get('[data-testid="checksum-diagnose"]').trigger('click')
    expect(w.emitted('diagnose')).toEqual([['C:/资料.zip.002']])
    await w.get('[data-testid="checksum-go-tasks"]').trigger('click');expect(w.emitted('tasks')).toHaveLength(1)
    expect(mocks.invoke).toHaveBeenCalledTimes(1);w.unmount()
  })
  it('retains old observations after an API failure but never shows a new success',async()=>{
    const w=await start(['matched'])
    mocks.invoke.mockRejectedValueOnce(Error('清单已移动'))
    await w.get('[data-testid="checksum-recheck-all"]').trigger('click');await flushPromises()
    expect(w.get('[data-testid="checksum-heading"]').text()).toBe('本次检查未完成')
    expect(w.get('[data-testid="checksum-error"]').text()).toContain('上次结果')
    expect(w.get('.checksum-summary').attributes('data-valid')).toBe('false');w.unmount()
  })
})
