import { defineStore } from 'pinia'
import { computed,ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useWorkflowStore } from './workflows'
import { normalizeWorkflowPlan,validateWorkflow,workflowClone } from '@/utils/workflow'
import type { WorkflowRunRequest } from '@/types/workflow'
import { workflowQueueTerminal,type WorkflowDirectoryRule,type WorkflowQueueItem,type WorkflowTriggerState,type WorkflowScheduleRule,type WorkflowSessionCredentials } from '@/types/workflowTrigger'
export const useWorkflowAutomationStore=defineStore('workflow-automation',()=>{
 const workflow=useWorkflowStore(),rules=ref<WorkflowDirectoryRule[]>([]),schedules=ref<WorkflowScheduleRule[]>([]),items=ref<WorkflowQueueItem[]>([]),paused=ref(false),error=ref(''),saving=ref(false),polling=ref(false),closing=ref(false),holds=ref<string[]>([])
 const credentials=new Map<string,WorkflowSessionCredentials>()
 let initialized=false,initializing:Promise<void>|undefined,pending:Promise<void>|undefined,execution:Promise<void>|undefined,timer:ReturnType<typeof setInterval>|undefined
 const waiting=computed(()=>items.value.filter(item=>!workflowQueueTerminal(item.state)&&item.state!=='running'))
 function accept(state:WorkflowTriggerState){rules.value=state?.rules||[];schedules.value=state?.schedules||[];items.value=state?.items||[];paused.value=state?.paused||false}
 function hold(key:string,on:boolean){holds.value=[...holds.value.filter(value=>value!==key),...(on?[key]:[])]}
 async function initialize(){if(initialized)return;if(initializing)return initializing;initializing=(async()=>{try{accept(await invoke<WorkflowTriggerState>('workflow_trigger_state'));if(rules.value.length||schedules.value.length)await workflow.initialize();initialized=true;error.value=''}catch(cause){error.value=String(cause)}finally{initializing=undefined}})();return initializing}
 function missingCredentials(item:WorkflowQueueItem){const session=credentials.get(item.ruleId);return item.plan.steps.some(step=>step.passwordRequired&&!session?.passwords[step.id])}
 function request(item:WorkflowQueueItem):WorkflowRunRequest{const session=credentials.get(item.ruleId);return {paths:[...item.paths],outputDirectory:item.outputDirectory,sourcePassword:session?.sourcePassword||'',sourceEncoding:session?.sourceEncoding,passwords:{...session?.passwords}}}
 function sessionFor(ruleId:string){return workflowClone(credentials.get(ruleId)||{sourcePassword:'',passwords:{}})}
 function remember(ruleId:string,value:WorkflowSessionCredentials){credentials.set(ruleId,workflowClone(value));error.value='';void tick()}
 function forget(ruleId:string){credentials.delete(ruleId)}
 async function change(command:string,args:Record<string,unknown>){saving.value=true;error.value='';try{accept(await invoke<WorkflowTriggerState>(command,args));initialized=true;return true}catch(cause){error.value=String(cause);return false}finally{saving.value=false}}
 const saveRule=(rule:WorkflowDirectoryRule)=>change('workflow_trigger_save',{rule})
 const saveSchedule=(rule:WorkflowScheduleRule)=>change('workflow_schedule_save',{rule})
 async function scheduleAction(id:string,action:string){const ok=await change('workflow_schedule_action',{id,action});if(ok&&action==='delete')credentials.delete(id);return ok}
 const enable=(id:string,enabled:boolean)=>change('workflow_trigger_enabled',{id,enabled})
 async function removeRule(id:string){if(await change('workflow_trigger_delete',{id}))credentials.delete(id)}
 const pause=(value:boolean)=>change('workflow_trigger_pause',{paused:value})
 const action=(id:string,action:string)=>change('workflow_trigger_action',{id,action})
 async function dispatch(item:WorkflowQueueItem){
  const id=crypto.randomUUID();let claimed=false
  try{
   await workflow.startQueued(async()=>{
    if(closing.value||paused.value||holds.value.length)throw Error('暂未派发，队列或确认界面已暂停启动')
    const entry=await invoke<WorkflowQueueItem>('workflow_trigger_claim',{id:item.id,runId:id});claimed=true
    const plan=normalizeWorkflowPlan(entry.plan),runRequest=request(entry)
    validateWorkflow(plan,runRequest.paths,runRequest.outputDirectory,runRequest.passwords)
    return {id,plan,request:runRequest}
   })
   if(claimed){if(workflow.runs.some(run=>run.id===id))accept(await invoke<WorkflowTriggerState>('workflow_trigger_finish'));else accept(await invoke<WorkflowTriggerState>('workflow_trigger_action',{id:item.id,action:'dispatch_failed'}))}
  }catch(cause){
   error.value=String(cause)
   if(claimed)try{accept(await invoke<WorkflowTriggerState>('workflow_trigger_action',{id:item.id,action:'dispatch_failed'}))}catch(errorCause){error.value+='；'+String(errorCause)}
  }
 }
 async function tick(){
  if(pending||closing.value)return pending
  pending=(async()=>{polling.value=true;try{
   await initialize();if(!initialized||closing.value||!rules.value.length&&!schedules.value.length)return
   accept(await invoke<WorkflowTriggerState>('workflow_trigger_poll',{busy:workflow.busy}))
   if(closing.value||execution||workflow.busy||paused.value||saving.value||holds.value.length)return
   const item=[...items.value].reverse().find(item=>['queued','needs_input'].includes(item.state)&&!missingCredentials(item)&&(item.trigger==='schedule'?schedules.value.some(rule=>rule.id===item.ruleId&&(rule.enabled||item.explicit)):rules.value.some(rule=>rule.id===item.ruleId&&rule.enabled)))
   if(item){await workflow.initialize();if(closing.value||holds.value.length||paused.value)return;execution=dispatch(item).finally(()=>{execution=undefined})}
  }catch(cause){error.value=String(cause)}finally{polling.value=false;pending=undefined}})();return pending
 }
 function begin(){if(timer)return;closing.value=false;void tick();timer=setInterval(()=>void tick(),2000)}
 function prepareExit(){closing.value=true}
 function resumeAfterExitFailure(){closing.value=false}
 async function flush(){try{await pending;await execution;if(initialized&&(rules.value.length||schedules.value.length))accept(await invoke<WorkflowTriggerState>('workflow_trigger_finish'));return true}catch(cause){error.value='自动化队列暂未保存：'+String(cause);return false}}
 function dispose(){closing.value=true;if(timer)clearInterval(timer);timer=undefined;credentials.clear();holds.value=[]}
 return {rules,schedules,saveSchedule,scheduleAction,items,waiting,paused,error,saving,polling,closing,initialize,tick,begin,dispose,hold,saveRule,enable,removeRule,pause,action,missingCredentials,remember,forget,sessionFor,prepareExit,resumeAfterExitFailure,flush}
})
