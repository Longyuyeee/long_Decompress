<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { computed, ref, watch } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import { usePasswordStore, type PasswordEntry } from '@/stores/password'
import type { LibrarySource, LibraryCredentials } from '@/types/archiveLibrary'
import { isZipPasswordEncoding, type ZipPasswordEncoding } from '@/types/zipPassword'
import { supportsZipPasswordEncoding } from '@/utils/zipPasswordSource'
const props = defineProps<{ source: LibrarySource; initial?: LibraryCredentials; busy: boolean; error?: string; action?: string }>()
const emit = defineEmits<{ close: []; apply: [value: LibraryCredentials]; stop: [] }>()
const password = ref(''), encoding = ref<ZipPasswordEncoding>(), picker = ref(false), query = ref(''), vaultError = ref('')
const vault = usePasswordStore()
watch(() => props.source.id, () => { password.value = props.initial?.password || ''; encoding.value = props.initial?.encoding || (isZipPasswordEncoding(props.source.sourceEncoding) ? props.source.sourceEncoding : undefined) }, { immediate: true })
const entries = computed(() => vault.entries.filter(entry => `${entry.name} ${entry.tags.join(' ')}`.toLowerCase().includes(query.value.toLowerCase())).sort((a,b) => Number(b.favorite)-Number(a.favorite)||a.name.localeCompare(b.name)))
async function openVault() { picker.value = true; vaultError.value = ''; await vault.checkUnlockStatus() }
async function choose(entry: PasswordEntry) { try { password.value = await vault.usePassword(entry.id) || entry.password; picker.value = false } catch (error) { vaultError.value = String(error) } }
</script>
<template>
  <Modal :visible="true" title="读取这份密码资料" icon="pi pi-key" size="md" :show-footer="true" :show-close-button="!busy" :close-on-backdrop="!busy" :close-on-escape="!busy" @update:visible="!busy && emit('close')">
    <div class="space-y-4" data-testid="library-credentials">
      <div class="rounded-xl border border-subtle bg-input/30 p-3"><strong class="block break-all text-sm">{{ source.name }}</strong><p class="mt-2 break-all text-xs text-muted">{{ source.path }}</p></div>
      <label class="block text-xs font-bold text-muted">来源密码<div class="mt-2 flex min-w-0 gap-2"><input v-model="password" type="password" autocomplete="off" class="library-password min-w-0 flex-1" data-testid="library-password" placeholder="填写密码，或从保险箱选择" :disabled="busy"><button class="library-password text-primary" data-testid="library-vault" :disabled="busy" @click="openVault">保险箱</button></div></label>
      <ZipPasswordEncodingField v-if="supportsZipPasswordEncoding(source.path) || supportsZipPasswordEncoding(source.path, source.format)" v-model="encoding" :source-path="source.path" :source-format="source.format" label="来源密码编码" test-id="library-password-encoding" :disabled="busy" />
      <span class="context-help"><HelpHint label="资料密码说明">密码只在当前应用会话使用，不写入资料库或索引。重新打开应用后，需要再次填写或从保险箱选择。读取加密内容可能需要一些时间，可以从资源库停止。</HelpHint></span>
      <p v-if="error" class="break-all text-xs leading-relaxed text-red-500" role="alert" data-testid="library-credential-error">{{ error }}</p>
    </div>
    <template #footer><button v-if="busy" class="library-password" data-testid="library-credential-stop" @click="emit('stop')">停止读取</button><button class="library-password" :disabled="busy" @click="emit('close')">返回</button><button class="library-password bg-primary text-white" data-testid="library-credential-apply" :disabled="busy" @click="emit('apply', { password, encoding })">{{ busy ? '正在读取…' : action || '读取并更新索引' }}</button></template>
  </Modal>
  <Modal v-model:visible="picker" title="从密码保险箱选择" size="sm" layer="nested" cancel-text="返回来源密码">
    <input v-model="query" class="library-password mb-3 w-full" placeholder="搜索密码名称或标签" aria-label="搜索保险箱">
    <p v-if="vault.isLoading" class="text-xs text-muted">正在读取保险箱…</p><p v-else-if="!vault.isUnlocked" class="text-xs text-muted">保险箱当前不可用，请到密码保险箱页面解锁。</p>
    <ul v-else class="max-h-64 space-y-2 overflow-y-auto custom-scrollbar"><li v-for="entry in entries" :key="entry.id"><button class="library-password w-full text-left text-primary" :data-vault-entry="entry.name" @click="choose(entry)">{{ entry.name }}</button></li><li v-if="!entries.length" class="text-xs text-muted">没有匹配的密码。</li></ul><p v-if="vaultError" class="mt-3 text-xs text-red-500">{{ vaultError }}</p>
  </Modal>
</template>
<style scoped>.library-password{border:1px solid var(--border-subtle);border-radius:.7rem;background:var(--bg-input);padding:.6rem .8rem;font-size:.75rem;overflow-wrap:anywhere}.library-password.bg-primary{background:var(--dynamic-accent)}.library-password:disabled{opacity:.5}</style>
