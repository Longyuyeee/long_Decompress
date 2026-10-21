import { beforeEach,afterEach,it,expect,vi } from 'vitest'
import { reactive } from 'vue'
import { flushPromises,mount } from '@vue/test-utils'
import { createPinia,setActivePinia } from 'pinia'
import { useWorkflowAutomationStore } from '../workflowAutomation'
import { workflowTemplate,cleanWorkflowPlan } from '@/utils/workflow'
import WorkflowAutomationPanel from '@/components/workflows/WorkflowAutomationPanel.vue'
import type { WorkflowQueueItem } from '@/types/workflowTrigger'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),workflow:null as any}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@/stores/workflows',()=>({useWorkflowStore:()=>mocks.workflow}))
let state:any,entry:WorkflowQueueItem,store:ReturnType<typeof useWorkflowAutomationStore>
beforeEach(()=>{
 setActivePinia(createPinia());vi.clearAllMocks();const plan=cleanWorkflowPlan(workflowTemplate('empty'))
 entry={id:'item',ruleId:'rule',ruleName:'来件',sourceKey:'C:/in/source.txt',paths:['C:/in/source.txt'],revision:'v1',plan,outputDirectory:'D:/out',state:'queued',reason:'已稳定',createdAt:'2026-10-21'}
 state={rules:[{id:'rule',name:'来件',planId:plan.id,root:'C:/in',outputDirectory:'D:/out',enabled:true,note:'',mode:'files',extensions:'txt',settleSeconds:3,recursive:true,includeExisting:false}],items:[entry],paused:false}
 mocks.workflow=reactive({busy:false,runs:[] as any[],plans:[plan],initialize:vi.fn(),stop:vi.fn(),startQueued:vi.fn(async prepare=>{mocks.workflow.busy=true;try{const ready=await prepare();mocks.workflow.runs.push({id:ready.id,status:'completed'});return true}finally{mocks.workflow.busy=false}})})
 mocks.invoke.mockImplementation(async(command,args)=>{if(command==='workflow_trigger_claim'){entry.state='running';entry.runId=args.runId;return structuredClone(entry)}if(command==='workflow_trigger_finish'){entry.state='completed'}if(command==='workflow_trigger_pause')state.paused=args.paused;if(command==='workflow_trigger_enabled')state.rules[0].enabled=args.enabled;return structuredClone(state)})
 store=useWorkflowAutomationStore()
})
afterEach(()=>store.dispose())
it('1552 starts away from the view using the queue snapshot and actual claim identity',async()=>{
 store.begin();await flushPromises();expect(mocks.workflow.initialize).toHaveBeenCalled();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1)
 const claim=mocks.invoke.mock.calls.find(call=>call[0]==='workflow_trigger_claim')![1];expect(mocks.workflow.runs[0].id).toBe(claim.runId);expect(mocks.invoke).toHaveBeenCalledWith('workflow_trigger_finish')
})
it('1552 duplicate ticks do not dispatch a second workflow while the first claim is pending',async()=>{
 let release!:(value:any)=>void;const original=mocks.invoke.getMockImplementation()!;mocks.invoke.mockImplementation((command,args)=>command==='workflow_trigger_claim'?new Promise(resolve=>release=resolve):original(command,args))
 await store.tick();await flushPromises();await store.tick();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1);release(structuredClone(entry));await flushPromises()
})
it('1552 output passwords wait for a session and never enter persisted rule or queue calls',async()=>{
 entry.plan.steps[0].passwordRequired=true;entry.state='needs_input';await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled()
 store.remember('rule',{sourcePassword:'source-session-only',passwords:{[entry.plan.steps[0].id]:'output-session-only'}});await flushPromises();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1);expect(JSON.stringify(mocks.invoke.mock.calls)).not.toContain('session-only');expect(store.sessionFor('rule').sourcePassword).toBe('source-session-only')
 store.forget('rule');expect(store.sessionFor('rule').sourcePassword).toBe('')
})
it('1552 manual review and current work block automatic dispatch',async()=>{
 store.hold('review',true);await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();store.hold('review',false);mocks.workflow.busy=true;await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();mocks.workflow.busy=false;await store.tick();await flushPromises();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1)
})
it('1552 queue and rule pauses block dispatch while observation notices keep ready work moving',async()=>{
 state.paused=true;await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();state.paused=false;state.rules[0].enabled=false;await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();state.rules[0].enabled=true;state.rules[0].note='已有 200 份等待来源';await store.tick();await flushPromises();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1);expect(store.items).toHaveLength(1)
})
it('1552 preparing exit during observation prevents a new claim and drains persistence',async()=>{
 await store.initialize();let resolve!:(value:any)=>void;const original=mocks.invoke.getMockImplementation()!;mocks.invoke.mockImplementation((command,args)=>command==='workflow_trigger_poll'?new Promise(r=>resolve=r):original(command,args));const tick=store.tick();await flushPromises();store.prepareExit();resolve(structuredClone(state));await tick;expect(mocks.workflow.startQueued).not.toHaveBeenCalled();expect(await store.flush()).toBe(true);store.resumeAfterExitFailure();expect(store.closing).toBe(false)
})
it('1552 a rejected native claim does not invent a run or completion',async()=>{
 const original=mocks.invoke.getMockImplementation()!;mocks.invoke.mockImplementation((command,args)=>command==='workflow_trigger_claim'?Promise.reject(Error('来源已变化')):original(command,args));await store.tick();await flushPromises();expect(mocks.workflow.runs).toHaveLength(0);expect(mocks.invoke.mock.calls.some(call=>call[0]==='workflow_trigger_finish')).toBe(false);expect(store.error).toContain('来源已变化')
})
it('1552 a claim interrupted before dispatch becomes explicit unfinished work',async()=>{
 mocks.workflow.startQueued.mockImplementation(async prepare=>{await prepare();return false});await store.tick();await flushPromises();expect(mocks.invoke).toHaveBeenCalledWith('workflow_trigger_action',{id:'item',action:'dispatch_failed'});expect(mocks.workflow.runs).toHaveLength(0)
})
it('1552 pending cancellation uses its receipt without starting work and deletion clears the session',async()=>{
 await store.initialize();store.hold('review',true);store.remember('rule',{sourcePassword:'private',passwords:{}});await store.action('item','cancel');expect(mocks.invoke).toHaveBeenCalledWith('workflow_trigger_action',{id:'item',action:'cancel'});expect(mocks.workflow.startQueued).not.toHaveBeenCalled();await store.removeRule('rule');expect(store.sessionFor('rule').sourcePassword).toBe('')
})
it('1552 directory rules expand inline while editing and credentials hold automatic dispatch',async()=>{
 await store.initialize();const view=mount(WorkflowAutomationPanel,{global:{stubs:{WorkflowDirectoryRuleDialog:{template:'<div data-rule-editor />'},WorkflowRunDialog:{template:'<div data-credentials />'},Modal:true}}});await flushPromises();await view.get('[data-testid="workflow-directories-tab"]').trigger('click');expect(view.find('[data-rule-details]').exists()).toBe(false);await view.get('[data-rule-toggle="rule"]').trigger('click');expect(view.find('[data-rule-details="rule"]').exists()).toBe(true);await view.get('[data-rule-edit="rule"]').trigger('click');expect(view.find('[data-rule-editor]').exists()).toBe(true);await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();view.unmount();expect(store.closing).toBe(false)
})
