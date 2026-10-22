<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import type { WorkflowPlan } from '@/types/workflow'
import type { WorkflowScheduleRule } from '@/types/workflowTrigger'
const props=defineProps<{rule?:WorkflowScheduleRule;plans:WorkflowPlan[];busy:boolean;error:string}>()
const emit=defineEmits<{close:[];save:[rule:WorkflowScheduleRule]}>()
const today=new Date(),date=`${today.getFullYear()}-${String(today.getMonth()+1).padStart(2,'0')}-${String(today.getDate()).padStart(2,'0')}`
const draft=ref<WorkflowScheduleRule>(props.rule?JSON.parse(JSON.stringify(props.rule)):{id:crypto.randomUUID(),name:'定时归档',planId:props.plans[0]?.id||'',paths:[],outputDirectory:'',kind:'daily',time:'18:00',date,weekdays:[1,2,3,4,5],busyPolicy:'queue',enabled:true,checkedAt:0,note:''})
const picking=ref(false)
async function choose(target:'files'|'folder'|'output'){
 picking.value=true
 try{const supplied=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined
 const value=supplied===undefined?await open({directory:target!=='files',multiple:target==='files',title:target==='output'?'选择保存位置':'选择固定来源'}):supplied
 if(target==='output'){if(typeof value==='string')draft.value.outputDirectory=value}else if(value){draft.value.paths=[...new Set([...draft.value.paths,...(Array.isArray(value)?value:[value])])]}
 }finally{picking.value=false}
}
</script>
<template>
 <Modal description="按电脑本地时间执行，应用需保持运行或托盘驻留。关闭应用后不会唤醒电脑；错过的时段由你选择补跑或跳过。" description-as-help :visible="true" :title="rule?'编辑定时规则':'添加定时规则'" icon="pi pi-clock" size="lg" :show-footer="true" :show-close-button="!busy" :close-on-backdrop="!busy" :close-on-escape="!busy" @update:visible="emit('close')">
  <fieldset :disabled="busy||picking" class="schedule-form" data-testid="workflow-schedule-dialog">

   <div class="form-grid"><label>规则名称<input v-model="draft.name" maxlength="60" data-testid="workflow-schedule-name"></label><label>使用方案<select v-model="draft.planId" data-testid="workflow-schedule-plan"><option v-for="plan in plans" :key="plan.id" :value="plan.id">{{ plan.name }}</option></select></label></div>
   <div><div class="source-heading"><strong>固定来源 · {{ draft.paths.length }} 份</strong><div class="buttons"><button class="button" data-testid="workflow-schedule-files" @click="choose('files')">添加文件 / 分卷</button><button class="button" data-testid="workflow-schedule-folder" @click="choose('folder')">添加文件夹</button></div></div><div class="sources custom-scrollbar"><div v-for="(path,index) in draft.paths" :key="path" class="source"><span>{{ path }}</span><button class="link" :aria-label="'移除来源 '+path" @click="draft.paths.splice(index,1)">移除</button></div><p v-if="!draft.paths.length" class="muted">选择日常需要处理的固定文件或文件夹，最多 32 份。</p></div><small>触发时使用当前资料；整组分卷会自动识别。已排队后资料发生变化，会提示重新加入。</small></div>
   <label>保存位置<div class="path-field"><input v-model="draft.outputDirectory" data-testid="workflow-schedule-output" placeholder="每次使用独立的运行目录"><button class="button" @click="choose('output')">选择</button></div><small>请放在来源文件夹以外，避免下一次把结果再次打包。</small></label>
   <div class="form-grid"><label>重复方式<select v-model="draft.kind" data-testid="workflow-schedule-kind"><option value="once">仅一次</option><option value="daily">每天</option><option value="weekly">每周</option></select></label><label>本地时间<input v-model="draft.time" type="time" data-testid="workflow-schedule-time"></label></div>
   <label v-if="draft.kind==='once'">执行日期<input v-model="draft.date" type="date" data-testid="workflow-schedule-date"></label>
   <div v-if="draft.kind==='weekly'" class="weekdays"><label v-for="(name,index) in ['一','二','三','四','五','六','日']" :key="name" class="day"><input v-model="draft.weekdays" type="checkbox" :value="index+1">周{{ name }}</label></div>
   <label>同一方案到点时仍在执行<select v-model="draft.busyPolicy" data-testid="workflow-schedule-busy"><option value="queue">排队等候（不重复加入相同来源）</option><option value="skip">跳过本次并保留记录</option></select><small>全局队列暂停时不启动新任务。立即运行仍进入同一个队列。</small></label>
   <label class="check"><input v-model="draft.enabled" type="checkbox" data-testid="workflow-schedule-enabled">保存后启用定时</label>
  </fieldset>
  <p v-if="error" class="failure" role="alert">{{ error }}</p>
  <template #footer><button class="button" :disabled="busy" @click="emit('close')">返回</button><button class="button primary" data-testid="workflow-schedule-save" :disabled="busy||picking||!draft.name.trim()||!draft.planId||!draft.paths.length||!draft.outputDirectory.trim()||!draft.time||draft.kind==='weekly'&&!draft.weekdays.length" @click="emit('save',draft)">{{ busy?'保存中…':draft.enabled?'保存并启用':'保存为暂停' }}</button></template>
 </Modal>
</template>
<style scoped>
.schedule-form{color-scheme:light;display:flex;flex-direction:column;gap:18px;min-width:0}.schedule-form label,.source-heading strong{font-size:12px;font-weight:700;min-width:0}.schedule-form input:not([type=checkbox]),select{display:block;width:100%;min-width:0;margin-top:8px;border:1px solid var(--border-subtle);border-radius:10px;background:var(--bg-input);padding:10px 12px;font-size:12px;color:var(--text-content)}.form-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px}.path-field,.source-heading,.buttons{display:flex;gap:8px;align-items:center;min-width:0}.source-heading{justify-content:space-between;flex-wrap:wrap}.path-field{margin-top:8px}.path-field input{flex:1;margin-top:0!important}.path-field .button{flex-shrink:0}.schedule-form small,.muted,.introduction{display:block;font-size:11px;font-weight:400;color:var(--text-muted);line-height:1.9;overflow-wrap:anywhere;margin-top:6px}.introduction{background:var(--bg-input);border:1px solid var(--border-subtle);padding:14px;border-radius:12px;margin:0}.sources{max-height:160px;overflow:auto;margin-top:10px;border:1px solid var(--border-subtle);padding:10px 12px;border-radius:10px}.source{display:flex;align-items:center;gap:14px;padding:5px 0;font-size:11px}.source span{flex:1;overflow-wrap:anywhere;min-width:0}.source button{flex-shrink:0}.check,.day{display:flex;align-items:center;gap:8px}.weekdays{display:flex;gap:12px;flex-wrap:wrap}.day{padding:8px;border:1px solid var(--border-subtle);border-radius:8px}.button{border:1px solid var(--border-subtle);background:var(--bg-input);border-radius:10px;padding:9px 14px;font-size:12px;color:var(--text-content)}.button.primary{background:var(--dynamic-accent);color:white}.button:disabled{opacity:.45}.link{color:var(--dynamic-accent)}.failure{font-size:12px;color:#ef4444;line-height:1.8;margin-top:15px;overflow-wrap:anywhere}@media(max-width:800px){.form-grid{grid-template-columns:minmax(0,1fr);gap:14px}.schedule-form{gap:14px}}
html.dark .schedule-form,html.mode-cyberpunk .schedule-form,html.mode-twilight .schedule-form{color-scheme:dark}
</style>
