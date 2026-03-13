<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { PasswordCategory, usePasswordStore } from '@/stores/password'
import { useAppStore } from '@/stores/app'
import Modal from '@/components/ui/Modal.vue'

const props = defineProps<{
  visible: boolean
  entry?: any 
}>()

const emit = defineEmits(['update:visible', 'saved'])

const passwordStore = usePasswordStore()
const appStore = useAppStore()
const isSaving = ref(false)
const showPassword = ref(true)

const form = reactive({
  name: '',
  password: '',
  notes: ''
})

watch(() => props.visible, (isOpening) => {
  if (isOpening) {
    if (props.entry) {
      Object.assign(form, {
        name: props.entry.name || '',
        password: props.entry.password || '',
        notes: props.entry.notes || ''
      })
    } else {
      const randomId = Math.random().toString(36).substring(2, 6).toUpperCase()
      Object.assign(form, {
        name: `ENTRY_${randomId}`,
        password: '',
        notes: ''
      })
    }
  }
}, { immediate: true })

const handleSave = async () => {
  if (!form.name || !form.password) return
  isSaving.value = true
  try {
    // 彻底修复：补全后端全量模型字段
    const now = new Date().toISOString()
    const payload = {
      ...form,
      username: '',
      url: '',
      category: 'Other',
      tags: [],
      strength: 'Medium',
      favorite: false,
      use_count: 0,
      usage_history: {},
      custom_fields: [],
      created_at: now,
      updated_at: now
    }
    if (props.entry) {
      // 更新时保留原有的统计和 ID
      await passwordStore.updateEntry(props.entry.id, { 
        ...props.entry,
        ...payload,
        updated_at: now
      })
    } else {
      await passwordStore.addEntry(payload)
    }
    emit('saved')
    emit('update:visible', false)
  } catch (e) {
    console.error(e)
  } finally {
    isSaving.value = false
  }
}
</script>

<template>
  <Modal 
    :visible="visible" 
    @update:visible="val => emit('update:visible', val)"
    :title="entry ? appStore.t('vault.edit_title') : appStore.t('vault.add_title')"
    :icon="entry ? 'pi pi-pencil' : 'pi pi-shield'"
    size="sm"
  >
    <div class="modal-content space-y-4 bg-modal text-content p-1">
      <!-- 凭证名称 -->
      <div class="space-y-1.5">
        <label class="text-[9px] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('vault.column.name') }} *</label>
        <input v-model="form.name" type="text" :placeholder="appStore.t('vault.placeholder.name')" class="w-full bg-input border border-subtle rounded-xl px-4 py-2.5 text-xs text-content focus:border-primary transition-all shadow-sm">
      </div>

      <!-- 访问密码 -->
      <div class="space-y-1.5 relative">
        <label class="text-[9px] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('vault.column.password') }} *</label>
        <div class="relative group">
          <input v-model="form.password" :type="showPassword ? 'text' : 'password'" :placeholder="appStore.t('vault.placeholder.password')" class="w-full bg-input border border-subtle rounded-xl px-4 py-2.5 text-xs text-primary font-mono font-bold focus:border-primary transition-all pr-12 shadow-sm">
          <button @click="showPassword = !showPassword" class="absolute right-4 top-1/2 -translate-y-1/2 text-dim hover:text-primary transition-colors"><i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye' " class="text-xs"></i></button>
        </div>
      </div>

      <!-- 备注说明 -->
      <div class="space-y-1.5">
        <label class="text-[9px] font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('vault.column.notes') }}</label>
        <textarea v-model="form.notes" rows="2" :placeholder="appStore.t('vault.placeholder.notes')" class="w-full bg-input border border-subtle rounded-xl px-4 py-2.5 text-[10px] text-muted focus:border-primary transition-all resize-none shadow-sm"></textarea>
      </div>

      <!-- 交互按钮 -->
      <div class="pt-2 flex gap-2">
        <button @click="emit('update:visible', false)" class="flex-1 py-2.5 rounded-xl bg-input border border-subtle text-muted text-[9px] font-black uppercase hover:text-content transition-all tracking-widest">{{ appStore.t('vault.confirm.cancel') }}</button>
        <button @click="handleSave" :disabled="isSaving || !form.name || !form.password" class="flex-[2] py-2.5 rounded-xl bg-primary text-white text-[9px] font-black shadow-lg shadow-primary/20 hover:brightness-110 flex items-center justify-center gap-2 transition-all tracking-widest">
          <i v-if="isSaving" class="pi pi-spin pi-spinner text-[8px]"></i>
          <span>{{ isSaving ? 'SYNC' : (entry ? 'UPDATE' : 'COMMIT') }}</span>
        </button>
      </div>
    </div>
  </Modal>
</template>
