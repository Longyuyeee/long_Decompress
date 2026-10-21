import { beforeEach,afterEach,it,expect,vi } from 'vitest'
import { reactive } from 'vue'
import { flushPromises,mount } from '@vue/test-utils'
import { createPinia,setActivePinia } from 'pinia'
import { useWorkflowAutomationStore } from '../workflowAutomation'
import { workflowTemplate,cleanWorkflowPlan } from '@/utils/workflow'
import WorkflowAutomationPanel from '@/components/workflows/WorkflowAutomationPanel.vue'
import WorkflowScheduleDialog from '@/components/workflows/WorkflowScheduleDialog.vue'
import type { WorkflowQueueItem } from '@/types/workflowTrigger'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),workflow:null as any}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@/stores/workflows',()=>({useWorkflowStore:()=>mocks.workflow}))
let state:any,entry:WorkflowQueueItem,store:ReturnType<typeof useWorkflowAutomationStore>
beforeEach(()=>{
 setActivePinia(createPinia());vi.clearAllMocks();const plan=cleanWorkflowPlan(workflowTemplate('empty'))
 entry={trigger:'schedule',explicit:false,id:'item',ruleId:'rule',ruleName:'来件',sourceKey:'C:/in/source.txt',paths:['C:/in/source.txt'],revision:'v1',plan,outputDirectory:'D:/out',state:'queued',reason:'已稳定',createdAt:'2026-10-21'}
 state={rules:[],schedules:[{id:'rule',name:'每日交付',planId:plan.id,paths:entry.paths,outputDirectory:entry.outputDirectory,kind:'daily',time:'18:00',date:'2026-10-21',weekdays:[1,3,5],busyPolicy:'queue',enabled:true,checkedAt:0,note:'',nextAt:'2026-10-22T10:00:00Z'}],items:[entry],paused:false}
 mocks.workflow=reactive({busy:false,runs:[] as any[],plans:[plan],initialize:vi.fn(),stop:vi.fn(),startQueued:vi.fn(async prepare=>{mocks.workflow.busy=true;try{const ready=await prepare();mocks.workflow.runs.push({id:ready.id,status:'completed'});return true}finally{mocks.workflow.busy=false}})})
 mocks.invoke.mockImplementation(async(command,args)=>{if(command==='workflow_trigger_claim'){entry.state='running';entry.runId=args.runId;return structuredClone(entry)}if(command==='workflow_trigger_finish'){entry.state='completed'}if(command==='workflow_trigger_pause')state.paused=args.paused;if(command==='workflow_trigger_enabled')state.rules[0].enabled=args.enabled;return structuredClone(state)})
 store=useWorkflowAutomationStore()
})
afterEach(()=>store.dispose())
it('1553 schedules alone initialize and dispatch in the shared executor away from the view',async()=>{
 store.begin();await flushPromises();expect(mocks.workflow.initialize).toHaveBeenCalled();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1);expect(mocks.invoke).toHaveBeenCalledWith('workflow_trigger_poll',{busy:false});expect(store.schedules[0].name).toBe('每日交付')
})
it('1553 missed occurrences never start until explicitly prepared and enabled rules still obey the global pause',async()=>{
 entry.state='missed';await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();entry.state='queued';state.paused=true;await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();state.paused=false;await store.tick();await flushPromises();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1)
})
it('1553 a paused schedule waits but an explicit run still uses the same claim',async()=>{
 state.schedules[0].enabled=false;await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();entry.explicit=true;await store.tick();await flushPromises();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1)
})
it('1553 schedule credentials stay in session and exit drains schedules without directory rules',async()=>{
 entry.plan.steps[0].passwordRequired=true;entry.state='needs_input';await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();store.remember('rule',{sourcePassword:'private-source',passwords:{[entry.plan.steps[0].id]:'private-output'}});await flushPromises();expect(mocks.workflow.startQueued).toHaveBeenCalledTimes(1);store.prepareExit();expect(await store.flush()).toBe(true);expect(JSON.stringify(mocks.invoke.mock.calls)).not.toContain('private-');store.dispose();expect(store.sessionFor('rule').passwords).toEqual({})
})
it('1553 editing schedules holds dispatch and leaves the original directory editor available',async()=>{
 await store.initialize();const view=mount(WorkflowAutomationPanel,{global:{stubs:{WorkflowScheduleDialog:{template:'<div data-schedule-editor />'},WorkflowDirectoryRuleDialog:{template:'<div data-directory-editor />'},WorkflowRunDialog:true,Modal:true}}});await flushPromises();await view.get('[data-schedule-toggle="rule"]').trigger('click');expect(view.text()).toContain('最近记录');await view.get('[data-schedule-edit="rule"]').trigger('click');await store.tick();expect(mocks.workflow.startQueued).not.toHaveBeenCalled();expect(view.find('[data-schedule-editor]').exists()).toBe(true);view.unmount();expect(store.holds).toBeUndefined()
})
it('1553 missed run and skip controls use the existing queue receipt',async()=>{
 entry.state='missed';await store.initialize();const view=mount(WorkflowAutomationPanel,{global:{stubs:{WorkflowRunDialog:true,Modal:true}}});await flushPromises();await view.get('[data-queue-run-missed="item"]').trigger('click');expect(mocks.invoke).toHaveBeenCalledWith('workflow_trigger_action',{id:'item',action:'run_missed'});await view.get('[data-queue-skip="item"]').trigger('click');expect(mocks.invoke).toHaveBeenCalledWith('workflow_trigger_action',{id:'item',action:'skip'});expect(mocks.workflow.startQueued).not.toHaveBeenCalled();view.unmount()
})
it('1553 deleting a schedule clears its current session and uses the dedicated rule command',async()=>{
 await store.initialize();store.hold('review',true);store.remember('rule',{sourcePassword:'private-delete',passwords:{}});await store.scheduleAction('rule','delete');expect(mocks.invoke).toHaveBeenCalledWith('workflow_schedule_action',{id:'rule',action:'delete'});expect(store.sessionFor('rule').sourcePassword).toBe('')
})
it('1553 schedule editor has plain time and weekday controls and cannot save an empty weekly choice',async()=>{
 const view=mount(WorkflowScheduleDialog,{props:{rule:{...state.schedules[0],kind:'weekly',weekdays:[]},plans:mocks.workflow.plans,busy:false,error:''},global:{stubs:{Modal:{template:'<div><slot/><slot name="footer"/></div>'}}}});expect(view.get('[data-testid="workflow-schedule-save"]').attributes('disabled')).toBeDefined();await view.find('.weekdays input').setValue(true);expect(view.get('[data-testid="workflow-schedule-save"]').attributes('disabled')).toBeUndefined();await view.get('[data-testid="workflow-schedule-save"]').trigger('click');expect((view.emitted('save')![0][0] as any).weekdays).toEqual([1]);view.unmount()
})
