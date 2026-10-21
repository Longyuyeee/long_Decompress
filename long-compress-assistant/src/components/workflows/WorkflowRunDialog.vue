<script setup lang="ts">
import { computed,onUnmounted,ref,watch } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import { usePasswordStore,type PasswordEntry } from '@/stores/password'
import { workflowKinds,type WorkflowPlan,type WorkflowRunRequest,type WorkflowProofProgress,type WorkflowReuseIssue } from '@/types/workflow'
import { effectiveFormatForPassword } from '@/utils/compressionFormat'
import { formatFileSize } from '@/utils'
import type { ZipPasswordEncoding } from '@/types/zipPassword'
const props=withDefaults(defineProps<{
  plan:WorkflowPlan;error?:string;initial?:Partial<WorkflowRunRequest>;startIndex?:number;recovery?:boolean;busy?:boolean;phase?:string;progress?:WorkflowProofProgress;issue?:WorkflowReuseIssue;
  inputCredential?:{visible:boolean;required:boolean;label:string;encoding?:ZipPasswordEncoding}
}>(),{startIndex:0})
const emit=defineEmits<{close:[];run:[request:WorkflowRunRequest];sources:[paths:string[]];stop:[];earlier:[index:number]}>()
const paths=ref<string[]>([...(props.initial?.paths||[])]),output=ref(props.initial?.outputDirectory||''),sourcePassword=ref(''),sourceEncoding=ref<ZipPasswordEncoding|undefined>(props.initial?.sourceEncoding)
const passwords=ref<Record<string,string>>(Object.fromEntries(props.plan.steps.map(step=>[step.id,step.settings.password||'']))),picker=ref(''),vaultQuery=ref(''),vaultError=ref(''),picking=ref(false)
const vault=usePasswordStore(),entries=computed(()=>vault.entries.filter(entry=>`${entry.name} ${entry.tags.join(' ')}`.includes(vaultQuery.value)))
const archiveFirst=computed(()=>['extract','convert'].includes(props.plan.steps[0]?.kind))
const showSourceCredential=computed(()=>props.inputCredential?props.inputCredential.visible:props.plan.steps.some(step=>['extract','convert'].includes(step.kind)))
const credentialLabel=computed(()=>props.inputCredential?.label||'原始来源密码（可留空）')
const outputSteps=computed(()=>props.plan.steps.slice(props.startIndex).filter(step=>['compress','convert'].includes(step.kind)&&step.passwordRequired))
watch(paths,value=>emit('sources',[...value]),{deep:true,immediate:true})
watch(()=>props.inputCredential?.encoding,value=>{sourceEncoding.value=value})
watch(()=>props.inputCredential?.label,()=>{sourcePassword.value=''})
const queued=()=>import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined
async function choose(directory:boolean,destination=false){
  if(props.busy)return;picking.value=true
  try{const supplied=queued(),value=supplied===undefined?await open({directory,multiple:!destination&&!directory,title:destination?'选择本次保存位置':'选择工作流来源'}):supplied
    if(!value)return;if(destination&&typeof value==='string')output.value=value;else paths.value=[...new Set([...paths.value,...(Array.isArray(value)?value:[value])])]
  }finally{picking.value=false}
}
async function openVault(target:string){picker.value=target;vaultError.value='';await vault.checkUnlockStatus()}
async function choosePassword(entry:PasswordEntry){try{const value=await vault.usePassword(entry.id)||entry.password;if(picker.value==='source')sourcePassword.value=value;else passwords.value[picker.value]=value;picker.value=''}catch(error){vaultError.value=String(error)}}
function submit(){emit('run',{paths:[...paths.value],outputDirectory:output.value,sourcePassword:sourcePassword.value,sourceEncoding:sourceEncoding.value,passwords:{...passwords.value}})}
onUnmounted(()=>{sourcePassword.value='';passwords.value={}})
</script>
<template>
  <Modal :visible="true" :title="recovery?'纠正并接续这次运行':'确认并运行这套方案'" icon="pi pi-play" size="xl" :show-footer="true" :show-close-button="!busy" :close-on-backdrop="!busy" :close-on-escape="!busy" @update:visible="emit('close')">
    <slot name="before" />
    <fieldset :disabled="busy" class="workflow-run-grid" :class="{'one-column':recovery&&!showSourceCredential&&!outputSteps.length}" data-testid="workflow-run-dialog">
      <section class="run-card">
        <h3>{{ recovery?'原始来源':'本次处理哪些资料' }}</h3>
        <p class="note">{{ archiveFirst?'选择归档或任意分卷成员，同组只处理一次。':'选择文件或目录，按方案读取并筛选。' }}<template v-if="recovery">更换原始来源会从第 1 步重新处理。</template></p>
        <div class="actions my-3"><button class="control" data-testid="workflow-choose-files" :disabled="picking||busy" @click="choose(false)"><i class="pi pi-file"/>选择文件</button><button v-if="!archiveFirst" class="control" data-testid="workflow-choose-folder" :disabled="picking||busy" @click="choose(true)"><i class="pi pi-folder"/>选择文件夹</button><button v-if="paths.length" class="control" data-testid="workflow-clear-sources" :disabled="busy" @click="paths=[]">清空选择</button></div>
        <div class="source-list custom-scrollbar"><div v-for="path in paths" :key="path" class="source-path"><span>{{ path }}</span><button aria-label="移除此来源" @click="paths=paths.filter(item=>item!==path)"><i class="pi pi-times"/></button></div><p v-if="!paths.length" class="note py-4">还没有选择来源</p></div>
        <p class="note mt-2">已选 {{ paths.length }} 处来源，原文件保留。</p>
        <label class="field mt-4">本次保存位置<div class="flex min-w-0 gap-2 mt-2"><input v-model="output" class="control flex-1 min-w-0" data-testid="workflow-output" placeholder="选择已有文件夹"><button class="control shrink-0" data-testid="workflow-choose-output" @click="choose(true,true)">选择</button></div></label>
        <p class="note mt-2">{{ recovery?'重做步骤保存到新的独立运行目录，复用结果沿用原位置。':'每次建立独立运行目录，各步产物分开保存，已有结果保留。' }}</p>
      </section>
      <section v-if="!recovery||showSourceCredential||outputSteps.length" class="run-card">
        <h3>{{ recovery?'本次读取与输出密码':plan.name }}</h3>
        <ol v-if="!recovery" class="space-y-3 mt-3"><li v-for="(step,index) in plan.steps" :key="step.id" class="step-preview"><span class="step-number">{{ index+1 }}</span><div class="min-w-0"><strong class="break-all">{{ step.name }}</strong><p class="note">{{ workflowKinds[step.kind].label }} · {{ recovery&&index<startIndex?'核对后复用已有产物':index?'上一步实际产物':'本次选择的来源' }}</p><p v-if="step.kind==='filter'" class="note">扩展名：{{ step.extensions||'全部文件' }}</p><p v-if="['compress','convert'].includes(step.kind)" class="note break-all">{{ step.settings.filename||'{name}-{date}' }}.{{ effectiveFormatForPassword(step.settings.format,passwords[step.id]||(step.passwordRequired?'required':'')) }}</p></div></li></ol>
        <div v-if="showSourceCredential" class="mt-4 space-y-3">
          <label class="field">{{ credentialLabel }}<span v-if="inputCredential?.required" class="text-primary ml-1">必填</span><div class="flex gap-2 mt-2 min-w-0"><input v-model="sourcePassword" type="password" autocomplete="off" class="control min-w-0 flex-1" data-testid="workflow-source-password"><button class="control shrink-0" data-testid="workflow-source-vault" @click="openVault('source')">保险箱</button></div></label>
          <ZipPasswordEncodingField v-model="sourceEncoding" label="ZIP / ALZ / EGG 读取编码" :source-path="paths[0]||''" test-id="workflow-source-encoding" />
        </div>
        <label v-for="step in outputSteps" :key="step.id" class="field mt-4">{{ step.name }} · 本次输出密码<div class="flex gap-2 mt-2 min-w-0"><input v-model="passwords[step.id]" type="password" autocomplete="off" class="control flex-1 min-w-0" :data-step-password="step.id"><button class="control shrink-0" @click="openVault(step.id)">保险箱</button></div></label>
        <p class="note mt-4">密码只在当前会话使用。重做步骤生成的密码包，会将本次凭据带给后续步骤。</p>
      </section>
    </fieldset>
    <div v-if="busy" class="check-progress mt-4" data-testid="workflow-reuse-progress" role="status"><strong><i class="pi pi-spin pi-spinner mr-2"/>{{ phase||'正在准备接续' }}</strong><p v-if="progress" class="note">本次读取 {{ formatFileSize(progress.bytes) }} · {{ progress.files }} 个文件<span v-if="progress.path" class="block break-all">{{ progress.path }}</span></p></div>
    <p v-if="error" class="error mt-3" role="alert">{{ error }}</p>
    <button v-if="issue&&!busy" class="control mt-3" data-testid="workflow-reuse-earlier" @click="emit('earlier',issue.index)">从第 {{ issue.index+1 }} 步重新处理</button>
    <template #footer><button v-if="busy" class="control" data-testid="workflow-check-stop" @click="emit('stop')">停止核对</button><template v-else><button class="control" @click="emit('close')">返回</button><button class="control primary" data-testid="workflow-run-confirm" :disabled="picking||!paths.length||!output.trim()" @click="submit"><i class="pi pi-play"/>{{ recovery?'核对并开始本次接续':'开始本次工作流' }}</button></template></template>
  </Modal>
  <Modal :visible="!!picker" title="从保险箱选择密码" size="sm" layer="nested" @update:visible="picker=''"><input v-model="vaultQuery" class="control w-full mb-3" placeholder="搜索密码名称或标签"><p v-if="!vault.isUnlocked" class="note">请先到密码保险箱页面解锁。</p><ul v-else class="space-y-2 max-h-64 overflow-auto"><li v-for="entry in entries" :key="entry.id"><button class="control w-full text-left" :data-workflow-vault="entry.name" @click="choosePassword(entry)">{{ entry.name }}</button></li></ul><p v-if="vaultError" class="error">{{ vaultError }}</p></Modal>
