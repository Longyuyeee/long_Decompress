<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { PasswordCategory, usePasswordStore } from '@/stores/password'
import Modal from '@/components/ui/Modal.vue'

const props = defineProps<{
  visible: boolean
  entry?: any 
}>()

const emit = defineEmits(['update:visible', 'saved'])

const passwordStore = usePasswordStore()
const isSaving = ref(false)
const showPassword = ref(true)

const form = reactive({
  name: '',
  password: '',
  notes: '',
  username: '',
  category: PasswordCategory.Other,
  tags: [] as string[],
  custom_fields: []
})

// 监听打开状态，确保重置
watch(() => props.visible, (isOpening) => {
  if (isOpening) {
    if (props.entry) {
      // 编辑模式：填充数据
      Object.assign(form, {
        name: props.entry.name || '',
        password: props.entry.password || '',
        notes: props.entry.notes || '',
        username: props.entry.username || '',
        category: props.entry.category || PasswordCategory.Other
      })
    } else {
      // 新增模式：彻底重置并给默认名
      const randomId = Math.random().toString(36).substring(2, 6).toUpperCase()
      Object.assign(form, {
        name: `新建凭证_${randomId}`,
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
    if (props.entry) {
      await passwordStore.updateEntry(props.entry.id, { ...form })
    } else {
      await passwordStore.addEntry({ ...form })
    }
    emit('saved')
    emit('update:visible', false)
  } catch (e) {
    console.error('Save failed', e)
  } finally {
    isSaving.value = false
  }
}
</script>

<template>
  <Modal 
    :visible="visible" 
    @update:visible="val => emit('update:visible', val)"
    :title="entry ? '编辑凭证' : '添加密码'"
    :icon="entry ? 'pi pi-pencil' : 'pi pi-plus-circle'"
    size="sm"
  >
    <div class="modal-content-compact space-y-4 p-1">
      <div class="space-y-1.5">
        <label class="text-[9px] font-black text-white/20 uppercase tracking-widest ml-1">凭证名称 *</label>
        <input 
          v-model="form.name"
          type="text" 
          class="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-2.5 text-sm text-white focus:outline-none focus:border-blue-500/50 transition-all"
        >
      </div>

      <div class="space-y-1.5 relative">
        <label class="text-[9px] font-black text-white/20 uppercase tracking-widest ml-1">解压密码 *</label>
        <div class="relative">
          <input 
            v-model="form.password"
            :type="showPassword ? 'text' : 'password'" 
            placeholder="内容"
            class="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-2.5 text-sm text-blue-400 font-mono focus:outline-none focus:border-blue-500/50 transition-all pr-12"
          >
          <button @click="showPassword = !showPassword" class="absolute right-4 top-1/2 -translate-y-1/2 text-white/20 hover:text-white/60 transition-colors">
            <i :class="showPassword ? 'pi pi-eye-slash' : 'pi pi-eye' " class="text-xs"></i>
          </button>
        </div>
      </div>

      <div class="space-y-1.5">
        <label class="text-[9px] font-black text-white/20 uppercase tracking-widest ml-1">详细备注</label>
        <textarea 
          v-model="form.notes"
          rows="2"
          placeholder="补充信息..."
          class="w-full bg-white/5 border border-white/10 rounded-xl px-4 py-2.5 text-xs text-white/60 focus:outline-none focus:border-blue-500/50 transition-all resize-none"
        ></textarea>
      </div>

      <div class="pt-2 flex gap-2">
        <button @click="emit('update:visible', false)" class="flex-1 py-2.5 rounded-xl bg-white/5 text-white/30 text-[10px] font-bold hover:bg-white/10 transition-all">取消</button>
        <button 
          @click="handleSave"
          :disabled="isSaving || !form.name || !form.password"
          class="flex-[2] py-2.5 rounded-xl bg-blue-500 hover:bg-blue-600 disabled:opacity-50 text-white text-[10px] font-black shadow-lg shadow-blue-500/10 transition-all flex items-center justify-center gap-2"
        >
          <i v-if="isSaving" class="pi pi-spin pi-spinner text-[10px]"></i>
          <span>{{ isSaving ? '同步中' : (entry ? '保存修改' : '存入列表') }}</span>
        </button>
      </div>
    </div>
  </Modal>
</template>

<style scoped>
/* 强制适配更窄的弹窗 */
.modal-content-compact {
  max-width: 320px;
  margin: 0 auto;
}
</style>
