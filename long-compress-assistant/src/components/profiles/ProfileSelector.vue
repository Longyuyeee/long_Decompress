<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useCompressionProfileStore } from '@/stores/compressionProfile'
import { useAppStore } from '@/stores/app'
import type { CompressionProfile } from '@/types/profile'
import { extractErrorMessage } from '@/utils'

defineProps<{ showManageButton?: boolean }>()

const emit = defineEmits<{
  apply: [profile: CompressionProfile]
  manage: []
}>()

const profileStore = useCompressionProfileStore()
const appStore = useAppStore()
const applyingId = ref<string | null>(null)
const profiles = computed(() => profileStore.sortedProfiles)
const loading = computed(() => profileStore.loading)

onMounted(async () => {
  try {
    await profileStore.loadAllProfiles()
  } catch (error) {
    appStore.setError(extractErrorMessage(error))
  }
})

const formatLastUsed = (timestamp: number | null) => {
  if (!timestamp) return '从未使用'
  const days = Math.floor((Date.now() - timestamp) / 86_400_000)
  if (days <= 0) return '今天'
  if (days === 1) return '昨天'
  if (days < 30) return `${days} 天前`
  return new Date(timestamp).toLocaleDateString()
}

const applyProfile = (profile: CompressionProfile) => {
  applyingId.value = profile.id
  emit('apply', profile)
}
</script>

<template>
  <div class="profile-selector min-w-0">
    <div class="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-subtle bg-input/25 px-4 py-3">
      <div>
        <p class="text-sm font-black text-content">选择一个配置组</p>
        <p class="mt-0.5 text-xs text-muted">点击卡片后立即应用，当前任务中的设置会被替换</p>
      </div>
      <button v-if="showManageButton" type="button" class="h-9 rounded-lg border border-subtle bg-input px-4 text-xs font-black text-muted hover:border-primary/30 hover:text-content" @click="emit('manage')">
        <i class="pi pi-cog mr-2"></i>{{ appStore.t('profiles.manage') }}
      </button>
    </div>

    <div v-if="loading" class="flex min-h-48 items-center justify-center text-primary">
      <i class="pi pi-spin pi-spinner text-xl"></i>
    </div>

    <div v-else-if="profiles.length === 0" class="flex min-h-52 flex-col items-center justify-center rounded-2xl border border-dashed border-subtle bg-input/15 px-6 text-center">
      <div class="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-primary/10 text-2xl">📦</div>
      <p class="font-black text-content">还没有可用配置组</p>
      <p class="mt-1 text-xs text-muted">先创建一个配置组，再回来快速应用。</p>
      <button type="button" class="mt-4 h-9 rounded-lg bg-primary px-4 text-xs font-black text-white" @click="emit('manage')">创建配置组</button>
    </div>

    <div v-else class="grid grid-cols-1 gap-3 md:grid-cols-2">
      <button
        v-for="profile in profiles"
        :key="profile.id"
        type="button"
        class="group min-w-0 rounded-2xl border bg-card/60 p-4 text-left transition hover:-translate-y-0.5 hover:border-primary/40 hover:bg-card hover:shadow-lg hover:shadow-primary/5"
        :class="applyingId === profile.id ? 'border-primary ring-2 ring-primary/15' : 'border-subtle'"
        @click="applyProfile(profile)"
      >
        <div class="flex min-w-0 items-start gap-3">
          <div class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-subtle bg-input/40 text-2xl">{{ profile.icon }}</div>
          <div class="min-w-0 flex-1">
            <div class="flex items-center justify-between gap-2">
              <h4 class="truncate text-sm font-black text-content group-hover:text-primary">{{ profile.name }}</h4>
              <i class="pi pi-arrow-right shrink-0 text-xs text-muted opacity-0 transition group-hover:translate-x-0.5 group-hover:text-primary group-hover:opacity-100"></i>
            </div>
            <p class="mt-1 line-clamp-2 min-h-8 text-xs leading-4 text-muted">{{ profile.description || '未填写说明' }}</p>
          </div>
        </div>

        <div class="mt-3 flex flex-wrap gap-1.5 text-xs font-bold">
          <span class="rounded-md bg-primary/10 px-2 py-1 text-primary">{{ profile.config.format.toUpperCase() }}</span>
          <span class="rounded-md bg-input px-2 py-1 text-muted">等级 {{ profile.config.level }}</span>
          <span v-if="profile.config.password" class="rounded-md bg-amber-500/10 px-2 py-1 text-amber-400"><i class="pi pi-lock mr-1"></i>加密</span>
          <span v-if="profile.config.splitArchive" class="rounded-md bg-violet-500/10 px-2 py-1 text-violet-400">分卷</span>
          <span v-if="profile.config.createSolidArchive" class="rounded-md bg-emerald-500/10 px-2 py-1 text-emerald-400">固实</span>
        </div>

        <div class="mt-3 flex items-center justify-between border-t border-subtle/60 pt-3 text-xs text-muted">
          <span>使用 {{ profile.stats.useCount }} 次</span>
          <span>{{ formatLastUsed(profile.lastUsedAt) }}</span>
        </div>
      </button>
    </div>
  </div>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
}
</style>
