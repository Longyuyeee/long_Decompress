<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import ComparisonPackageSources from './ComparisonPackageSources.vue'
import { computed,ref } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import { useZipEditStore } from '@/stores/zipEdit'
import { usePasswordStore,type PasswordEntry } from '@/stores/password'
const props=defineProps<{sourcePath:string}>()
const store=useZipEditStore(),vault=usePasswordStore(),picker=ref(false),field=ref<'sourcePassword'|'outputPassword'>('sourcePassword'),query=ref(''),error=ref('')
const session=computed(()=>store.sessionFor(props.sourcePath))
const entries=computed(()=>[...vault.entries].filter(entry=>(entry.name+' '+entry.tags.join(' ')).toLowerCase().includes(query.value.toLowerCase())).sort((a,b)=>Number(b.favorite)-Number(a.favorite)||a.name.localeCompare(b.name)))
const encryptedOutput=computed(()=>session.value&&(session.value.outputMode==='keep'?session.value.catalog.encrypted:session.value.outputMode!=='none'))
const encryptionLabel=(value?:string)=>value==='zipcrypto'?'ZipCrypto':value==='aes128'?'AES-128':value==='aes192'?'AES-192':value==='aes256'?'AES-256':'无密码'
async function selectPassword(target:typeof field.value){field.value=target;query.value='';error.value='';picker.value=true;await vault.checkUnlockStatus()}
async function choose(entry:PasswordEntry){try{if(session.value)session.value[field.value]=await vault.usePassword(entry.id)||entry.password;picker.value=false}catch(value){error.value=String(value)}}
</script>
<template>
 <fieldset v-if="session" :disabled="session.busy" class="min-w-0 space-y-3 rounded-xl border border-subtle bg-input/20 p-3" data-testid="zip-edit-credentials">
  <ComparisonPackageSources :source-path="sourcePath"/>
  <strong class="text-xs text-content"><i class="pi pi-key mr-2 text-primary"></i>原包与新包的密码</strong>
  <template v-if="session.catalog.encrypted">
   <label class="block text-xs text-muted">原包密码<div class="mt-2 flex min-w-0 gap-2"><input v-model="session.sourcePassword" type="password" autocomplete="off" class="password-input min-w-0 flex-1" data-testid="zip-edit-source-password" placeholder="输入原包密码" /><button class="password-button text-primary" data-testid="zip-edit-source-vault" @click="selectPassword('sourcePassword')">保险箱</button></div></label>
   <ZipPasswordEncodingField v-model="session.sourceEncoding" label="原包密码编码" test-id="zip-edit-source-encoding" />
  </template>
  <p v-else-if="session.drafts.some(row=>row.comparison)" class="text-xs text-muted">来源密码只用于取材。新 ZIP 的密码在下方单独设置。</p><p v-else class="text-xs text-muted">原包没有密码内容。</p>
  <label class="block text-xs text-muted">新 ZIP 的加密方式<ThemedSelect v-model="session.outputMode" :options="[{value:'keep',label:session.catalog.encrypted?'保留原加密与密码':'保持无密码'},{value:'aes256',label:'设置新密码 · AES-256'},{value:'zipcrypto',label:'设置新密码 · ZipCrypto（传统兼容，较弱）'},{value:'none',label:'新包不加密'}]" test-id="zip-edit-output-mode" floating /></label>
  <label v-if="['aes256','zipcrypto'].includes(session.outputMode)" class="block text-xs text-muted">新包密码<div class="mt-2 flex min-w-0 gap-2"><input v-model="session.outputPassword" type="password" autocomplete="new-password" class="password-input min-w-0 flex-1" data-testid="zip-edit-output-password" placeholder="重新填写，或从保险箱选择" /><button class="password-button text-primary" data-testid="zip-edit-output-vault" @click="selectPassword('outputPassword')">保险箱</button></div></label>
  <ZipPasswordEncodingField v-if="encryptedOutput" v-model="session.outputEncoding" mode="output" label="新包密码编码" test-id="zip-edit-output-encoding" />
  <p v-if="session.outputMode==='keep'&&session.catalog.encrypted" class="text-xs leading-relaxed text-muted">原条目保留各自加密方案及公开状态；新增条目使用 {{ encryptionLabel(session.catalog.defaultEncryption) }}，替换沿原条目方案。密码内容会重新写入，新包密码字节按上方编码保存。</p>
  <p v-else-if="session.outputMode==='none'" class="text-xs leading-relaxed text-muted">新包不设密码，原包及其密码保持不变。</p>
  <span class="context-help"><HelpHint label="编辑密码说明">草稿只保存修改和配置，不保存密码；恢复后请重新填写。ZIP 密码不隐藏包内名称。</HelpHint></span>
 </fieldset>
 <Modal v-model:visible="picker" title="从密码保险箱选择" size="sm" layer="nested" cancel-text="返回密码设置">
  <input v-model="query" class="password-input mb-3 w-full" placeholder="搜索密码名称或标签" aria-label="搜索保险箱" />
  <p v-if="vault.isLoading" class="text-xs text-muted">正在读取保险箱…</p><p v-else-if="!vault.isUnlocked" class="text-xs text-muted">保险箱当前不可用，请到密码保险箱页面解锁。</p>
  <ul v-else class="max-h-64 space-y-2 overflow-y-auto custom-scrollbar"><li v-for="entry in entries" :key="entry.id"><button class="password-button w-full justify-start text-left" :data-vault-entry="entry.name" @click="choose(entry)"><i :class="entry.favorite?'pi pi-star-fill':'pi pi-key'" class="text-primary"></i><span class="min-w-0 break-words [overflow-wrap:anywhere]">{{ entry.name }}</span></button></li><li v-if="!entries.length" class="text-xs text-muted">没有匹配的密码。</li></ul><p v-if="error" class="mt-2 text-xs text-red-500">{{ error }}</p>
 </Modal>
</template>
<style scoped>
.password-input{min-width:0;border:1px solid var(--border-subtle);border-radius:.65rem;background:var(--bg-input);padding:.55rem .65rem;font-size:.75rem}.password-button{display:inline-flex;align-items:center;justify-content:center;gap:.5rem;border:1px solid var(--border-subtle);border-radius:.65rem;padding:.5rem .7rem;font-size:.75rem}button:focus-visible,input:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:2px}
</style>
