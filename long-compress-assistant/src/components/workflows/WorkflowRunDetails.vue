<script setup lang="ts">
import { ref } from 'vue'
import { useTaskStore } from '@/stores/task'
import ResultActions from '@/components/tasks/ResultActions.vue'
import TaskExecutionOverview from '@/components/tasks/TaskExecutionOverview.vue'
import ArchiveOutputSummary from '@/components/compression/ArchiveOutputSummary.vue'
import { workflowKinds, workflowStatuses, type WorkflowRun } from '@/types/workflow'
const props = defineProps<{ run: WorkflowRun }>()
const tasks = useTaskStore(), expanded = ref('')
const taskFor = (ids: string[]) => tasks.tasks.find(task => task.id === ids.at(-1))
const stamp = (value: string) => new Date(value).toLocaleString()
</script>
<template>
  <section class="space-y-4 min-w-0" data-testid="workflow-run-details">
    <div class="run-heading"><div class="min-w-0"><h3>{{ props.run.name }}</h3><p class="muted">{{ stamp(run.startedAt) }} · {{ workflowStatuses[run.status] }}</p></div><span class="badge" :class="run.status">{{ workflowStatuses[run.status] }}</span></div>
    <ResultActions :path="run.directory" directory show-path />
    <p v-if="run.status==='interrupted'" class="muted callout">上次运行已中断。已完成的产物保留，本版可查看记录；需要处理时请手动重新运行。</p>
    <article v-for="(step,index) in run.steps" :key="step.id" class="run-step">
      <button class="step-heading" :data-workflow-run-step="index" :aria-expanded="expanded===step.id || ['running','failed'].includes(step.status)" @click="expanded=expanded===step.id?'':step.id"><span class="number">{{ index+1 }}</span><span class="min-w-0 flex-1 text-left"><strong>{{ step.name }}</strong><small>{{ workflowKinds[step.kind].label }} · {{ step.inputs.length }} 处来源 · {{ step.outputs.length }} 项产物</small></span><span class="badge" :class="step.status">{{ workflowStatuses[step.status] }}</span><i class="pi pi-chevron-down" /></button>
      <div v-if="expanded===step.id || ['running','failed'].includes(step.status)" class="step-body space-y-4">
        <p v-if="step.message" class="muted" :class="{'failure':step.status==='failed'}" role="status">{{ step.message }}</p>
        <TaskExecutionOverview v-if="taskFor(step.taskIds)" :stage="taskFor(step.taskIds)!.stage || workflowStatuses[step.status]" :status="taskFor(step.taskIds)!.status" :progress="taskFor(step.taskIds)!.progress" :speed="taskFor(step.taskIds)!.speed" :current-file="taskFor(step.taskIds)!.currentFile" />
        <details v-if="step.inputs.length"><summary class="muted cursor-pointer">本步来源 · {{ step.inputs.length }} 项</summary><ul class="path-list custom-scrollbar"><li v-for="path in step.inputs.slice(0,50)" :key="path">{{ path }}</li><li v-if="step.inputs.length>50">仅展示前 50 项</li></ul></details>
        <div v-if="step.outputs.length" class="space-y-3"><h4>本步实际产物</h4><div v-for="output in step.outputs.slice(0,50)" :key="output.path" class="output-card"><ArchiveOutputSummary v-if="output.files.length" :output="{primaryPath:output.path,files:output.files,totalBytes:output.totalBytes,split:output.split}" /><ResultActions v-else :path="output.path" :directory="output.directory" show-path /></div><p v-if="step.outputs.length>50" class="muted">仅展示前 50 项，完整文件可在本次运行目录查看。</p></div>
        <RouterLink v-if="step.taskIds.length" class="text-xs text-primary inline-block" to="/history">在任务历史查看本步处理记录 <i class="pi pi-arrow-up-right"/></RouterLink>
        <details v-if="taskFor(step.taskIds)?.logs?.length" :open="step.status==='running'"><summary class="muted cursor-pointer">执行日志</summary><pre class="log custom-scrollbar">{{ taskFor(step.taskIds)!.logs.map(log=>`[${log.timestamp}] ${log.message}`).join('\n') }}</pre></details>
      </div>
    </article>
  </section>
</template>
<style scoped>
h3{font-size:17px;font-weight:900;overflow-wrap:anywhere}h4{font-size:12px;font-weight:800}.muted{font-size:11px;line-height:1.9;color:var(--text-muted);overflow-wrap:anywhere}.run-heading{display:flex;align-items:flex-start;justify-content:space-between;gap:12px}.run-step{min-width:0;border:1px solid var(--border-subtle);border-radius:14px;background:var(--bg-input)}.step-heading{display:flex;align-items:center;gap:12px;width:100%;padding:16px}.step-heading strong{display:block;font-size:13px;overflow-wrap:anywhere}.step-heading small{display:block;font-size:10px;color:var(--text-muted);margin-top:5px}.number{display:grid;place-items:center;flex-shrink:0;width:30px;height:30px;border-radius:10px;color:var(--dynamic-accent);background:var(--bg-card);font-weight:900;font-size:12px}.badge{flex-shrink:0;border:1px solid var(--border-subtle);border-radius:999px;padding:4px 8px;font-size:10px;color:var(--text-muted)}.badge.completed{color:#22c55e}.badge.running{color:var(--dynamic-accent)}.badge.failed,.failure{color:#ef4444}.step-body{padding:0 16px 16px;min-width:0}.output-card{min-width:0;border:1px solid var(--border-subtle);border-radius:12px;padding:12px;background:var(--bg-card)}.output-card :deep([data-testid="archive-output-summary"]){border:0;margin:0;padding:0;background:none}.path-list{max-height:150px;overflow:auto;font-size:11px;line-height:1.9;overflow-wrap:anywhere;padding:10px}.log{max-height:240px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font-size:11px;line-height:1.8;margin-top:10px;padding:12px;background:var(--bg-card);border-radius:10px}.callout{border:1px solid var(--border-subtle);border-radius:12px;padding:12px}@media(max-width:800px){.step-heading{gap:8px;padding:12px;flex-wrap:wrap}.step-body{padding:0 12px 12px}}
</style>
