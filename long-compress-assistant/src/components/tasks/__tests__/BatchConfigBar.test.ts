import {describe,expect,it,vi} from 'vitest'
import {mount} from '@vue/test-utils'
import {createPinia} from 'pinia'
import BatchConfigBar from '../BatchConfigBar.vue'
vi.mock('@tauri-apps/api/tauri',()=>({invoke:vi.fn(async()=>'{}')}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
describe('batch ZIP output encoding',()=>{
  it('tarlzo171 batch modal clears irrelevant strength and RAR fields when selecting a folder format',async()=>{
    const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
    await wrapper.get('button[aria-haspopup="dialog"]').trigger('click');await wrapper.get('[data-testid="batch-format"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(o=>o.text().includes('TAR.LZO'))!.trigger('click')
    expect(wrapper.find('[data-testid="batch-level"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="batch-rar-recovery"]').exists()).toBe(false)
    await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
    expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format:'tar.lzo'});wrapper.unmount()
  })
  it('lzop172 applies the multi-file format through the existing modal without irrelevant extraction/strength fields',async()=>{
    const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
    await wrapper.get('button[aria-haspopup="dialog"]').trigger('click');await wrapper.get('[data-testid="batch-format"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(o=>o.text().includes('LZOP'))!.trigger('click')
    expect(wrapper.find('[data-testid="batch-level"]').exists()).toBe(false);expect(wrapper.text()).not.toContain('文件放置方式');for(const id of ['batch-zip-password-encoding','batch-rar-recovery','batch-rar-recovery-volumes'])expect(wrapper.find(`[data-testid="${id}"]`).exists()).toBe(false)
    await wrapper.get('[data-testid="batch-config-apply"]').trigger('click');expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format:'lzo'});wrapper.unmount()
  })
  it('wim1527 configures WIM only through the original selected-row modal',async()=>{
    const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
    expect(wrapper.find('[data-testid="batch-format"]').exists()).toBe(false)
    await wrapper.get('button[aria-haspopup="dialog"]').trigger('click');await wrapper.get('[data-testid="batch-format"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(option=>option.find('strong').text()==='WIM（无加密）')!.trigger('click')
    await wrapper.get('[data-testid="batch-config-apply"]').trigger('click');expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format:'wim'});wrapper.unmount()
  })
  it('zpaq1526 configures selected rows only inside the original modal',async()=>{
    const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
    expect(wrapper.find('[data-testid="batch-format"]').exists()).toBe(false)
    await wrapper.get('button[aria-haspopup="dialog"]').trigger('click');await wrapper.get('[data-testid="batch-format"]').trigger('click')
    await wrapper.findAll('[role="option"]').find(option=>option.find('strong').text()==='ZPAQ')!.trigger('click')
    await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
    expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format:'zpaq'});wrapper.unmount()
  })
  it.each(['cp932','cp936','cp950'])('emits only %s when no other compression setting was changed',async encoding=>{
    const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
    await wrapper.get('button[aria-haspopup="dialog"]').trigger('click')
    await wrapper.get('[data-testid="batch-zip-password-encoding"]').trigger('click')
    expect(wrapper.findAll('[role="option"]').some(o=>o.text().includes('自动兼容'))).toBe(false)
    await wrapper.findAll('[role="option"]').find(o=>o.text().includes(encoding.toUpperCase()))!.trigger('click')
    await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
    expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({zipPasswordEncoding:encoding})
    wrapper.unmount()
  })
})

it('lzma1523 configures selected files only inside the existing modal',async()=>{
  const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
  expect(wrapper.find('[data-testid="batch-format"]').exists()).toBe(false)
  await wrapper.get('button[aria-haspopup="dialog"]').trigger('click')
  await wrapper.get('[data-testid="batch-format"]').trigger('click')
  await wrapper.findAll('[role="option"]').find(o=>o.text().includes('LZMA'))!.trigger('click')
  expect(wrapper.get('[data-testid="batch-lzma-hint"]').text()).toContain('一个普通文件')
  await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
  expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format:'lzma'});wrapper.unmount()
})

it.each(['tar.lzma','lzma.aes','tar.lzma.aes'])('lzma1524 configures %s inside the existing batch modal',async format=>{
  const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
  expect(wrapper.find('[data-testid="batch-format"]').exists()).toBe(false)
  await wrapper.get('button[aria-haspopup="dialog"]').trigger('click')
  await wrapper.get('[data-testid="batch-format"]').trigger('click')
  const label=format.toUpperCase()
  await wrapper.findAll('[role="option"]').find(o=>o.find('strong').text().startsWith(label+'（'))!.trigger('click')
  expect(wrapper.get('[data-testid="batch-lzma-combination-hint"]').text()).toContain(format==='tar.lzma'?'目录':'Long解压')
  await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
  expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format});wrapper.unmount()
})

it('wim1527 store only omits a previously chosen strength instead of applying an ineffective setting',async()=>{
  const wrapper=mount(BatchConfigBar,{props:{count:2,mode:'compression'},global:{plugins:[createPinia()],stubs:{Teleport:true,Transition:false}}})
  await wrapper.get('button[aria-haspopup="dialog"]').trigger('click');await wrapper.get('[data-testid="batch-level"]').trigger('click');await wrapper.findAll('[role="option"]').find(option=>option.text().trim().startsWith('9'))!.trigger('click')
  await wrapper.get('[data-testid="batch-format"]').trigger('click');await wrapper.findAll('[role="option"]').find(option=>option.find('strong').text()==='WIM（无加密）')!.trigger('click');expect(wrapper.get('[data-testid="batch-level"]').attributes('disabled')).toBeDefined()
  await wrapper.get('[data-testid="batch-config-apply"]').trigger('click');expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({format:'wim'});wrapper.unmount()
})
