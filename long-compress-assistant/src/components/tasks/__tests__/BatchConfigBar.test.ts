import {describe,expect,it,vi} from 'vitest'
import {mount} from '@vue/test-utils'
import {createPinia} from 'pinia'
import BatchConfigBar from '../BatchConfigBar.vue'
vi.mock('@tauri-apps/api/tauri',()=>({invoke:vi.fn(async()=>'{}')}))
vi.mock('@tauri-apps/api/dialog',()=>({open:vi.fn()}))
describe('batch ZIP output encoding',()=>{
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
