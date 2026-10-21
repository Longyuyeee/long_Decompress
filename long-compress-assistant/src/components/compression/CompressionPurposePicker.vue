<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { compressionPurposes, compressionPurposePatch, type CompressionPurposeId, type PurposePatch } from '@/utils/compressionPurpose'

const props = withDefaults(defineProps<{ hasPassword?: boolean; allowSplit?: boolean; disabled?: boolean; staged?: boolean }>(), { allowSplit: true })
const emit = defineEmits<{ apply: [patch: PurposePatch] }>()
const selected = ref<CompressionPurposeId>()
const password = ref('')
const reuse = ref(false)
const splitMiB = ref('1024')
const message = ref('')
const purpose = computed(() => compressionPurposes.find(item => item.id === selected.value))
const valid = computed(() => selected.value === 'encrypted' ? reuse.value && props.hasPassword || password.value.length > 0
  : selected.value === 'split' ? Number.isSafeInteger(Number(splitMiB.value)) && Number(splitMiB.value) >= 1 : !!selected.value)
function select(id: CompressionPurposeId) {
  selected.value = selected.value === id ? undefined : id
  password.value = ''; reuse.value = props.hasPassword; message.value = ''
}
watch(() => props.hasPassword, has => { if (!has) reuse.value = false })
watch(() => props.disabled, disabled => { if (disabled) { selected.value = undefined; password.value = '' } })
function apply() {
  if (!selected.value || !valid.value || props.disabled) return
  const patch = compressionPurposePatch(selected.value, { password: reuse.value ? undefined : password.value, reusePassword: reuse.value && props.hasPassword, splitMiB: Number(splitMiB.value) })
  emit('apply', patch)
  message.value = `${props.staged ? '已填入，确认弹窗后生效' : '已应用，可继续微调'} · ${purpose.value!.parameters}${selected.value === 'split' ? ` · ${splitMiB.value} MiB` : ''}`
  selected.value = undefined; password.value = ''
}
</script>

<template>
  <section class="purpose-picker" data-testid="purpose-picker" aria-label="按用途快捷配置">
    <div class="purpose-heading"><span><i class="pi pi-sparkles" aria-hidden="true"></i> 用途方案</span><span class="text-muted">选择后确认应用</span></div>
    <div class="purpose-choices">
      <button v-for="item in compressionPurposes" :key="item.id" type="button" :data-testid="`purpose-${item.id}`" :disabled="disabled || item.id === 'split' && !allowSplit" :aria-pressed="selected === item.id" :title="item.parameters" @click="select(item.id)">
        <i :class="item.icon" aria-hidden="true"></i><span>{{ item.name }}</span>
      </button>
    </div>
    <div v-if="purpose" class="purpose-preview" data-testid="purpose-preview">
      <p class="font-semibold text-content">{{ purpose.parameters }}</p>
      <p class="text-muted">{{ purpose.hint }}</p>
      <p v-if="selected === 'share' || selected === 'split'" class="text-muted">关闭不适用的文件名加密；{{ selected === 'share' ? '密码、分卷大小与卷型保留。' : '已有密码保留。' }}</p>
      <p v-if="selected === 'smaller'" class="text-muted">密码、分卷和文件名加密沿用当前设置。</p>
      <div v-if="selected === 'encrypted'" class="purpose-inputs">
        <label v-if="hasPassword" class="purpose-reuse"><input v-model="reuse" type="checkbox" data-testid="purpose-reuse-password" />沿用已有新包密码</label>
        <label v-if="!reuse || !hasPassword" class="purpose-field">{{ staged ? '所选任务的新包密码' : '新包密码' }}<input v-model="password" data-testid="purpose-password" type="password" autocomplete="new-password" placeholder="填写后才能应用加密方案" /></label>
        <p class="text-muted">分卷保留{{ staged ? '各自设置；填写时会统一新包密码。' : '当前设置。' }}</p>
      </div>
      <label v-if="selected === 'split'" class="purpose-field">每卷大小（MiB）<input v-model="splitMiB" data-testid="purpose-split-size" type="number" min="1" step="1" inputmode="numeric" /><span v-if="!valid" class="text-amber-600">请输入大于等于 1 的整数。</span></label>
      <div class="purpose-footer"><span class="text-muted">名称、目录和其他参数保留{{ staged ? '各自设置' : '原值' }}；可继续微调。</span><button type="button" class="purpose-apply" data-testid="purpose-apply" :disabled="!valid || disabled" @click="apply">{{ staged ? '填入方案' : '应用方案' }}</button></div>
    </div>
    <p v-if="message" class="purpose-message text-primary" role="status" data-testid="purpose-message">{{ message }}</p>
  </section>
</template>

<style scoped>
.purpose-picker { min-width:0; border:1px solid var(--border-subtle); border-radius:.85rem; padding:.75rem; background:var(--bg-input); font-size:.75rem; }
.purpose-heading { display:flex; flex-wrap:wrap; justify-content:space-between; gap:.35rem .75rem; margin-bottom:.6rem; }
.purpose-heading>span:first-child { color:var(--text-base); font-weight:700; }
.purpose-choices { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:.4rem; }
.purpose-choices button { display:flex; align-items:center; justify-content:center; flex-wrap:wrap; gap:.35rem; min-width:0; min-height:2.2rem; padding:.45rem .3rem; border:1px solid var(--border-subtle); border-radius:.6rem; color:var(--text-secondary); transition:background .15s,border-color .15s; }
.purpose-choices button:hover,.purpose-choices button[aria-pressed=true] { border-color:var(--dynamic-accent); color:var(--dynamic-accent); background:color-mix(in srgb,var(--dynamic-accent) 8%,transparent); }
button:disabled { opacity:.4; cursor:not-allowed; }
button:focus-visible,input:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
.purpose-preview { margin-top:.65rem; padding-top:.65rem; border-top:1px solid var(--border-subtle); display:flex; flex-direction:column; gap:.4rem; line-height:1.6; overflow-wrap:anywhere; }
.purpose-inputs { display:flex; flex-direction:column; gap:.4rem; }
.purpose-field { display:flex; flex-direction:column; gap:.25rem; min-width:0; }
.purpose-field input { width:100%; min-width:0; padding:.5rem .65rem; border:1px solid var(--border-subtle); border-radius:.6rem; background:var(--bg-input); }
.purpose-reuse { display:flex; align-items:center; gap:.4rem; }
.purpose-footer { display:flex; align-items:center; flex-wrap:wrap; justify-content:space-between; gap:.5rem; margin-top:.2rem; }
.purpose-apply { flex-shrink:0; margin-left:auto; padding:.4rem .85rem; border-radius:.6rem; background:var(--dynamic-accent); color:white; font-weight:600; }
.purpose-message { margin-top:.5rem; line-height:1.5; overflow-wrap:anywhere; }
@container (max-width:460px) { .purpose-choices { grid-template-columns:repeat(2,minmax(0,1fr)); } }
.purpose-picker { container-type:inline-size; }
@media(max-width:800px) { .purpose-choices { grid-template-columns:repeat(2,minmax(0,1fr)); } }
</style>
