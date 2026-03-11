<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { PasswordCategory, usePasswordStore } from '@/stores/password'
import Modal from '@/components/ui/Modal.vue'

const props = defineProps<{
  visible: boolean,
  entry?: any // 新增可选属性用于编辑
}>()

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void
  (e: 'saved'): void
}>()

const passwordStore = usePasswordStore()
const isSaving = ref(false)
const showPassword = ref(false)

const getDefaultName = () => {
  const now = new Date()
  const month = now.getMonth() + 1
  const day = now.getDate()
  const hours = now.getHours().toString().padStart(2, '0')
  const minutes = now.getMinutes().toString().padStart(2, '0')
  return `解压凭证_${month}${day}_${hours}${minutes}`
}

const form = reactive({
  name: getDefaultName(),
  password: '',
  notes: ''
})

watch(() => props.visible, (newVal) => {
  if (newVal) {
    if (props.entry) {
      // 进入编辑模式
      form.name = props.entry.name
      form.password = props.entry.password
      form.notes = props.entry.notes || ''
    } else {
      // 进入新增模式
      resetForm()
    }
  }
})

const handleSave = async () => {
  if (!form.name || !form.password) return
  
  isSaving.value = true
  try {
    const payload = {
      name: form.name,
      password: form.password,
      category: PasswordCategory.Other,
      username: null,
      notes: form.notes || null,
      url: null,
      tags: [],
      custom_fields: []
    }

    if (props.entry?.id) {
      // 执行更新逻辑
      await passwordStore.updateEntry(props.entry.id, payload)
    } else {
      // 执行新增逻辑
      await passwordStore.addEntry(payload)
    }
    
    emit('saved')
    emit('update:visible', false)
  } catch (e) {
    console.error('Save failed', e)
  } finally {
    isSaving.value = false
  }
}

const resetForm = () => {
  form.name = getDefaultName()
  form.password = ''
  form.notes = ''
}
</script>

<template>
  <Modal 
    :visible="visible" 
    @update:visible="val => emit('update:visible', val)"
    title="添加新密码"
    icon="pi pi-plus-circle"
    size="md"
  >
    <div class="space-y-6 py-2">
      <!-- 基础信息组 -->
      <div class="space-y-5">
        <div class="group">
          <label class="block text-[10px] font-black text-white/30 uppercase tracking-[0.2em] mb-2 ml-1">名称</label>
          <input 
            v-model="form.name"
            type="text" 
            placeholder="例如：通用压缩解压密码"
            class="w-full bg-white/5 border border-white/10 rounded-2xl px-5 py-3.5 text-white text-sm focus:outline-none focus:border-blue-500/50 focus:bg-white/10 transition-all shadow-inner"
          >
        </div>

        <div class="group">
          <label class="block text-[10px] font-black text-white/30 uppercase tracking-[0.2em] mb-2 ml-1">密码</label>
          <div class="relative">
            <input 
              v-model="form.password"
              :type="showPassword ? 'text' : 'password'" 
              placeholder="请输入解压密码"
              class="w-full bg-white/5 border border-white/10 rounded-2xl px-5 py-3.5 text-white text-sm focus:outline-none focus:border-blue-500/50 transition-all font-mono tracking-wider"
            >
            <button @click="showPassword = !showPassword" class="absolute right-4 top-1/2 -translate-y-1/2 text-white/20 hover:text-white/60 transition-colors">
              <i :class="['pi', showPassword ? 'pi-eye-slash' : 'pi-eye']"></i>
            </button>
          </div>
        </div>

        <div class="group">
          <label class="block text-[10px] font-black text-white/30 uppercase tracking-[0.2em] mb-2 ml-1">备注</label>
          <textarea 
            v-model="form.notes"
            rows="3"
            placeholder="关于这个密码的一些额外说明..."
            class="w-full bg-white/5 border border-white/10 rounded-2xl px-5 py-3.5 text-white text-sm focus:outline-none focus:border-blue-500/50 transition-all resize-none"
          ></textarea>
        </div>
      </div>

      <!-- 动作按钮 -->
      <div class="flex gap-4 pt-4">
        <button 
          @click="emit('update:visible', false)"
          class="flex-1 py-4 rounded-2xl bg-white/5 border border-white/10 text-white/40 text-[10px] font-black uppercase tracking-[0.2em] hover:bg-white/10 hover:text-white transition-all"
        >
          取消
        </button>
        <button 
          @click="handleSave"
          :disabled="isSaving || !form.name || !form.password"
          class="flex-[2] py-4 rounded-2xl bg-blue-600 text-white text-[10px] font-black uppercase tracking-[0.2em] hover:bg-blue-500 hover:shadow-[0_10px_30px_rgba(37,99,235,0.4)] disabled:opacity-30 disabled:hover:shadow-none transition-all flex items-center justify-center gap-2"
        >
          <i v-if="isSaving" class="pi pi-spin pi-spinner text-xs"></i>
          <span>{{ isSaving ? '存储中...' : '安全存入' }}</span>
        </button>
      </div>
    </div>
  </Modal>
</template>

<style scoped>
input::placeholder, textarea::placeholder {
  color: rgba(255, 255, 255, 0.1);
}
</style>
