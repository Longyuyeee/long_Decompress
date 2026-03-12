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
  notes: '',
  username: '',
  category: PasswordCategory.Other,
  tags: [] as string[]
})

watch(() => props.visible, (isOpening) => {
  if (isOpening) {
    if (props.entry) {
      Object.assign(form, {
        name: props.entry.name || '',
        password: props.entry.password || '',
        notes: props.entry.notes || '',
        username: props.entry.username || '',
        category: props.entry.category || PasswordCategory.Other
      })
    } else {
      const randomId = Math.random().toString(36).substring(2, 6).toUpperCase()
      Object.assign(form, {
        name: `NEW_ENTRY_${randomId}`,
        password: '',
        notes: '',
        username: '',
        category: PasswordCategory.Other
      })
    }
  }
}, { immediate: true })

const handleSave = async () => {
  if (!form.name || !form.password) return
  isSaving.value = true
  try {
    if (props.entry) await passwordStore.updateEntry(props.entry.id, { ...form })
    else await passwordStore.addEntry({ ...form })
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
    :title="entry ? 'Edit Security Entry' : 'Create New Entry'"
    :icon="entry ? 'pi pi-pencil' : 'pi pi-shield'"
    size="md"
  >
    <div class="modal-content space-y-6 bg-modal text-content">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- 找回：凭证名称 -->
        <div class="space-y-2">
          <label class="text-[9px] font-black text-muted uppercase tracking-widest ml-1">Entry Name *</label>
          <input v-model="form.name" type="text" class="w-full bg-input border border-subtle rounded-xl px-4 py-3 text-sm text-content focus:border-primary transition-all">
        </div>
        <!-- 找回：关联账号 -->
        <div class="space-y-2">
          <label class="text-[9px] font-black text-muted uppercase tracking-widest ml-1">Account Identity</label>
          <input v-model="form.username" type="text" class="w-full bg-input border border-subtle rounded-xl px-4 py-3 text-sm text-content focus:border-primary transition-all" placeholder="Optional">
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- 找回：密码核心 -->
        <div class="space-y-2 relative">
          <label class="text-[9px] font-black text-muted uppercase tracking-widest ml-1">Access Secret *</label>
          <div class="relative">
            <input v-model="form.password" :type="showPassword ? 'text' : 'password'" class="w-full bg-input border border-subtle rounded-xl px-4 py-3 text-sm text-primary font-mono font-bold focus:border-primary transition-all pr-12">
            <button @click="showPassword = !showPassword" class="absolute right-4 top-1/2 -translate-y-1/2 text-dim hover:text-primary"><i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye' " class="text-xs"></i></button>
          </div>
        </div>
        <!-- 找回：所属分类 -->
        <div class="space-y-2">
          <label class="text-[9px] font-black text-muted uppercase tracking-widest ml-1">Classification</label>
          <select v-model="form.category" class="w-full bg-input border border-subtle rounded-xl px-4 py-3 text-sm text-content focus:border-primary transition-all appearance-none">
            <option v-for="cat in Object.values(PasswordCategory)" :key="cat" :value="cat">{{ cat }}</option>
          </select>
        </div>
      </div>

      <div class="space-y-2">
        <label class="text-[9px] font-black text-muted uppercase tracking-widest ml-1">Security Notes & Metadata</label>
        <textarea v-model="form.notes" rows="3" placeholder="Contextual information..." class="w-full bg-input border border-subtle rounded-xl px-4 py-3 text-xs text-muted focus:border-primary transition-all resize-none"></textarea>
      </div>

      <div class="pt-4 flex gap-3">
        <button @click="emit('update:visible', false)" class="flex-1 py-3 rounded-xl bg-input border border-subtle text-muted text-[10px] font-black uppercase hover:text-content">Abort</button>
        <button @click="handleSave" :disabled="isSaving || !form.name || !form.password" class="flex-[2] py-3 rounded-xl bg-primary text-white text-[10px] font-black shadow-lg shadow-primary/20 hover:brightness-110 flex items-center justify-center gap-2">
          <i v-if="isSaving" class="pi pi-spin pi-spinner"></i>
          <span>{{ isSaving ? 'SYNCHRONIZING' : 'COMMIT TO VAULT' }}</span>
        </button>
      </div>
    </div>
  </Modal>
</template>
