<template>
  <div class="password-manager">
    <!-- 头部：标题和操作按钮 -->
    <div class="mb-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-white">密码本管理</h1>
          <p class="text-gray-600 dark:text-gray-400 mt-1">管理您的压缩文件密码，提高解压效率</p>
        </div>
        <div class="flex space-x-3">
          <button
            @click="showAddPasswordModal = true"
            class="glass-button-primary px-4 py-2"
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-plus mr-2"></i>
            添加密码
          </button>
          <button
            @click="importPasswords"
            class="glass-button px-4 py-2"
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-upload mr-2"></i>
            导入
          </button>
          <button
            @click="exportPasswords"
            class="glass-button px-4 py-2"
            :disabled="passwordStore.isLoading"
          >
            <i class="pi pi-download mr-2"></i>
            导出
          </button>
        </div>
      </div>
    </div>

    <!-- 错误和成功消息 -->
    <div v-if="passwordStore.errorMessage" class="mb-4 p-4 rounded-lg bg-red-500/10 border border-red-500/20">
      <div class="flex items-center">
        <i class="pi pi-exclamation-triangle text-red-500 mr-2"></i>
        <span class="text-red-500">{{ passwordStore.errorMessage }}</span>
        <button @click="passwordStore.errorMessage = ''" class="ml-auto text-red-500 hover:text-red-700">
          <i class="pi pi-times"></i>
        </button>
      </div>
    </div>

    <!-- 列表展示区域 -->
    <div class="glass-card">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead>
            <tr class="border-b border-gray-200 dark:border-gray-700">
              <th class="py-3 px-4 text-left w-12">
                <input type="checkbox" v-model="isAllSelected" />
              </th>
              <th class="py-3 px-4 text-left">名称</th>
              <th class="py-3 px-4 text-left">用户/密码</th>
              <th class="py-3 px-4 text-left">强度</th>
              <th class="py-3 px-4 text-left">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="password in passwordStore.paginatedPasswords" :key="password.id" class="border-b border-gray-100 dark:border-gray-800">
              <td class="py-3 px-4">
                <input type="checkbox" :value="password.id" v-model="passwordStore.selectedPasswords" />
              </td>
              <td class="py-3 px-4 font-medium">{{ password.name }}</td>
              <td class="py-3 px-4">{{ password.username || '-' }} / ••••••••</td>
              <td class="py-3 px-4">
                <span :class="passwordStore.getStrengthTextColor(password.strength)">
                  {{ passwordStore.getStrengthLabel(password.strength) }}
                </span>
              </td>
              <td class="py-3 px-4">
                <button @click="editPassword(password)" class="text-blue-600 mr-2"><i class="pi pi-pencil"></i></button>
                <button @click="deletePassword(password.id)" class="text-red-600"><i class="pi pi-trash"></i></button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 分页控制 -->
    <div class="mt-4 flex justify-between items-center">
      <span class="text-sm text-gray-500">共 {{ passwordStore.filteredPasswords.length }} 条记录</span>
      <div class="flex space-x-2">
        <button @click="passwordStore.currentPage--" :disabled="passwordStore.currentPage === 1" class="glass-button px-3 py-1">上一页</button>
        <span class="px-3 py-1">{{ passwordStore.currentPage }} / {{ passwordStore.totalPages }}</span>
        <button @click="passwordStore.currentPage++" :disabled="passwordStore.currentPage === passwordStore.totalPages" class="glass-button px-3 py-1">下一页</button>
      </div>
    </div>

    <PasswordModal
      v-if="showAddPasswordModal"
      :password="editingPassword"
      @save="handleSavePassword"
      @close="closeModal"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { usePasswordStore, PasswordCategory } from '@/stores'
import type { PasswordEntry, AddPasswordRequest, UpdatePasswordRequest } from '@/stores'
import PasswordModal from './PasswordModal.vue'

const passwordStore = usePasswordStore()

const showAddPasswordModal = ref(false)
const editingPassword = ref<PasswordEntry | null>(null)

const isAllSelected = computed({
  get: () => passwordStore.isAllSelected,
  set: (val) => {
    if (val) passwordStore.selectedPasswords = passwordStore.filteredPasswords.map(p => p.id)
    else passwordStore.selectedPasswords = []
  }
})

const deletePassword = async (id: string) => {
  if (confirm('确定删除吗？')) await passwordStore.deletePassword(id)
}

const editPassword = (p: PasswordEntry) => {
  editingPassword.value = p
  showAddPasswordModal.value = true
}

const handleSavePassword = async (data: any) => {
  if (editingPassword.value) {
    await passwordStore.updatePassword(editingPassword.value.id, data)
  } else {
    await passwordStore.addPassword(data)
  }
  closeModal()
}

const closeModal = () => {
  showAddPasswordModal.value = false
  editingPassword.value = null
}

const importPasswords = () => alert('开发中')
const exportPasswords = () => alert('开发中')

onMounted(() => passwordStore.loadPasswords())
</script>
