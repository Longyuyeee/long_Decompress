<script setup lang="ts">
import { computed } from 'vue'
import type { Task } from '@/stores/task'
import { archiveInspectionMessageForDisplay, describeTaskFailure, getTaskRecoveryGuidance } from '@/utils/taskFailure'
import { isPasswordRelatedError } from '@/utils'
const props=defineProps<{task?:Task}>()
const record=computed(()=>({status:props.task?.status==='failed'?'failed' as const:'cancelled' as const,errorMessage:props.task?.error,
  failure:{schemaVersion:1 as const,category:'unknown',stage:props.task?.stage || 'unknown',evidence:props.task?.stage?'observed-stage' as const:'unknown' as const}}))
const failure=computed(()=>describeTaskFailure(record.value))
const guidance=computed(()=>getTaskRecoveryGuidance(record.value))
const password=computed(()=>failure.value?.category==='unknown' && (props.task?.passwordRequired || isPasswordRelatedError(props.task?.error || '')))
const steps=computed(()=>guidance.value?.steps.filter(step=>!step.startsWith('可通过下方')).slice(0,2) || [])
</script>
<template>
  <section v-if="failure" data-testid="task-failure-hint" class="failure-hint">
    <div class="failure-title"><i class="pi pi-exclamation-circle" aria-hidden="true"></i><strong>{{ password ? '需要确认密码后继续' : guidance?.title }}</strong><span v-if="failure.stage!=='unknown'" class="text-muted">{{ failure.stageLabel }}</span></div>
    <p v-if="password">重新确认密码；转换时请分别填写原包密码和新包密码，或明确选择新包不加密。</p>
    <p v-else v-for="step in steps" :key="step" class="text-muted">{{ step }}</p>
    <details><summary>查看原始错误</summary><p class="failure-error">{{ archiveInspectionMessageForDisplay(failure.message) }}</p></details>
    <slot />
  </section>
</template>
<style scoped>
.failure-hint{min-width:0;border:1px solid color-mix(in srgb,var(--dynamic-accent) 25%,var(--border-subtle));border-radius:.8rem;padding:.85rem;background:var(--bg-input);font-size:.75rem;line-height:1.65;overflow-wrap:anywhere;margin-bottom:1rem;display:flex;flex-direction:column;gap:.5rem;}
.failure-title{display:flex;flex-wrap:wrap;align-items:baseline;gap:.35rem .55rem;color:var(--text-base);}
.failure-title>span{font-size:.7rem;margin-left:auto;}
summary{cursor:pointer;width:fit-content;color:var(--dynamic-accent);}
.failure-error{white-space:pre-wrap;user-select:text;margin-top:.4rem;max-height:8rem;overflow:auto;color:var(--text-secondary);}
</style>