</template>
<style scoped>
.workflow-run-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px;min-width:0}.workflow-run-grid.one-column{grid-template-columns:minmax(0,1fr)}.run-card{min-width:0;border:1px solid var(--border-subtle);border-radius:16px;background:var(--bg-card);padding:18px}h3{font-weight:900;font-size:15px}.note{font-size:11px;color:var(--text-muted);line-height:1.9;overflow-wrap:anywhere}.control{display:inline-flex;align-items:center;justify-content:center;gap:7px;min-width:0;border:1px solid var(--border-subtle);border-radius:10px;padding:9px 12px;font-size:12px;background:var(--bg-input)}.control.primary{background:var(--dynamic-accent);color:white}.control:disabled{opacity:.45}.field{display:block;font-size:12px;font-weight:700}.source-list{max-height:190px;overflow:auto}.source-path{display:flex;gap:8px;padding:10px;border-bottom:1px solid var(--border-subtle);font-size:11px}.source-path span{min-width:0;flex:1;overflow-wrap:anywhere}.actions{display:flex;gap:8px;flex-wrap:wrap}.step-preview{display:flex;gap:10px;font-size:12px;border:1px solid var(--border-subtle);border-radius:12px;padding:12px}.step-number{display:grid;place-items:center;border-radius:9px;width:28px;height:28px;flex-shrink:0;background:var(--bg-input);color:var(--dynamic-accent);font-weight:900}.error{font-size:12px;color:#ef4444;line-height:1.8;overflow-wrap:anywhere}.check-progress{font-size:12px;padding:14px;border:1px solid var(--border-subtle);border-radius:12px;background:var(--bg-input)}@media(max-width:800px){.workflow-run-grid{grid-template-columns:minmax(0,1fr)}.run-card{padding:14px}.source-list{max-height:150px}}
</style>
