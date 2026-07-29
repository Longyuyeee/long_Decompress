<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { PasswordStrength, usePasswordStore } from '@/stores/password'
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
const showPassword = ref(false)

const form = reactive({
  name: '',
  password: '',
  notes: ''
})

const strengthFromScore = (score: number): PasswordStrength => {
  if (score >= 80) return PasswordStrength.VeryStrong
  if (score >= 60) return PasswordStrength.Strong
  if (score >= 40) return PasswordStrength.Medium
  if (score >= 20) return PasswordStrength.Weak
  return PasswordStrength.VeryWeak
}

watch(() => props.visible, (isOpening) => {
  if (isOpening) {
    if (props.entry) {
      Object.assign(form, {
        name: props.entry.name || '',
        password: props.entry.password || '',
        notes: props.entry.notes || ''
      })
    } else {
      Object.assign(form, {
        name: '',
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
    const now = new Date().toISOString()
    const assessment = await passwordStore.assessPasswordStrength(form.password)
    const strength = strengthFromScore(assessment.score)

    if (props.entry) {
      await passwordStore.updateEntry(props.entry.id, {
        name: form.name,
        password: form.password,
        notes: form.notes,
        strength,
        updated_at: now,
      })
    } else {
      await passwordStore.addEntry({
        name: form.name,
        password: form.password,
        notes: form.notes,
        username: '',
        url: '',
        category: 'Other',
        tags: [],
        strength,
        custom_fields: [],
      })
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
      <!-- 密码名称 -->
      <div class="space-y-1.5">
        <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('vault.column.name') }} *</label>
        <input v-model="form.name" type="text" :placeholder="appStore.t('vault.placeholder.name')" class="w-full bg-input border border-subtle rounded-xl px-4 py-2.5 text-xs text-content focus:border-primary transition-all shadow-sm">
      </div>

      <!-- 密码正文 -->
      <div class="space-y-1.5 relative">
        <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('vault.column.password') }} *</label>
        <div class="relative group">
          <input v-model="form.password" :type="showPassword ? 'text' : 'password'" :placeholder="appStore.t('vault.placeholder.password')" class="w-full bg-input border border-subtle rounded-xl px-4 py-2.5 text-xs text-primary font-mono font-bold focus:border-primary transition-all pr-12 shadow-sm">
          <button type="button" @click="showPassword = !showPassword" :aria-label="showPassword ? appStore.t('vault.action.hide') : appStore.t('vault.action.show')" class="absolute right-4 top-1/2 -translate-y-1/2 text-dim hover:text-primary transition-colors"><i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye' " class="text-xs"></i></button>
        </div>
      </div>

      <!-- 备注说明 -->
      <div class="space-y-1.5">
        <label class="text-xs font-black text-muted uppercase tracking-widest ml-1">{{ appStore.t('vault.column.notes') }}</label>
        <textarea v-model="form.notes" rows="2" :placeholder="appStore.t('vault.placeholder.notes')" class="w-full bg-input border border-subtle rounded-xl px-4 py-2.5 text-sm text-muted focus:border-primary transition-all resize-none shadow-sm"></textarea>
      </div>

      <!-- 交互按钮 -->
      <div class="pt-2 flex gap-2">
        <button @click="emit('update:visible', false)" class="flex-1 py-2.5 rounded-xl bg-input border border-subtle text-muted text-xs font-black uppercase hover:text-content transition-all tracking-widest">{{ appStore.t('vault.confirm.cancel') }}</button>
        <button data-testid="password-entry-save" @click="handleSave" :disabled="isSaving || !form.name || !form.password" class="flex-[2] py-2.5 rounded-xl bg-primary text-white text-xs font-black shadow-lg shadow-primary/20 hover:brightness-110 flex items-center justify-center gap-2 transition-all tracking-widest">
          <i v-if="isSaving" class="pi pi-spin pi-spinner text-xs"></i>
          <span>{{ isSaving ? appStore.t('form.sync') : (entry ? appStore.t('form.update') : appStore.t('form.commit')) }}</span>
        </button>
      </div>
    </div>
  </Modal>
</template>
