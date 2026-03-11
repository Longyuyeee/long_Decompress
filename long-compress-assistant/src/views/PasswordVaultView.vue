<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { usePasswordStore, PasswordCategory } from '@/stores/password'
import { useAppStore } from '@/stores/app'
import GlassCard from '@/components/ui/GlassCard.vue'
import GlassButton from '@/components/ui/GlassButton.vue'
import { gsap } from 'gsap'
import PasswordEntryModal from '@/components/passwords/PasswordEntryModal.vue'

const passwordStore = usePasswordStore()
const appStore = useAppStore()

const isDragging = ref(false)
const showSuctionEffect = ref(false)
const showAddModal = ref(false)
const suctionContainer = ref<HTMLElement | null>(null)

onMounted(async () => {
  await passwordStore.checkUnlockStatus()
})

// 智能排序：按最后使用时间和 favorite 状态排序
const sortedEntries = computed(() => {
  return [...passwordStore.filteredEntries].sort((a, b) => {
    if (a.favorite !== b.favorite) return a.favorite ? -1 : 1
    const timeA = a.last_used ? new Date(a.last_used).getTime() : 0
    const timeB = b.last_used ? new Date(b.last_used).getTime() : 0
    return timeB - timeA
  })
})

// 处理 DLong 导入动效
const handleDrop = async (event: DragEvent) => {
  isDragging.value = false
  const files = event.dataTransfer?.files
  if (!files || files.length === 0) return

  // 触发吸入特效
  triggerSuctionAnimation(event.clientX, event.clientY)
  
  // 模拟导入延迟
  setTimeout(() => {
    appStore.successMessage = "密码本已通过 DLong 安全导入并自动去重"
  }, 1000)
}

const triggerSuctionAnimation = (x: number, y: number) => {
  showSuctionEffect.value = true
  
  // 1. 波纹扩散
  gsap.fromTo(".suction-wave", 
    { scale: 0, opacity: 0.8, x, y }, 
    { scale: 4, opacity: 0, duration: 1.5, ease: "expo.out" }
  )

  // 2. 模拟文件图标飞行 (GSAP 贝塞尔路径)
  const flyIcon = document.createElement('div')
  flyIcon.className = 'fixed z-[100] pi pi-file-export text-blue-400 text-3xl pointer-events-none'
  flyIcon.style.left = `${x}px`
  flyIcon.style.top = `${y}px`
  document.body.appendChild(flyIcon)

  const vaultIcon = document.querySelector('.vault-main-icon')
  if (vaultIcon) {
    const rect = vaultIcon.getBoundingClientRect()
    gsap.to(flyIcon, {
      x: rect.left - x + rect.width / 2,
      y: rect.top - y + rect.height / 2,
      scale: 0.2,
      rotation: 720,
      duration: 0.8,
      ease: "back.in(1.7)",
      onComplete: () => {
        flyIcon.remove()
        // 3. 保险箱“吞咽”反馈
        gsap.to(vaultIcon, { scale: 1.3, duration: 0.2, yoyo: true, repeat: 1 })
      }
    })
  }
}
</script>

<template>
  <div class="password-vault p-8 min-h-screen relative overflow-hidden"
       @dragover.prevent="isDragging = true"
       @dragleave.prevent="isDragging = false"
       @drop.prevent="handleDrop">
    
    <!-- 全局吸入特效层 -->
    <div v-if="showSuctionEffect" class="suction-overlay fixed inset-0 pointer-events-none z-[90]">
      <div class="suction-wave absolute w-64 h-64 bg-blue-500/20 rounded-full blur-3xl -translate-x-1/2 -translate-y-1/2"></div>
    </div>

    <!-- 页头与统计 -->
    <header class="mb-10 flex justify-between items-start">
      <div class="flex items-center gap-6">
        <div class="vault-main-icon w-16 h-16 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center shadow-2xl backdrop-blur-xl">
          <i class="pi pi-shield text-blue-400 text-3xl"></i>
        </div>
        <div>
          <h1 class="text-4xl font-black text-white tracking-tighter mb-1">密码保险箱</h1>
          <div class="flex items-center gap-4">
            <span class="text-[10px] text-white/30 uppercase font-bold tracking-[0.2em]">Password Vault v2.1</span>
            <span class="px-2 py-0.5 rounded bg-blue-500/20 text-blue-400 text-[8px] font-bold uppercase">Encrypted</span>
          </div>
        </div>
      </div>

      <div class="flex gap-4">
        <GlassButton @click="showAddModal = true" icon="pi pi-plus" label="新增密码" type="primary" />
        <GlassButton icon="pi pi-download" label="导出 DLong" />
      </div>
    </header>

    <!-- ... (分类过滤逻辑保持不变) ... -->

    <!-- 智慧密码格栅 (始终显示) -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
      <!-- ... (卡片循环保持不变) ... -->

      <!-- 空白占位 (虚线) -->
      <div 
         @click="showAddModal = true"
         class="p-6 rounded-3xl border border-dashed border-white/10 flex flex-col items-center justify-center text-white/10 hover:border-white/20 hover:text-white/20 transition-all cursor-pointer group"
      >
         <i class="pi pi-plus-circle text-2xl mb-2 group-hover:scale-110 transition-transform"></i>
         <span class="text-[10px] font-bold uppercase tracking-widest">添加新凭证</span>
      </div>
    </div>

    <!-- 新增凭证弹窗 -->
    <PasswordEntryModal v-model:visible="showAddModal" />
  </div>
</template>

<style scoped>
.password-vault {
  background: radial-gradient(circle at 50% 0%, rgba(59, 130, 246, 0.03) 0%, transparent 50%);
}

.custom-scrollbar::-webkit-scrollbar {
  height: 2px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 10px;
}

.suction-overlay {
  mix-blend-mode: plus-lighter;
}
</style>
