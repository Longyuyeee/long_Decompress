<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { ref } from 'vue'
import { useWorkflowAutomationStore } from '@/stores/workflowAutomation'
import { useWorkflowStore } from '@/stores/workflows'
import { workflowScheduleLabel,workflowQueueStatuses,type WorkflowScheduleRule } from '@/types/workflowTrigger'
const emit=defineEmits<{edit:[rule:WorkflowScheduleRule];credentials:[rule:WorkflowScheduleRule];remove:[id:string];open:[id:string]}>()
const automation=useWorkflowAutomationStore(),workflow=useWorkflowStore(),expanded=ref('')
const format=(value?:string|null)=>value?new Date(value).toLocaleString():'无后续时间'
const records=(id:string)=>automation.items.filter(item=>item.ruleId===id)
const recent=(id:string)=>records(id)[0]
</script>
<template>
 <div class="schedule-list custom-scrollbar" data-testid="workflow-schedule-list">
  <article v-for="rule in automation.schedules" :key="rule.id" class="rule" :data-workflow-schedule="rule.name">
   <button class="rule-toggle" :data-schedule-toggle="rule.id" :aria-expanded="expanded===rule.id" @click="expanded=expanded===rule.id?'':rule.id"><i class="pi pi-clock"/><span class="label"><strong>{{ rule.name }}</strong><small>{{ workflowScheduleLabel(rule) }}</small><small class="next">{{ rule.enabled?'下次：'+format(rule.nextAt):'定时已暂停' }}</small><small v-if="recent(rule.id)">最近：{{ workflowQueueStatuses[recent(rule.id)!.state] }}</small></span><span class="badge">{{ rule.enabled?'已启用':'已暂停' }}</span><i :class="expanded===rule.id?'pi pi-chevron-up':'pi pi-chevron-down'"/></button>
   <div v-if="expanded===rule.id" class="details" :data-schedule-details="rule.id"><dl><dt>使用方案</dt><dd>{{ workflow.plans.find(p=>p.id===rule.planId)?.name||'原方案已删除' }}</dd><dt>固定来源</dt><dd v-for="path in rule.paths" :key="path">{{ path }}</dd><dt>保存位置</dt><dd>{{ rule.outputDirectory }}</dd><dt>同方案忙碌时</dt><dd>{{ rule.busyPolicy==='queue'?'排队等候':'跳过本次并保留记录' }}；相同方案及来源不重复加入。</dd></dl><p v-if="rule.note" class="failure">{{ rule.note }}</p>
    <div class="buttons"><button class="button" :disabled="automation.saving" :data-schedule-run="rule.id" @click="automation.scheduleAction(rule.id,'run');automation.tick()">立即运行</button><button class="button" :disabled="automation.saving" :data-schedule-enabled="rule.id" @click="automation.scheduleAction(rule.id,rule.enabled?'pause':'enable')">{{ rule.enabled?'暂停定时':'启用定时' }}</button><button class="button" :data-schedule-edit="rule.id" @click="emit('edit',rule)">编辑</button><button class="button" :data-schedule-credentials="rule.id" @click="emit('credentials',rule)">本次密码</button><button class="link" @click="automation.forget(rule.id)">清除本次密码</button><button class="link" @click="emit('remove',rule.id)">删除规则</button></div>
    <span class="context-help"><HelpHint label="暂停定时规则说明">暂停只控制未来定时；已排队的项随之等待，主动选择的立即运行或补跑仍可执行。</HelpHint></span>
    <div v-if="records(rule.id).length" class="history"><h4>最近记录</h4><div v-for="item in records(rule.id).slice(0,5)" :key="item.id" class="history-row"><span>{{ format(item.scheduledAt||item.createdAt) }} · {{ workflowQueueStatuses[item.state] }}</span><button v-if="item.runId" class="link" @click="emit('open',item.runId)">查看运行与结果</button></div></div>
   </div>
  </article>
  <div v-if="!automation.schedules.length" class="empty"><i class="pi pi-clock"/><p>把常用处理安排到合适的时间</p><small>单次、每天或每周执行，错过后由你决定补跑或跳过。</small></div>
 </div>
</template>
<style scoped>
.schedule-list{max-height:680px;overflow:auto;min-width:0}.rule{border:1px solid var(--border-subtle);border-radius:13px;overflow:hidden;margin-bottom:10px}.rule-toggle{display:flex;align-items:center;gap:12px;padding:14px;width:100%;min-width:0}.rule-toggle>i:first-child,.next,.link{color:var(--dynamic-accent)}.label{flex:1;text-align:left;min-width:0}.label strong{font-size:12px;display:block;overflow-wrap:anywhere}.label small{display:block;font-size:10px;margin-top:6px;overflow-wrap:anywhere}.label small:not(.next){color:var(--text-muted)}.badge{font-size:10px;color:var(--text-muted);border:1px solid var(--border-subtle);border-radius:20px;padding:4px 8px;white-space:nowrap;flex-shrink:0}.details{padding:0 14px 14px}.details dl{font-size:11px;line-height:1.9}.details dt{color:var(--text-muted);margin-top:8px}.details dd{overflow-wrap:anywhere}.buttons{display:flex;flex-wrap:wrap;gap:8px;margin-top:16px;align-items:center}.button{font-size:11px;border:1px solid var(--border-subtle);background:var(--bg-input);border-radius:10px;padding:9px 12px}.button:disabled{opacity:.45}.link{font-size:11px}.muted,.failure{font-size:11px;color:var(--text-muted);line-height:1.9;margin-top:12px;overflow-wrap:anywhere}.failure{color:#ef4444}.history{margin-top:18px;border-top:1px solid var(--border-subtle);padding-top:12px}.history h4{font-size:11px;font-weight:700}.history-row{display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;font-size:10px;color:var(--text-muted);line-height:1.8;padding-top:10px}.empty{text-align:center;padding:45px 16px;font-size:12px}.empty i{font-size:26px;color:var(--dynamic-accent)}.empty p{margin-top:15px;font-weight:700}.empty small{display:block;font-size:11px;color:var(--text-muted);line-height:1.9;margin-top:8px}@media(max-width:1050px){.schedule-list{max-height:450px}}@media(max-width:800px){.rule-toggle{padding:12px;gap:8px}}
</style>
