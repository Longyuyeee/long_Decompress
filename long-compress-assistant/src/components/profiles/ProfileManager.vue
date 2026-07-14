<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useCompressionProfileStore } from '@/stores/compressionProfile'
import { useAppStore } from '@/stores/app'
import type { CompressionProfile, CreateProfileRequest } from '@/types/profile'

const profileStore = useCompressionProfileStore()
const appStore = useAppStore()

const showCreateModal = ref(false)
const showEditModal = ref(false)
const showDeleteConfirm = ref(false)
const editingProfile = ref<CompressionProfile | null>(null)
const deletingProfileId = ref<string | null>(null)

const formData = ref<CreateProfileRequest>({
  name: '',
  icon: '📦',
  description: '',
  config: {
    format: 'zip',
    level: 6,
    password: null,
    splitArchive: false,
    splitSize: null,
    keepStructure: true,
    deleteAfter: false,
    createSolidArchive: false,
    filenameTemplate: null,
    extraParams: {}
  }
})

const profiles = computed(() => profileStore.sortedProfiles)
const loading = computed(() => profileStore.loading)

const iconOptions = ['📦', '🗜️', '📁', '🔐', '⚡', '🎯', '💼', '🎨', '🔧', '⭐']

onMounted(async () => {
  try {
    await profileStore.loadAllProfiles()
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
})

const openCreateModal = () => {
  resetForm()
  showCreateModal.value = true
}

const openEditModal = (profile: CompressionProfile) => {
  editingProfile.value = profile
  formData.value = {
    name: profile.name,
    icon: profile.icon,
    description: profile.description,
    config: { ...profile.config }
  }
  showEditModal.value = true
}

const openDeleteConfirm = (profileId: string) => {
  deletingProfileId.value = profileId
  showDeleteConfirm.value = true
}

const resetForm = () => {
  formData.value = {
    name: '',
    icon: '📦',
    description: '',
    config: {
      format: 'zip',
      level: 6,
      password: null,
      splitArchive: false,
      splitSize: null,
      keepStructure: true,
      deleteAfter: false,
      createSolidArchive: false,
      filenameTemplate: null,
      extraParams: {}
    }
  }
}

const handleCreate = async () => {
  try {
    await profileStore.addProfile(formData.value)
    showCreateModal.value = false
    appStore.setSuccess(appStore.t('common.success'))
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

const handleUpdate = async () => {
  if (!editingProfile.value) return

  try {
    const updatedProfile: CompressionProfile = {
      ...editingProfile.value,
      name: formData.value.name,
      icon: formData.value.icon,
      description: formData.value.description,
      config: formData.value.config
    }
    await profileStore.modifyProfile(updatedProfile)
    showEditModal.value = false
    editingProfile.value = null
    appStore.setSuccess(appStore.t('common.success'))
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

const handleDelete = async () => {
  if (!deletingProfileId.value) return

  try {
    await profileStore.removeProfile(deletingProfileId.value)
    showDeleteConfirm.value = false
    deletingProfileId.value = null
    appStore.setSuccess(appStore.t('common.success'))
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`
}

const formatDate = (timestamp: number | null): string => {
  if (!timestamp) return '--'
  return new Date(timestamp).toLocaleString()
}
</script>

<template>
  <div class="profile-manager">
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h2 class="text-2xl font-bold text-slate-100">{{ appStore.t('profiles.title') }}</h2>
        <p class="text-sm text-slate-400 mt-1">{{ appStore.t('profiles.description') }}</p>
      </div>
      <button
        @click="openCreateModal"
        class="px-4 py-2 bg-sky-500 hover:bg-sky-600 text-white rounded-lg transition-colors flex items-center gap-2"
      >
        <span>+</span>
        <span>{{ appStore.t('profiles.add_new') }}</span>
      </button>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex items-center justify-center py-20">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-sky-400"></div>
    </div>

    <!-- Empty State -->
    <div v-else-if="profiles.length === 0" class="text-center py-20">
      <div class="text-6xl mb-4">📦</div>
      <p class="text-slate-400 mb-6">{{ appStore.t('profiles.empty') }}</p>
      <button
        @click="openCreateModal"
        class="px-6 py-3 bg-sky-500/20 hover:bg-sky-500/30 text-sky-400 rounded-lg transition-colors"
      >
        {{ appStore.t('profiles.add_new') }}
      </button>
    </div>

    <!-- Profile List -->
    <div v-else class="space-y-4">
      <div
        v-for="profile in profiles"
        :key="profile.id"
        class="bg-slate-800/40 backdrop-blur-sm border border-slate-700/50 rounded-xl p-6 hover:bg-slate-700/40 hover:border-slate-600/50 transition-all"
      >
        <div class="flex items-start justify-between">
          <!-- Profile Info -->
          <div class="flex items-start gap-4 flex-1">
            <div class="text-4xl">{{ profile.icon }}</div>
            <div class="flex-1">
              <h3 class="text-lg font-semibold text-slate-100 mb-1">{{ profile.name }}</h3>
              <p class="text-sm text-slate-400 mb-4">{{ profile.description }}</p>

              <!-- Config Details -->
              <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
                <div class="text-xs">
                  <span class="text-slate-500">{{ appStore.t('profiles.details_format') }}</span>
                  <span class="ml-2 text-slate-200 font-medium">{{ profile.config.format.toUpperCase() }}</span>
                </div>
                <div class="text-xs">
                  <span class="text-slate-500">{{ appStore.t('profiles.details_level') }}</span>
                  <span class="ml-2 text-slate-200 font-medium">{{ profile.config.level }}</span>
                </div>
                <div class="text-xs">
                  <span class="text-slate-500">{{ appStore.t('profiles.details_solid') }}</span>
                  <span class="ml-2 text-slate-200">{{ profile.config.createSolidArchive ? appStore.t('profiles.details_yes') : appStore.t('profiles.details_no') }}</span>
                </div>
                <div class="text-xs">
                  <span class="text-slate-500">{{ appStore.t('profiles.details_split') }}</span>
                  <span class="ml-2 text-slate-200">{{ profile.config.splitArchive ? appStore.t('profiles.details_yes') : appStore.t('profiles.details_no') }}</span>
                </div>
              </div>

              <!-- Stats -->
              <div class="flex flex-wrap gap-3 text-xs">
                <div class="px-3 py-1.5 bg-slate-700/50 rounded-lg">
                  <span class="text-slate-400">{{ appStore.t('profiles.use_count') }}:</span>
                  <span class="ml-2 text-sky-400 font-medium">{{ profile.stats.useCount }}</span>
                </div>
                <div class="px-3 py-1.5 bg-slate-700/50 rounded-lg">
                  <span class="text-slate-400">{{ appStore.t('profiles.stats_success') }}</span>
                  <span class="ml-2 text-green-400 font-medium">{{ profile.stats.successCount }}</span>
                </div>
                <div class="px-3 py-1.5 bg-slate-700/50 rounded-lg">
                  <span class="text-slate-400">{{ appStore.t('profiles.stats_failed') }}</span>
                  <span class="ml-2 text-red-400 font-medium">{{ profile.stats.failureCount }}</span>
                </div>
                <div class="px-3 py-1.5 bg-slate-700/50 rounded-lg">
                  <span class="text-slate-400">{{ appStore.t('profiles.stats_processed') }}</span>
                  <span class="ml-2 text-slate-200 font-medium">{{ formatBytes(profile.stats.totalBytesProcessed) }}</span>
                </div>
                <div class="px-3 py-1.5 bg-slate-700/50 rounded-lg">
                  <span class="text-slate-400">{{ appStore.t('profiles.last_used') }}:</span>
                  <span class="ml-2 text-slate-200">{{ formatDate(profile.lastUsedAt) }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex gap-2">
            <button
              @click="openEditModal(profile)"
              class="px-4 py-2 bg-slate-700/50 hover:bg-slate-600/50 text-slate-200 rounded-lg transition-colors text-sm"
            >
              {{ appStore.t('profiles.edit') }}
            </button>
            <button
              @click="openDeleteConfirm(profile.id)"
              class="px-4 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-400 rounded-lg transition-colors text-sm"
            >
              {{ appStore.t('profiles.delete') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Create/Edit Modal -->
    <div
      v-if="showCreateModal || showEditModal"
      class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50"
      @click.self="showCreateModal = false; showEditModal = false"
    >
      <div class="bg-slate-800 border border-slate-700 rounded-xl p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
        <h3 class="text-xl font-semibold text-slate-100 mb-6">
          {{ showCreateModal ? appStore.t('profiles.add_new') : appStore.t('profiles.edit') }}
        </h3>

        <div class="space-y-4">
          <!-- Icon Selection -->
          <div>
            <label class="block text-sm text-slate-300 mb-2">{{ appStore.t('profiles.icon_label') }}</label>
            <div class="flex gap-2 flex-wrap">
              <button
                v-for="icon in iconOptions"
                :key="icon"
                @click="formData.icon = icon"
                class="w-12 h-12 text-2xl rounded-lg transition-all"
                :class="formData.icon === icon ? 'bg-sky-500/20 ring-2 ring-sky-500' : 'bg-slate-700/50 hover:bg-slate-600/50'"
              >
                {{ icon }}
              </button>
            </div>
          </div>

          <!-- Name -->
          <div>
            <label class="block text-sm text-slate-300 mb-2">{{ appStore.t('profiles.name_required_mark') }}</label>
            <input
              v-model="formData.name"
              type="text"
              :placeholder="appStore.t('profiles.name_placeholder')"
              class="w-full px-4 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-slate-100 placeholder-slate-500 focus:outline-none focus:border-sky-500"
            />
          </div>

          <!-- Description -->
          <div>
            <label class="block text-sm text-slate-300 mb-2">{{ appStore.t('profiles.desc_label') }}</label>
            <textarea
              v-model="formData.description"
              rows="2"
              :placeholder="appStore.t('profiles.desc_placeholder')"
              class="w-full px-4 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-slate-100 placeholder-slate-500 focus:outline-none focus:border-sky-500"
            ></textarea>
          </div>

          <!-- Format -->
          <div>
            <label class="block text-sm text-slate-300 mb-2">{{ appStore.t('profiles.format_label') }}</label>
            <select
              v-model="formData.config.format"
              class="w-full px-4 py-2 bg-slate-700/50 border border-slate-600 rounded-lg text-slate-100 focus:outline-none focus:border-sky-500"
            >
              <option value="zip">ZIP</option>
              <option value="7z">7Z</option>
              <option value="tar">TAR</option>
              <option value="tar.gz">TAR.GZ</option>
              <option value="tar.bz2">TAR.BZ2</option>
              <option value="tar.xz">TAR.XZ</option>
            </select>
          </div>

          <!-- Compression Level -->
          <div>
            <label class="block text-sm text-slate-300 mb-2">{{ appStore.t('profiles.level_label') }}: {{ formData.config.level }}</label>
            <input
              v-model.number="formData.config.level"
              type="range"
              min="0"
              max="9"
              class="w-full"
            />
            <div class="flex justify-between text-xs text-slate-500 mt-1">
              <span>{{ appStore.t('profiles.level_range_min') }}</span>
              <span>{{ appStore.t('profiles.level_range_max') }}</span>
            </div>
          </div>

          <!-- Options -->
          <div class="space-y-2">
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                v-model="formData.config.createSolidArchive"
                type="checkbox"
                class="w-4 h-4 rounded border-slate-600 bg-slate-700 text-sky-500 focus:ring-sky-500"
              />
              <span class="text-sm text-slate-300">{{ appStore.t('profiles.solid_archive') }}</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                v-model="formData.config.splitArchive"
                type="checkbox"
                class="w-4 h-4 rounded border-slate-600 bg-slate-700 text-sky-500 focus:ring-sky-500"
              />
              <span class="text-sm text-slate-300">{{ appStore.t('profiles.split_label') }}</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                v-model="formData.config.keepStructure"
                type="checkbox"
                class="w-4 h-4 rounded border-slate-600 bg-slate-700 text-sky-500 focus:ring-sky-500"
              />
              <span class="text-sm text-slate-300">{{ appStore.t('profiles.keep_structure_label') }}</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                v-model="formData.config.deleteAfter"
                type="checkbox"
                class="w-4 h-4 rounded border-slate-600 bg-slate-700 text-sky-500 focus:ring-sky-500"
              />
              <span class="text-sm text-slate-300">{{ appStore.t('profiles.delete_after_label') }}</span>
            </label>
          </div>
        </div>

        <!-- Modal Actions -->
        <div class="flex gap-3 mt-6">
          <button
            @click="showCreateModal ? handleCreate() : handleUpdate()"
            class="flex-1 px-4 py-2 bg-sky-500 hover:bg-sky-600 text-white rounded-lg transition-colors"
          >
            {{ showCreateModal ? appStore.t('common.add') : appStore.t('profiles.save') }}
          </button>
          <button
            @click="showCreateModal = false; showEditModal = false"
            class="px-4 py-2 bg-slate-700/50 hover:bg-slate-600/50 text-slate-300 rounded-lg transition-colors"
          >
            {{ appStore.t('common.cancel') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <div
      v-if="showDeleteConfirm"
      class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50"
      @click.self="showDeleteConfirm = false"
    >
      <div class="bg-slate-800 border border-slate-700 rounded-xl p-6 max-w-md w-full mx-4">
        <h3 class="text-xl font-semibold text-slate-100 mb-4">{{ appStore.t('profiles.confirm_delete') }}</h3>
        <p class="text-slate-400 mb-6">{{ appStore.t('profiles.confirm_delete_desc') }}</p>
        <div class="flex gap-3">
          <button
            @click="handleDelete"
            class="flex-1 px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded-lg transition-colors"
          >
            {{ appStore.t('common.delete') }}
          </button>
          <button
            @click="showDeleteConfirm = false"
            class="px-4 py-2 bg-slate-700/50 hover:bg-slate-600/50 text-slate-300 rounded-lg transition-colors"
          >
            {{ appStore.t('common.cancel') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
input[type="range"] {
  -webkit-appearance: none;
  appearance: none;
  background: transparent;
  cursor: pointer;
}

input[type="range"]::-webkit-slider-track {
  background-color: rgb(51 65 85 / 0.5);
  border-radius: 0.5rem;
  height: 0.5rem;
}

input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  margin-top: -4px;
  background-color: #38bdf8;
  border-radius: 50%;
  height: 1rem;
  width: 1rem;
  transition: all 0.2s;
}

input[type="range"]::-webkit-slider-thumb:hover {
  background-color: #0ea5e9;
  transform: scale(1.2);
}

input[type="range"]::-moz-range-track {
  background-color: rgb(51 65 85 / 0.5);
  border-radius: 0.5rem;
  height: 0.5rem;
}

input[type="range"]::-moz-range-thumb {
  background-color: #38bdf8;
  border-radius: 50%;
  height: 1rem;
  width: 1rem;
  border: none;
  transition: all 0.2s;
}

input[type="range"]::-moz-range-thumb:hover {
  background-color: #0ea5e9;
  transform: scale(1.2);
}
</style>
