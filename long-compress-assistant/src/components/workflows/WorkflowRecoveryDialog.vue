<script setup lang="ts">
import { computed,ref } from 'vue'
import WorkflowRunDialog from './WorkflowRunDialog.vue'
import CompressionSettingsPanel from '@/components/compression/CompressionSettingsPanel.vue'
import { normalizeWorkflowPlan,workflowDefaultResume,workflowResumeFrom,workflowParentDirectory,workflowInputCredential } from '@/utils/workflow'
import { workflowKinds,type WorkflowRun,type WorkflowPlan,type WorkflowRunRequest,type WorkflowProofProgress,type WorkflowReuseIssue } from '@/types/workflow'
import type { CompressionOptions } from '@/stores/compression'
const props=defineProps<{run:WorkflowRun;from?:number;busy:boolean;phase:string;progress?:WorkflowProofProgress;error?:string;issue?:WorkflowReuseIssue}>()
const emit=defineEmits<{close:[];run:[plan:WorkflowPlan,request:WorkflowRunRequest,from:number];stop:[]}>()
const plan=ref(normalizeWorkflowPlan(props.run.plan!)),requested=ref(props.from??workflowDefaultResume(props.run)),paths=ref([...props.run.inputs]),expanded=ref('')
const from=computed(()=>workflowResumeFrom(props.run,plan.value,paths.value,requested.value)),credential=computed(()=>workflowInputCredential(props.run,plan.value,from.value))
const initial={paths:props.run.inputs,outputDirectory:props.run.outputDirectory||workflowParentDirectory(props.run.directory),sourceEncoding:credential.value.encoding}
function updateSettings(index:number,value:CompressionOptions){const step=plan.value.steps[index];step.settings={...value,deleteAfter:false,fastPack:false};if(value.password)step.passwordRequired=true}
</script>
<template>
  <WorkflowRunDialog :plan="plan" :initial="initial" :start-index="from" :input-credential="credential" recovery :busy="busy" :phase="phase" :progress="progress" :error="error" :issue="issue" @close="emit('close')" @sources="paths=$event" @run="emit('run',plan,$event,from)" @stop="emit('stop')" @earlier="requested=$event">
    <template #before>
      <section class="recovery-summary" data-testid="workflow-recovery-dialog">
        <p class="text-xs text-muted break-all">接续 {{ run.name }} · {{ new Date(run.startedAt).toLocaleString() }}</p>
        <label class="block text-xs font-bold mt-3">从哪一步重新处理<select v-model.number="requested" class="from-select mt-2" data-testid="workflow-resume-from" :disabled="busy"><option v-for="(step,index) in plan.steps" :key="step.id" :value="index">第 {{ index+1 }} 步 · {{ step.name }}</option></select></label>
        <p class="text-xs leading-6 mt-3" data-testid="workflow-reuse-scope"><strong class="text-primary">前 {{ from }} 步核对后复用</strong>，从第 {{ from+1 }} 步起重新处理。原运行和已有结果保留。</p>
        <p v-if="from<requested" class="text-xs leading-6 text-primary">来源或上游参数已调整，重跑范围已提前到第 {{ from+1 }} 步。</p>
        <p class="text-xs leading-6 text-muted">修改只用于这次接续。开始时按内容核对复用部分；若有变化，会提示从哪里重新处理。密码不会自动恢复。</p>
      </section>
      <section class="recovery-steps space-y-2 mb-5" data-testid="workflow-recovery-settings">
        <article v-for="(step,index) in plan.steps" :key="step.id" class="recovery-step">
          <button class="recovery-toggle" :data-workflow-recovery-step="index" :aria-expanded="expanded===step.id" @click="expanded=expanded===step.id?'':step.id"><span class="min-w-0 flex-1 text-left break-all"><strong>{{ index+1 }} · {{ step.name }}</strong><small>{{ workflowKinds[step.kind].label }} · {{ index<from?'待核对复用':'本次重做' }}</small></span><i :class="expanded===step.id?'pi pi-chevron-up':'pi pi-chevron-down'"/></button>
          <fieldset v-if="expanded===step.id" :disabled="busy" class="recovery-step-body space-y-3">
            <label class="block text-xs">步骤名称<input v-model="step.name" class="from-select mt-2" maxlength="60"></label>
            <label v-if="step.kind==='filter'" class="block text-xs">保留扩展名<input v-model="step.extensions" class="from-select mt-2" data-testid="workflow-recovery-extensions"></label>
            <p v-else-if="step.kind==='extract'" class="text-xs leading-6 text-muted">读取密码和编码在下方填写；已完成的解压产物仍可保留。</p>
            <template v-else><label class="flex items-center gap-2 text-xs"><input v-model="step.passwordRequired" type="checkbox" :data-recovery-require-password="index" @change="!step.passwordRequired&&(step.settings.password='')">本步生成密码包，运行时填写输出密码</label><CompressionSettingsPanel :model-value="step.settings" :show-output-path="false" :allow-delete-after="false" :allow-single-file-formats="false" :allow-regular-file-formats="false" compact @update:model-value="updateSettings(index,$event)" /></template>
          </fieldset>
        </article>
      </section>
    </template>
  </WorkflowRunDialog>
</template>
<style scoped>
.recovery-summary{border:1px solid color-mix(in srgb,var(--dynamic-accent) 30%,var(--border-subtle));border-radius:14px;padding:16px;margin-bottom:16px;background:color-mix(in srgb,var(--dynamic-accent) 5%,var(--bg-card))}.from-select{display:block;width:100%;min-width:0;border:1px solid var(--border-subtle);border-radius:10px;background:var(--bg-input);padding:10px;font-size:12px;color:var(--text-content)}.recovery-step{border:1px solid var(--border-subtle);border-radius:12px;background:var(--bg-card);min-width:0}.recovery-toggle{display:flex;align-items:center;gap:12px;width:100%;padding:13px 15px;font-size:12px}.recovery-toggle small{display:block;font-size:10px;color:var(--text-muted);margin-top:4px}.recovery-step-body{padding:0 15px 15px;min-width:0}@media(max-width:800px){.recovery-summary{padding:12px}.recovery-step-body{padding:0 12px 12px}}
</style>
