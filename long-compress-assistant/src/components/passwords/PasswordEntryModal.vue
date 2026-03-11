<script setup lang="ts">
import { ref, reactive } from 'vue'
import { PasswordCategory, usePasswordStore } from '@/stores/password'
import Modal from '@/components/ui/Modal.vue'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void
  (e: 'saved'): void
}>()

const passwordStore = usePasswordStore()
const isSaving = ref(false)
const showPassword = ref(false)

const form = reactive({
  name: '',
  password: '',
  category: PasswordCategory.Other,
  username: '',
  notes: ''
})

const categories = Object.values(PasswordCategory)

const handleSave = async () => {
  if (!form.name || !form.password) return
  
  isSaving.value = true
  try {
    await passwordStore.addEntry({
      name: form.name,
      password: form.password,
      category: form.category,
      username: form.username || null,
      notes: form.notes || null,
      url: null,
      tags: [],
      custom_fields: []
    })
    emit('saved')
    emit('update:visible', false)
    resetForm()
  } catch (e) {
    console.error('Save failed', e)
  } finally {
    isSaving.value = false
  }
}

const resetForm = () => {
  form.name = ''
  form.password = ''
  form.category = PasswordCategory.Other
  form.username = ''
  form.notes = ''
}
</script>

<template>
  <Modal 
    :visible="visible" 
    @update:visible="val => emit('update:visible', val)"
    title="新增凭证"
    icon="pi pi-shield"
    size="md"
  >
    <div class="space-y-6 py-2">
      <!-- 基础信息组 -->
      <div class="space-y-4">
        <div class="group">
          <label class="block text-[10px] font-black text-white/30 uppercase tracking-[0.2em] mb-2 ml-1">名称</label>
          <input 
            v-model="form.name"
            type="text" 
            placeholder="例如：我的解压通用密码"
            class="w-full bg-white/5 border border-white/10 rounded-2xl px-5 py-3.5 text-white text-sm focus:outline-none focus:border-blue-500/50 focus:bg-white/10 transition-all shadow-inner"
          >
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div class="group">
            <label class="block text-[10px] font-black text-white/30 uppercase tracking-[0.2em] mb-2 ml-1">用户名 (可选)</label>
            <input 
              v-model="form.username"
              type="text" 
              placeholder="Admin"
              class="w-full bg-white/5 border border-white/10 rounded-2xl px-5 py-3.5 text-white text-sm focus:outline-none focus:border-blue-500/50 transition-all"
            >
          </div>
          <div class="group">
            <label class="block text-[10px] font-black text-white/30 uppercase tracking-[0.2em] mb-2 ml-1">密码</label>
            <div class="relative">
              <input 
                v-model="form.password"
                :type="showPassword ? 'text' : 'password'" 
                placeholder="••••••••"
                class="w-full bg-white/5 border border-white/10 rounded-2xl px-5 py-3.5 text-white text-sm focus:outline-none focus:border-blue-500/50 transition-all font-mono tracking-wider"
              >
              <button @click="showPassword = !showPassword" class="absolute right-4 top-1/2 -translate-y-1/2 text-white/20 hover:text-white/60 transition-colors">
                <i :class="['pi', showPassword ? 'pi-eye-slash' : 'pi-eye']"></i>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 分类选择 -->
      <div class="space-y-3">
        <label class="block text-[10px] font-black text-white/30 uppercase tracking-[0.2em] ml-1">分类</label>
        <div class="flex flex-wrap gap-2">
          <button 
            v-for="cat in categories" :key="cat"
            @click="form.category = cat"
            class="px-4 py-1.5 rounded-full border text-[9px] font-bold uppercase tracking-widest transition-all"
            :class="form.category === cat 
              ? 'bg-blue-500/30 border-blue-500/50 text-white shadow-[0_0_15px_rgba(59,130,246,0.3)]' 
              : 'bg-white/5 border-white/10 text-white/30 hover:border-white/20 hover:text-white/60'"
          >
            {{ cat }}
          </button>
        </div>
      </div>

      <!-- 备注 -->
      <div class="group">
        <label class="block text-[10px] font-black text-white/30 uppercase tracking-[0.2em] mb-2 ml-1">备注</label>
        <textarea 
          v-model="form.notes"
          rows="3"
          placeholder="添加一些额外说明..."
          class="w-full bg-white/5 border border-white/10 rounded-2xl px-5 py-3.5 text-white text-sm focus:outline-none focus:border-blue-500/50 transition-all resize-none"
        ></textarea>
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
  color: rgba(255, 255, 255, 0.15);
}
</style>
