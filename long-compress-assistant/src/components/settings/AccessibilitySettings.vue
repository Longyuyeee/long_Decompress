<template>
  <div class="accessibility-settings space-y-8">
    <!-- 字体大小 -->
    <section class="aero-card p-10">
      <h3 class="text-sm font-black text-content uppercase tracking-[0.3em] mb-6">
        {{ appStore.t('accessibility.fontSize') }}
      </h3>
      <div class="grid grid-cols-3 gap-4">
        <button
          v-for="size in fontSizes"
          :key="size.value"
          @click="updateAccessibility({ fontSize: size.value as 'normal' | 'large' | 'x-large' })"
          class="p-6 rounded-2xl border-2 transition-all text-center hover:scale-[1.02]"
          :class="appStore.settings.accessibility?.fontSize === size.value
            ? 'bg-primary/10 border-primary shadow-lg'
            : 'bg-input/30 border-subtle hover:border-primary/50'"
        >
          <div class="text-2xl mb-2">{{ size.icon }}</div>
          <div class="text-xs font-black text-content uppercase tracking-widest">{{ appStore.t(size.label) }}</div>
          <div class="text-[0.5625rem] text-muted mt-2">{{ size.example }}</div>
        </button>
      </div>
    </section>

    <!-- 高对比度 -->
    <section class="aero-card p-10">
      <div class="flex items-center justify-between mb-6">
        <div>
          <h3 class="text-sm font-black text-content uppercase tracking-[0.3em]">
            {{ appStore.t('accessibility.highContrast') }}
          </h3>
          <p class="text-[0.5625rem] text-muted mt-2 uppercase tracking-tighter">
            {{ appStore.t('accessibility.highContrast.desc') }}
          </p>
        </div>
        <button type="button" role="switch"
          :aria-checked="appStore.settings.accessibility?.highContrast"
          :aria-label="appStore.t('accessibility.highContrast')"
          @click="updateAccessibility({ highContrast: !appStore.settings.accessibility?.highContrast })"
          class="w-12 h-6 rounded-full border border-subtle p-0.5 transition-all cursor-pointer"
          :class="appStore.settings.accessibility?.highContrast ? 'bg-primary/40 border-primary' : 'bg-input'"
        >
          <div
            class="w-5 h-5 rounded-full bg-white shadow-sm transition-all"
            :class="appStore.settings.accessibility?.highContrast ? 'translate-x-6' : ''"
          ></div>
        </button>
      </div>
    </section>

    <!-- 色盲模式 -->
    <section class="aero-card p-10">
      <h3 class="text-sm font-black text-content uppercase tracking-[0.3em] mb-6">
        {{ appStore.t('accessibility.colorBlind') }}
      </h3>
      <div class="grid grid-cols-2 gap-4">
        <button
          v-for="mode in colorBlindModes"
          :key="mode.value"
          @click="updateAccessibility({ colorBlindMode: mode.value as 'none' | 'protanopia' | 'deuteranopia' | 'tritanopia' })"
          class="p-6 rounded-2xl border-2 transition-all text-left hover:scale-[1.02]"
          :class="appStore.settings.accessibility?.colorBlindMode === mode.value
            ? 'bg-primary/10 border-primary shadow-lg'
            : 'bg-input/30 border-subtle hover:border-primary/50'"
        >
          <div class="text-2xl mb-2">{{ mode.icon }}</div>
          <div class="text-xs font-black text-content uppercase tracking-widest">{{ appStore.t(mode.label) }}</div>
          <div class="text-[0.5625rem] text-muted mt-2 uppercase tracking-tighter">{{ mode.desc }}</div>
        </button>
      </div>
    </section>

    <!-- 减少动画 -->
    <section class="aero-card p-10">
      <div class="flex items-center justify-between">
        <div>
          <h3 class="text-sm font-black text-content uppercase tracking-[0.3em]">
            {{ appStore.t('accessibility.reduceMotion') }}
          </h3>
          <p class="text-[0.5625rem] text-muted mt-2 uppercase tracking-tighter">
            {{ appStore.t('accessibility.reduceMotion.desc') }}
          </p>
        </div>
        <button type="button" role="switch"
          :aria-checked="appStore.settings.accessibility?.reduceMotion"
          :aria-label="appStore.t('accessibility.reduceMotion')"
          @click="updateAccessibility({ reduceMotion: !appStore.settings.accessibility?.reduceMotion })"
          class="w-12 h-6 rounded-full border border-subtle p-0.5 transition-all cursor-pointer"
          :class="appStore.settings.accessibility?.reduceMotion ? 'bg-primary/40 border-primary' : 'bg-input'"
        >
          <div
            class="w-5 h-5 rounded-full bg-white shadow-sm transition-all"
            :class="appStore.settings.accessibility?.reduceMotion ? 'translate-x-6' : ''"
          ></div>
        </button>
      </div>
    </section>

    <!-- 焦点指示器 -->
    <section class="aero-card p-10">
      <div class="flex items-center justify-between">
        <div>
          <h3 class="text-sm font-black text-content uppercase tracking-[0.3em]">
            {{ appStore.t('accessibility.focusIndicator') }}
          </h3>
          <p class="text-[0.5625rem] text-muted mt-2 uppercase tracking-tighter">
            {{ appStore.t('accessibility.focusIndicator.desc') }}
          </p>
        </div>
        <button type="button" role="switch"
          :aria-checked="appStore.settings.accessibility?.focusIndicator"
          :aria-label="appStore.t('accessibility.focusIndicator')"
          @click="updateAccessibility({ focusIndicator: !appStore.settings.accessibility?.focusIndicator })"
          class="w-12 h-6 rounded-full border border-subtle p-0.5 transition-all cursor-pointer"
          :class="appStore.settings.accessibility?.focusIndicator ? 'bg-primary/40 border-primary' : 'bg-input'"
        >
          <div
            class="w-5 h-5 rounded-full bg-white shadow-sm transition-all"
            :class="appStore.settings.accessibility?.focusIndicator ? 'translate-x-6' : ''"
          ></div>
        </button>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()

const fontSizes = [
  { value: 'normal', label: 'accessibility.fontSize.normal', icon: 'A', example: '16px' },
  { value: 'large', label: 'accessibility.fontSize.large', icon: 'A', example: '18px' },
  { value: 'x-large', label: 'accessibility.fontSize.xlarge', icon: 'A', example: '20px' },
]

const colorBlindModes = [
  { value: 'none', label: 'accessibility.colorBlind.none', icon: '👁️', desc: '正常色觉' },
  { value: 'protanopia', label: 'accessibility.colorBlind.protanopia', icon: '🔴', desc: '红色感知缺失' },
  { value: 'deuteranopia', label: 'accessibility.colorBlind.deuteranopia', icon: '🟢', desc: '绿色感知缺失' },
  { value: 'tritanopia', label: 'accessibility.colorBlind.tritanopia', icon: '🔵', desc: '蓝色感知缺失' },
]

const updateAccessibility = (updates: Partial<typeof appStore.settings.accessibility>) => {
  const current = appStore.settings.accessibility || {
    fontSize: 'normal',
    highContrast: false,
    colorBlindMode: 'none',
    reduceMotion: false,
    focusIndicator: true,
  }

  appStore.updateSettings({
    accessibility: {
      ...current,
      ...updates,
    },
  })

  appStore.saveSettingsToStorage()
}
</script>
