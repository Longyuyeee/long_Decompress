<script setup lang="ts">
import {ref,watch} from 'vue'
import {invoke} from '@tauri-apps/api/tauri'
import Modal from '@/components/ui/Modal.vue'
import {pendingZpaqGroup,finishZpaqGroup} from '@/utils/zpaqMultipart'
const busy=ref(false),error=ref('')
watch(pendingZpaqGroup,()=>{error.value=''})
const name=(path:string)=>path.replace(/\\/g,'/').split('/').pop()
const size=(bytes:number)=>bytes<1048576?`${(bytes/1024).toFixed(1)} KB`:`${(bytes/1048576).toFixed(1)} MB`
async function confirm(single=false){
 const pending=pendingZpaqGroup.value;if(!pending||busy.value)return;busy.value=true;error.value=''
 try{const group=single?{...pending.group,members:pending.group.members.filter(m=>name(m.path)?.toLowerCase()===name(pending.path)?.toLowerCase()),missing:[]}:pending.group
  const result=await invoke<string>('confirm_zpaq_multipart',{group});finishZpaqGroup(result)
 }catch(e){error.value=String(e)}finally{busy.value=false}
}
</script>
<template>
 <Modal :visible="!!pendingZpaqGroup" title="确认 ZPAQ 来源" description="这些文件可能共同组成一份备份" icon="pi pi-box" size="lg" :close-on-backdrop="false" @close="!busy&&finishZpaqGroup(null)">
  <section v-if="pendingZpaqGroup" data-testid="zpaq-multipart-confirm" class="group-confirm">
   <p>原生多部分按编号一起读取，历史快照也依赖此前的部分。请确认以下文件属于同一份备份。</p>
   <div class="group-summary"><strong>{{pendingZpaqGroup.group.members.length}} 个部分</strong><span>{{size(pendingZpaqGroup.group.members.reduce((sum,m)=>sum+m.size,0))}}</span></div>
   <ol><li v-for="(member,index) in pendingZpaqGroup.group.members" :key="member.path" :title="member.path"><span class="part-index">{{String(index+1).padStart(2,'0')}}</span><span class="part-name">{{name(member.path)}}<small>{{member.path.replace(/\\/g,'/').split('/').slice(0,-1).join('/')}}</small></span><span>{{size(member.size)}}</span></li></ol>
   <p v-if="pendingZpaqGroup.group.missing.length" class="group-error" data-testid="zpaq-multipart-missing">缺少：{{pendingZpaqGroup.group.missing.join('、')}}。补齐后重新选择任一成员；现在不会只读取前半份备份。</p>
   <p class="group-note">如果它们是几份独立的压缩包，请选择“仅打开此文件”。这里不修改原件，也不保存密码。</p>
   <p v-if="error" class="group-error" role="alert">{{error}}</p>
  </section>
  <template #footer><div class="group-actions"><button :disabled="busy" class="browser-secondary" @click="finishZpaqGroup(null)">取消</button><button data-testid="zpaq-multipart-single" :disabled="busy" class="browser-secondary" @click="confirm(true)">仅打开此文件</button><button data-testid="zpaq-multipart-accept" :disabled="busy||!!pendingZpaqGroup?.group.missing.length" class="browser-primary" @click="confirm()">{{busy?'确认中…':'确认整组并打开'}}</button></div></template>
 </Modal>
</template>
<style scoped>
.group-confirm{display:grid;gap:1rem;min-width:0;font-size:.8rem;line-height:1.65}.group-summary{display:flex;justify-content:space-between;padding:.8rem 1rem;border-radius:1rem;background:rgb(var(--color-primary-rgb,14 165 233)/.08);color:var(--color-primary,#0ea5e9)}ol{list-style:none;margin:0;padding:0;border:1px solid var(--border-subtle);border-radius:1rem;overflow:hidden}li{display:flex;align-items:center;gap:.8rem;padding:.8rem;border-bottom:1px solid var(--border-subtle);min-width:0}li:last-child{border:0}.part-index{color:var(--color-primary,#0ea5e9);font-weight:900}.part-name{flex:1;min-width:0;overflow-wrap:anywhere;font-weight:700}.part-name small{display:block;font-size:.65rem;opacity:.6;font-weight:400;overflow-wrap:anywhere}.group-note{opacity:.65}.group-error{color:#f59e0b;overflow-wrap:anywhere}.group-actions{display:flex;flex-wrap:wrap;justify-content:flex-end;gap:.6rem;min-width:0}.group-actions button{padding:.7rem 1rem;border-radius:.8rem;font-size:.75rem;font-weight:700;border:1px solid var(--border-subtle)}.group-actions .browser-primary{background:#0ea5e9;color:white}.group-actions button:disabled{opacity:.45}
</style>
