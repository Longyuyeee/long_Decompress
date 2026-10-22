import {expect,it} from 'vitest'
import {mount,nextTick} from '@vue/test-utils'
import ZpaqBlockDetails from '../ZpaqBlockDetails.vue'
import {zpaqCheckCounts,zpaqReportLines,type ZpaqBlockReport} from '@/types/zpaqBlocks'
function report():ZpaqBlockReport{return {snapshot:{version:3,date:20261020150000},revision:'a'.repeat(64),blocks:[{id:1,offset:100,compressedSize:100,state:'failed',files:2},{id:2,offset:200,compressedSize:100,state:'verified',files:53},{id:3,offset:300,compressedSize:100,state:'unchecked',files:1}],entries:Array.from({length:56},(_,i)=>({path:`目录/文件-${i}.txt`,size:i,state:i<2?'affected':i===55?'unchecked':'verified',blocks:[i<2?1:i===55?3:2]}))}}
function create(value=report()){return mount(ZpaqBlockDetails,{props:{report:value},global:{stubs:{Modal:{props:['visible'],template:'<div v-if="visible"><slot/><slot name="footer"/></div>'}}}})}
it('zpaq163 shared block links navigate to precisely the affected files without mutating the report',async()=>{
 const data=report(),before=JSON.stringify(data),w=create(data);await w.get('[data-testid="zpaq-block-details-open"]').trigger('click');expect(w.findAll('[role="listitem"]')).toHaveLength(2)
 await w.findAll('[data-testid="zpaq-file-blocks"]')[0].trigger('click');expect(w.findAll('[role="listitem"]')).toHaveLength(1);expect(w.get('[role="listitem"]').text()).toContain('数据块 #1');expect(w.get('[role="listitem"]').text()).toContain('校验失败')
 await w.get('[data-testid="zpaq-block-files"]').trigger('click');expect(w.findAll('[role="listitem"]')).toHaveLength(2);expect(w.get('[data-testid="zpaq-block-clear-scope"]').exists()).toBe(true);expect(JSON.stringify(data)).toBe(before);w.unmount()
})
it('zpaq163 long results paginate and search without promoting unchecked files to passed',async()=>{
 const w=create();await w.get('[data-testid="zpaq-block-details-open"]').trigger('click');await w.get('[data-testid="zpaq-block-filter"]').setValue('all');expect(w.findAll('[role="listitem"]')).toHaveLength(20);await w.get('[data-testid="zpaq-block-next-page"]').trigger('click');expect(w.findAll('[role="listitem"]')[0].text()).toContain('文件-20.txt')
 await w.get('[data-testid="zpaq-block-query"]').setValue('文件-55.txt');expect(w.findAll('[role="listitem"]')).toHaveLength(1);expect(w.get('[role="listitem"]').text()).toContain('未检查');await w.get('[data-testid="zpaq-block-filter"]').setValue('verified');expect(w.findAll('[role="listitem"]')).toHaveLength(0);w.unmount()
})
it('zpaq163 exported scope keeps selected version, states and the partial-recovery boundary',()=>{
 const value=report();expect(zpaqCheckCounts(value)).toEqual({verified:53,affected:2,unchecked:1});const text=zpaqReportLines(value).join('\n');expect(text).toContain('ZPAQ 快照 3');expect(text).toContain('受影响不等于每个文件已证实损坏');expect(text).toContain('未检查：目录/文件-55.txt');expect(text).toContain('未执行部分提取')
})
