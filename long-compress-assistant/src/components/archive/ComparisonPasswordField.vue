<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import {computed,ref} from 'vue'
import Modal from '@/components/ui/Modal.vue'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import {usePasswordStore,type PasswordEntry} from '@/stores/password'
import type {ZipPasswordEncoding} from '@/types/zipPassword'
const props=defineProps<{password?:string;encoding?:ZipPasswordEncoding;disabled?:boolean;testId?:string}>()
const emit=defineEmits<{'update:password':[string];'update:encoding':[ZipPasswordEncoding|undefined]}>()
const vault=usePasswordStore(),picker=ref(false),query=ref(''),error=ref('')
const entries=computed(()=>[...vault.entries].filter(entry=>(entry.name+' '+entry.tags.join(' ')).toLowerCase().includes(query.value.toLowerCase())).sort((a,b)=>Number(b.favorite)-Number(a.favorite)||a.name.localeCompare(b.name)))
async function openVault(){query.value='';error.value='';picker.value=true;try{await vault.checkUnlockStatus()}catch(cause){error.value=String(cause)}}
async function choose(entry:PasswordEntry){try{emit('update:password',await vault.usePassword(entry.id)||entry.password);picker.value=false}catch(cause){error.value=String(cause)}}
</script>
<template>
 <fieldset :disabled="disabled" class="min-w-0 space-y-3">
  <label class="block text-xs text-muted">本次 ZIP 密码<div class="mt-2 flex min-w-0 gap-2"><input :value="password" type="password" autocomplete="off" class="control min-w-0 flex-1" :data-testid="testId" placeholder="输入密码，或从保险箱选择" @input="emit('update:password',($event.target as HTMLInputElement).value)"><button class="control text-primary" :data-testid="testId+'-vault'" type="button" @click="openVault">保险箱</button></div></label>
  <ZipPasswordEncodingField :model-value="encoding" :disabled="disabled" :test-id="testId+'-encoding'" @update:model-value="emit('update:encoding',$event)"/>
  <span class="context-help"><HelpHint label="密码使用说明">密码仅用于本次会话；草稿和历史保留所选范围，重启后重新填写。</HelpHint></span>
 </fieldset>
 <Modal v-model:visible="picker" title="从密码保险箱选择" size="sm" layer="nested" cancel-text="返回密码设置">
  <input v-model="query" class="control mb-3 w-full" placeholder="搜索密码名称或标签" aria-label="搜索保险箱">
  <p v-if="vault.isLoading" class="text-xs text-muted">正在读取保险箱…</p><p v-else-if="!vault.isUnlocked" class="text-xs text-muted">保险箱尚未解锁，请到密码保险箱页面解锁。</p>
  <ul v-else class="max-h-64 space-y-2 overflow-y-auto custom-scrollbar"><li v-for="entry in entries" :key="entry.id"><button class="control w-full text-left" :data-vault-entry="entry.name" @click="choose(entry)"><i class="pi pi-key text-primary mr-2"/>{{ entry.name }}</button></li><li v-if="!entries.length" class="text-xs text-muted">没有匹配的密码。</li></ul>
  <p v-if="error" class="text-xs text-red-500 mt-2">{{ error }}</p>
 </Modal>
</template>
<style scoped>
.control{min-width:0;border:1px solid var(--border-subtle);border-radius:10px;background:var(--bg-input);color:var(--text-content);padding:9px 12px;font-size:12px}input:focus-visible,button:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:2px}
</style>
