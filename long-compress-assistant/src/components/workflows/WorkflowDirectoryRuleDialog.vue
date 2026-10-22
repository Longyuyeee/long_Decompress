<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import type { WorkflowPlan } from '@/types/workflow'
import type { WorkflowDirectoryRule } from '@/types/workflowTrigger'
const props=defineProps<{rule?:WorkflowDirectoryRule;plans:WorkflowPlan[];busy:boolean;error:string}>()
const emit=defineEmits<{close:[];save:[rule:WorkflowDirectoryRule]}>()
const draft=ref<WorkflowDirectoryRule>(props.rule?JSON.parse(JSON.stringify(props.rule)):{id:crypto.randomUUID(),name:'目录自动归档',planId:props.plans[0]?.id||'',root:'',outputDirectory:'',mode:'files',extensions:'',recursive:true,settleSeconds:5,enabled:true,includeExisting:false,note:''})
const picking=ref(false)
async function choose(target:'root'|'outputDirectory'){picking.value=true;try{const supplied=import.meta.env.VITE_DESKTOP_E2E==='1'?window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection():undefined;const value=supplied===undefined?await open({directory:true,multiple:false,title:target==='root'?'选择监控目录':'选择保存位置'}):supplied;if(typeof value==='string')draft.value[target]=value}finally{picking.value=false}}
</script>
<template>
 <Modal description="把资料放入目录后，按已保存方案自动处理。仅在应用运行或托盘驻留时执行；暂停规则不影响已开始的运行。" description-as-help :visible="true" :title="rule?'编辑目录规则':'添加目录规则'" icon="pi pi-folder-open" size="lg" :show-footer="true" :show-close-button="!busy" :close-on-backdrop="!busy" :close-on-escape="!busy" @update:visible="emit('close')">
  <fieldset :disabled="busy||picking" class="rule-form" data-testid="workflow-rule-dialog">

   <div class="form-grid"><label>规则名称<input v-model="draft.name" maxlength="60" data-testid="workflow-rule-name"></label><label>使用方案<select v-model="draft.planId" data-testid="workflow-rule-plan"><option value="" disabled>先保存一套方案</option><option v-for="plan in plans" :key="plan.id" :value="plan.id">{{ plan.name }}</option></select></label></div>
   <label>监控目录<div class="path-field"><input v-model="draft.root" data-testid="workflow-rule-root" placeholder="资料放入这里后触发"><button type="button" class="button" data-testid="workflow-rule-choose-root" @click="choose('root')">选择</button></div></label>
   <label>保存位置<div class="path-field"><input v-model="draft.outputDirectory" data-testid="workflow-rule-output" placeholder="每次使用新的独立运行目录"><button type="button" class="button" data-testid="workflow-rule-choose-output" @click="choose('outputDirectory')">选择</button></div><small>可以放在监控目录的子目录中；注册的输出位置会排除，避免再次触发。</small></label>
   <div class="form-grid"><label>一份来源是什么<select v-model="draft.mode" data-testid="workflow-rule-mode"><option value="files">每个文件 / 整组分卷</option><option value="folders">每个顶层文件夹</option></select><small>{{ draft.mode==='folders'?'适合每个项目文件夹分别归档；等待内部文件稳定。':'同组分卷只入队一次，未齐时显示等待原因。' }}</small></label><label>稳定等待（秒）<input v-model.number="draft.settleSeconds" type="number" min="3" max="300" data-testid="workflow-rule-settle"><small>连续 {{ draft.settleSeconds }} 秒未观察到变化后才开始。</small></label></div>
   <template v-if="draft.mode==='files'"><label>接收扩展名<input v-model="draft.extensions" maxlength="512" data-testid="workflow-rule-extensions" placeholder="例如 zip,7z,rar；留空接收全部文件"><small>分卷按归档扩展名匹配；临时文件可用扩展名筛选避开。</small></label><label class="check"><input v-model="draft.recursive" type="checkbox" data-testid="workflow-rule-recursive">同时观察子目录</label></template>
   <section class="options"><label class="check"><input v-model="draft.includeExisting" type="checkbox" data-testid="workflow-rule-existing">{{ rule?'更换来源范围时，也处理其中已有内容':'也处理目录中已有内容' }}</label><span class="context-help"><HelpHint label="变化接收说明">默认只接收创建后新增或变化的来源。编辑不会改变已排队项的方案快照与保存位置。</HelpHint></span><label class="check"><input v-model="draft.enabled" type="checkbox" data-testid="workflow-rule-enabled">保存后启用自动执行</label><span class="context-help"><HelpHint label="自动化密码说明">密码在“本次密码”中提供，只在当前会话使用；需要密码的步骤会等待填写。</HelpHint></span></section>
  </fieldset>
  <p v-if="error" class="failure" role="alert">{{ error }}</p>
  <template #footer><button class="button" :disabled="busy" @click="emit('close')">返回</button><button class="button primary" data-testid="workflow-rule-save" :disabled="busy||picking||!draft.planId||!draft.root.trim()||!draft.outputDirectory.trim()||!draft.name.trim()" @click="emit('save',draft)">{{ busy?'保存中…':draft.enabled?'保存并启用':'保存为暂停' }}</button></template>
 </Modal>
</template>
<style scoped>
.rule-form{display:flex;flex-direction:column;gap:18px;min-width:0}.rule-form label{display:block;font-size:12px;font-weight:700;min-width:0}.rule-form input:not([type=checkbox]),select{display:block;width:100%;min-width:0;margin-top:8px;border:1px solid var(--border-subtle);border-radius:10px;background:var(--bg-input);padding:10px 12px;font-size:12px;color:var(--text-content)}.form-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px}.path-field{display:flex;gap:8px;min-width:0;margin-top:8px}.path-field input{flex:1;margin-top:0!important}.path-field .button{flex-shrink:0}.rule-form small,.note{display:block;font-size:11px;font-weight:400;color:var(--text-muted);line-height:1.9;overflow-wrap:anywhere;margin-top:6px}.introduction,.options{background:var(--bg-input);border:1px solid var(--border-subtle);padding:14px;border-radius:12px}.check{display:flex!important;align-items:center;gap:9px}.options .check:not(:first-child){margin-top:14px}.button{border:1px solid var(--border-subtle);background:var(--bg-input);border-radius:10px;padding:9px 14px;font-size:12px;color:var(--text-content)}.button.primary{background:var(--dynamic-accent);color:white}.button:disabled{opacity:.45}.failure{font-size:12px;color:#ef4444;line-height:1.8;margin-top:15px;overflow-wrap:anywhere}@media(max-width:800px){.form-grid{grid-template-columns:minmax(0,1fr);gap:14px}.rule-form{gap:14px}}
</style>
