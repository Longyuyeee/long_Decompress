<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useCompressionProfileStore } from '@/stores/compressionProfile'
import { useAppStore } from '@/stores/app'
import type { CompressionProfile } from '@/types/profile'

const profileStore = useCompressionProfileStore()
const appStore = useAppStore()

const props = defineProps<{
  showManageButton?: boolean
}>()

const emit = defineEmits<{
  apply: [profile: CompressionProfile]
  manage: []
}>()

const hoveredProfileId = ref<string | null>(null)
const selectedProfileId = ref<string | null>(null)

const profiles = computed(() => profileStore.sortedProfiles)
const loading = computed(() => profileStore.loading)

onMounted(async () => {
  try {
    await profileStore.loadAllProfiles()
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
})

const formatLastUsed = (timestamp: number | null): string => {
  if (!timestamp) return appStore.t('profiles.last_used') + ': --'
  const date = new Date(timestamp)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24))

  if (diffDays === 0) return appStore.t('profiles.last_used') + ': ' + '今天'
  if (diffDays === 1) return appStore.t('profiles.last_used') + ': ' + '昨天'
  if (diffDays < 7) return appStore.t('profiles.last_used') + ': ' + `${diffDays}天前`
  if (diffDays < 30) return appStore.t('profiles.last_used') + ': ' + `${Math.floor(diffDays / 7)}周前`
  return appStore.t('profiles.last_used') + ': ' + `${Math.floor(diffDays / 30)}月前`
}

const formatSuccessRate = (stats: CompressionProfile['stats']): string => {
  if (stats.useCount === 0) return '0%'
  const rate = (stats.successCount / stats.useCount) * 100
  return `${Math.round(rate)}%`
}

const applyProfile = (profile: CompressionProfile) => {
  selectedProfileId.value = profile.id
  emit('apply', profile)
}

const openManage = () => {
  emit('manage')
}
</script>

<template>
  <div class="profile-selector">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <h3 class="text-lg font-semibold text-slate-100">
        {{ appStore.t('profiles.title') }}
      </h3>
      <button
        v-if="showManageButton"
        @click="openManage"
        class="px-4 py-2 text-sm bg-slate-700/50 hover:bg-slate-600/50 rounded-lg transition-colors"
      >
        管理配置组
      </button>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-sky-400"></div>
    </div>

    <!-- Empty State -->
    <div v-else-if="profiles.length === 0" class="text-center py-12">
      <p class="text-slate-400 mb-4">{{ appStore.t('profiles.empty') }}</p>
      <button
        @click="openManage"
        class="px-4 py-2 bg-sky-500/20 hover:bg-sky-500/30 text-sky-400 rounded-lg transition-colors"
      >
        {{ appStore.t('profiles.add_new') }}
      </button>
    </div>

    <!-- Profile Grid -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <div
        v-for="profile in profiles"
        :key="profile.id"
        @mouseenter="hoveredProfileId = profile.id"
        @mouseleave="hoveredProfileId = null"
        @click="applyProfile(profile)"
        class="profile-card group cursor-pointer"
        :class="{ 'profile-card-selected': selectedProfileId === profile.id }"
      >
        <!-- Card Content -->
        <div class="relative p-4 bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl transition-all duration-200 hover:bg-slate-700/40 hover:border-slate-600/50 hover:shadow-lg hover:shadow-sky-500/10 hover:-translate-y-1">
          <!-- Icon & Name -->
          <div class="flex items-start gap-3 mb-3">
            <div class="text-3xl">{{ profile.icon }}</div>
            <div class="flex-1">
              <h4 class="font-medium text-slate-100 group-hover:text-sky-400 transition-colors">
                {{ profile.name }}
              </h4>
              <p class="text-xs text-slate-400 mt-1 line-clamp-2">
                {{ profile.description }}
              </p>
            </div>
          </div>

          <!-- Stats Badges -->
          <div class="flex flex-wrap gap-2 mb-3">
            <span class="px-2 py-1 text-xs bg-slate-700/50 rounded text-slate-300">
              {{ profile.config.format.toUpperCase() }} · L{{ profile.config.level }}
            </span>
            <span
              v-if="profile.stats.useCount > 0"
              class="px-2 py-1 text-xs bg-sky-500/10 text-sky-400 rounded"
            >
              {{ appStore.t('profiles.use_count') }}: {{ profile.stats.useCount }}
            </span>
          </div>

          <!-- Statistics -->
          <div class="flex items-center justify-between text-xs text-slate-400">
            <span>{{ appStore.t('profiles.success_rate') }}: {{ formatSuccessRate(profile.stats) }}</span>
            <span>{{ formatLastUsed(profile.lastUsedAt) }}</span>
          </div>

          <!-- Hover Preview Tooltip -->
          <div
            v-if="hoveredProfileId === profile.id"
            class="absolute left-0 right-0 top-full mt-2 p-3 bg-slate-800/95 backdrop-blur-md border border-slate-600/50 rounded-lg shadow-xl z-10 text-xs space-y-2"
          >
            <div class="grid grid-cols-2 gap-2">
              <div>
                <span class="text-slate-400">格式:</span>
                <span class="ml-2 text-slate-200">{{ profile.config.format }}</span>
              </div>
              <div>
                <span class="text-slate-400">压缩级别:</span>
                <span class="ml-2 text-slate-200">{{ profile.config.level }}</span>
              </div>
              <div>
                <span class="text-slate-400">固实归档:</span>
                <span class="ml-2 text-slate-200">{{ profile.config.createSolidArchive ? '是' : '否' }}</span>
              </div>
              <div>
                <span class="text-slate-400">分卷:</span>
                <span class="ml-2 text-slate-200">{{ profile.config.splitArchive ? '是' : '否' }}</span>
              </div>
            </div>
            <div v-if="profile.autoApply.enabled" class="pt-2 border-t border-slate-700">
              <span class="text-sky-400">🎯 自动应用: {{ profile.autoApply.mode }}</span>
            </div>
          </div>

          <!-- Selected Indicator -->
          <div
            v-if="selectedProfileId === profile.id"
            class="absolute -top-2 -right-2 w-6 h-6 bg-sky-500 rounded-full flex items-center justify-center shadow-lg"
          >
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.profile-card-selected {
  animation: pulse-glow 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

@keyframes pulse-glow {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.8;
  }
}

.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
