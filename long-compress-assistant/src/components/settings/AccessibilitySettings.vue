<template>
  <div class="accessibility-settings">
    <!-- 字体大小 -->
    <section class="accessibility-row accessibility-choice-row accessibility-font">
      <h3 class="text-sm font-black text-content uppercase tracking-[0.3em] mb-6">
        {{ appStore.t('accessibility.fontSize') }}
      </h3>
      <div class="grid grid-cols-3 gap-4">
        <button
          v-for="size in fontSizes"
          :key="size.value"
          :aria-pressed="appStore.settings.accessibility?.fontSize === size.value"
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
    <section class="accessibility-row">
      <div class="flex items-center justify-between mb-6">
        <div>
          <div class="flex items-center gap-2"><h3 class="text-sm font-black text-content uppercase tracking-[0.3em]">
            {{ appStore.t('accessibility.highContrast') }}
          </h3><HelpHint :label="appStore.t('accessibility.highContrast')">{{ appStore.t('accessibility.highContrast.desc') }}</HelpHint></div>
        </div>
        <button type="button" role="switch"
          :aria-checked="appStore.settings.accessibility?.highContrast"
          :aria-label="appStore.t('accessibility.highContrast')"
          @click="updateAccessibility({ highContrast: !appStore.settings.accessibility?.highContrast })"
          class="settings-toggle-track cursor-pointer"
          :class="{ 'is-on': appStore.settings.accessibility?.highContrast }"
        >
          <div
            class="settings-toggle-knob"
          ></div>
        </button>
      </div>
    </section>

    <!-- 色盲模式 -->
    <section class="accessibility-row accessibility-choice-row accessibility-color">
      <h3 class="text-sm font-black text-content uppercase tracking-[0.3em] mb-6">
        {{ appStore.t('accessibility.colorBlind') }}
      </h3>
      <div class="grid grid-cols-2 gap-4">
        <button
          v-for="mode in colorBlindModes"
          :key="mode.value"
          :aria-pressed="appStore.settings.accessibility?.colorBlindMode === mode.value"
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
    <section class="accessibility-row">
      <div class="flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2"><h3 class="text-sm font-black text-content uppercase tracking-[0.3em]">
            {{ appStore.t('accessibility.reduceMotion') }}
          </h3><HelpHint :label="appStore.t('accessibility.reduceMotion')">{{ appStore.t('accessibility.reduceMotion.desc') }}</HelpHint></div>
        </div>
        <button type="button" role="switch"
          :aria-checked="appStore.settings.accessibility?.reduceMotion"
          :aria-label="appStore.t('accessibility.reduceMotion')"
          @click="updateAccessibility({ reduceMotion: !appStore.settings.accessibility?.reduceMotion })"
          class="settings-toggle-track cursor-pointer"
          :class="{ 'is-on': appStore.settings.accessibility?.reduceMotion }"
        >
          <div
            class="settings-toggle-knob"
          ></div>
        </button>
      </div>
    </section>

    <!-- 焦点指示器 -->
    <section class="accessibility-row">
      <div class="flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2"><h3 class="text-sm font-black text-content uppercase tracking-[0.3em]">
            {{ appStore.t('accessibility.focusIndicator') }}
          </h3><HelpHint :label="appStore.t('accessibility.focusIndicator')">{{ appStore.t('accessibility.focusIndicator.desc') }}</HelpHint></div>
        </div>
        <button type="button" role="switch"
          :aria-checked="appStore.settings.accessibility?.focusIndicator"
          :aria-label="appStore.t('accessibility.focusIndicator')"
          @click="updateAccessibility({ focusIndicator: !appStore.settings.accessibility?.focusIndicator })"
          class="settings-toggle-track cursor-pointer"
          :class="{ 'is-on': appStore.settings.accessibility?.focusIndicator }"
        >
          <div
            class="settings-toggle-knob"
          ></div>
        </button>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
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

<style scoped>
.accessibility-row { padding: .75rem 0; min-width: 0; }
.accessibility-row:first-child { padding-top: 0; }
.accessibility-row:last-child { padding-bottom: 0; }
.accessibility-row + .accessibility-row { border-top: 1px solid var(--border-subtle); }
.accessibility-choice-row { display: grid; grid-template-columns: 9.5rem minmax(0, 1fr); gap: 1rem; align-items: center; }
.accessibility-row h3 { margin: 0; font-size: .8125rem; font-weight: 650; letter-spacing: 0; text-transform: none; }
.accessibility-row .grid { gap: .6rem; min-width: 0; }
.accessibility-color > .grid { grid-template-columns: repeat(4, minmax(0, 1fr)); }
.accessibility-row .grid > button { padding: .6rem .75rem; border-width: 1px; border-radius: .7rem; box-shadow: none; min-width: 0; }
.accessibility-font .grid > button { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: .35rem .8rem; min-height: 2.75rem; }
.accessibility-row .grid > button > div { margin: 0; letter-spacing: 0; text-transform: none; }
.accessibility-font .grid > button > div:first-child { font-size: 1.1rem; color: var(--dynamic-accent); font-weight: 650; }
.accessibility-row .grid > button > div:last-child { font-size: .6875rem; }
.accessibility-color .grid > button { display: grid; grid-template-columns: 1.5rem minmax(0, 1fr); align-items: center; gap: .2rem .5rem; }
.accessibility-color .grid > button > div:first-child { font-size: 1rem; grid-row: 1 / 3; }
.accessibility-color .grid > button > div:last-child { grid-column: 2; font-size: .625rem; }
.accessibility-row button[aria-pressed="true"] { border-color: color-mix(in srgb, var(--dynamic-accent) 48%, transparent); }
.accessibility-row > .flex { gap: 1rem; margin: 0; }
.accessibility-row p { font-size: .75rem; line-height: 1.5; margin-top: .25rem; letter-spacing: 0; text-transform: none; }
.accessibility-row .settings-toggle-track { flex-shrink: 0; box-shadow: none; }
@container (max-width: 48rem) {
  .accessibility-choice-row { grid-template-columns: 7rem minmax(0, 1fr); gap: .7rem; }
  .accessibility-color > .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@container (max-width: 32rem) {
  .accessibility-choice-row { grid-template-columns: minmax(0, 1fr); }
  .accessibility-font .grid > button { flex-direction: column; }
}
</style>
