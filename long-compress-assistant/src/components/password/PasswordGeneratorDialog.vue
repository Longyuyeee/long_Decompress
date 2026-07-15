<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center">
    <!-- 背景遮罩 -->
    <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" @click="closeDialog"></div>

    <!-- 对话框 -->
    <div class="relative w-full max-w-md mx-4 bg-surface rounded-2xl shadow-2xl border border-subtle">
      <!-- 头部 -->
      <div class="px-6 py-5 border-b border-subtle">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <span class="text-2xl">🎲</span>
            <h2 class="text-lg font-bold text-primary">{{ $t('password.generator.title', '密码生成器') }}</h2>
          </div>
          <button
            @click="closeDialog"
            class="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-input/50 transition-colors text-muted hover:text-primary"
          >
            <span class="text-lg">×</span>
          </button>
        </div>
      </div>

      <!-- 内容 -->
      <div class="p-6 space-y-6">
        <!-- 生成的密码显示 -->
        <div class="relative">
          <div class="flex items-center gap-2 p-4 bg-input/30 rounded-xl border border-subtle font-mono text-sm break-all select-all">
            <span class="flex-1 text-primary">{{ generatedPassword || $t('password.generator.placeholder', '点击生成密码...') }}</span>
            <button
              v-if="generatedPassword"
              @click="copyPassword"
              class="shrink-0 px-3 py-1.5 rounded-lg bg-primary/10 hover:bg-primary/20 text-primary text-xs font-bold transition-colors"
            >
              {{ copied ? '✓' : $t('password.generator.copy', '复制') }}
            </button>
          </div>

          <!-- 密码强度指示 -->
          <div v-if="generatedPassword" class="mt-2 flex items-center gap-2">
            <div class="flex-1 h-2 bg-input rounded-full overflow-hidden">
              <div
                class="h-full transition-all duration-300"
                :class="strengthColor"
                :style="{ width: `${strengthScore}%` }"
              ></div>
            </div>
            <span class="text-xs font-bold" :class="strengthTextColor">
              {{ strengthLabel }}
            </span>
          </div>
        </div>

        <!-- 生成模式 -->
        <div class="space-y-3">
          <label class="text-xs font-bold text-primary uppercase tracking-wider">
            {{ $t('password.generator.mode', '生成模式') }}
          </label>
          <div class="grid grid-cols-2 gap-2">
            <button
              v-for="mode in modes"
              :key="mode.value"
              @click="selectedMode = mode.value"
              class="px-4 py-3 rounded-xl border transition-all text-left"
              :class="selectedMode === mode.value
                ? 'bg-primary/10 border-primary text-primary'
                : 'bg-input/30 border-subtle text-muted hover:border-primary/50'"
            >
              <div class="flex items-center gap-2">
                <span>{{ mode.icon }}</span>
                <span class="text-sm font-bold">{{ mode.label }}</span>
              </div>
            </button>
          </div>
        </div>

        <!-- 标准模式选项 -->
        <div v-if="selectedMode === 'standard'" class="space-y-4">
          <!-- 强度选择 -->
          <div class="space-y-2">
            <label class="text-xs font-bold text-primary uppercase tracking-wider">
              {{ $t('password.generator.strength', '强度级别') }}
            </label>
            <div class="grid grid-cols-4 gap-2">
              <button
                v-for="strength in strengths"
                :key="strength.value"
                @click="selectedStrength = strength.value"
                class="px-3 py-2 rounded-lg border text-xs font-bold transition-all"
                :class="selectedStrength === strength.value
                  ? 'bg-primary/10 border-primary text-primary'
                  : 'bg-input/30 border-subtle text-muted hover:border-primary/50'"
              >
                {{ strength.label }}
              </button>
            </div>
          </div>

          <!-- 字符集选项 -->
          <div class="space-y-2">
            <label class="text-xs font-bold text-primary uppercase tracking-wider">
              {{ $t('password.generator.charset', '字符集') }}
            </label>
            <div class="space-y-2">
              <label v-for="option in charsetOptions" :key="option.key" class="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  v-model="charset[option.key]"
                  class="w-4 h-4 rounded border-subtle"
                />
                <span class="text-sm text-primary">{{ option.label }}</span>
              </label>
            </div>
          </div>
        </div>

        <!-- 易记模式选项 -->
        <div v-if="selectedMode === 'memorable'" class="space-y-2">
          <label class="text-xs font-bold text-primary uppercase tracking-wider">
            {{ $t('password.generator.word_count', '单词数量') }}
          </label>
          <input
            v-model.number="wordCount"
            type="range"
            min="2"
            max="5"
            class="w-full"
          />
          <div class="text-center text-sm text-muted">{{ wordCount }} 个单词</div>
        </div>

        <!-- PIN 模式选项 -->
        <div v-if="selectedMode === 'pin'" class="space-y-2">
          <label class="text-xs font-bold text-primary uppercase tracking-wider">
            {{ $t('password.generator.pin_length', 'PIN 长度') }}
          </label>
          <input
            v-model.number="pinLength"
            type="range"
            min="4"
            max="12"
            class="w-full"
          />
          <div class="text-center text-sm text-muted">{{ pinLength }} 位数字</div>
        </div>
      </div>

      <!-- 底部操作 -->
      <div class="px-6 py-4 border-t border-subtle flex gap-3">
        <button
          @click="generatePassword"
          class="flex-1 px-4 py-3 rounded-xl bg-primary text-white font-bold hover:bg-primary/90 transition-colors"
        >
          {{ $t('password.generator.generate', '生成密码') }}
        </button>
        <button
          v-if="generatedPassword"
          @click="usePassword"
          class="flex-1 px-4 py-3 rounded-xl bg-input/30 border border-subtle text-primary font-bold hover:border-primary transition-colors"
        >
          {{ $t('password.generator.use', '使用此密码') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

const props = defineProps<{
  isOpen: boolean
}>()

const emit = defineEmits<{
  close: []
  select: [password: string]
}>()

const selectedMode = ref<'standard' | 'memorable' | 'pin'>('standard')
const selectedStrength = ref<'weak' | 'medium' | 'strong' | 'very_strong'>('strong')
const generatedPassword = ref('')
const copied = ref(false)
const wordCount = ref(3)
const pinLength = ref(6)

const charset = ref({
  lowercase: true,
  uppercase: true,
  numbers: true,
  symbols: true,
  excludeAmbiguous: true,
})

const modes = [
  { value: 'standard', label: '标准', icon: '🔑' },
  { value: 'memorable', label: '易记', icon: '💭' },
  { value: 'pin', label: 'PIN', icon: '🔢' },
]

const strengths = [
  { value: 'weak', label: '弱' },
  { value: 'medium', label: '中' },
  { value: 'strong', label: '强' },
  { value: 'very_strong', label: '超强' },
]

const charsetOptions = [
  { key: 'lowercase', label: '小写字母 (a-z)' },
  { key: 'uppercase', label: '大写字母 (A-Z)' },
  { key: 'numbers', label: '数字 (0-9)' },
  { key: 'symbols', label: '符号 (!@#$...)' },
  { key: 'excludeAmbiguous', label: '排除易混淆字符 (0,O,l,1,I)' },
]

// 密码强度评分
const strengthScore = computed(() => {
  if (!generatedPassword.value) return 0

  const len = generatedPassword.value.length
  const hasLower = /[a-z]/.test(generatedPassword.value)
  const hasUpper = /[A-Z]/.test(generatedPassword.value)
  const hasDigit = /\d/.test(generatedPassword.value)
  const hasSymbol = /[^a-zA-Z0-9]/.test(generatedPassword.value)

  let score = 0
  if (len >= 8) score += 10
  if (len >= 12) score += 10
  if (len >= 16) score += 10
  if (len >= 20) score += 10
  if (hasLower) score += 15
  if (hasUpper) score += 15
  if (hasDigit) score += 15
  if (hasSymbol) score += 15

  return Math.min(100, score)
})

const strengthLabel = computed(() => {
  const score = strengthScore.value
  if (score < 30) return '弱'
  if (score < 60) return '中等'
  if (score < 85) return '强'
  return '超强'
})

const strengthColor = computed(() => {
  const score = strengthScore.value
  if (score < 30) return 'bg-red-500'
  if (score < 60) return 'bg-yellow-500'
  if (score < 85) return 'bg-blue-500'
  return 'bg-green-500'
})

const strengthTextColor = computed(() => {
  const score = strengthScore.value
  if (score < 30) return 'text-red-500'
  if (score < 60) return 'text-yellow-500'
  if (score < 85) return 'text-blue-500'
  return 'text-green-500'
})

const generatePassword = async () => {
  try {
    let password = ''

    if (selectedMode.value === 'standard') {
      password = await invoke<string>('generate_password', {
        strength: selectedStrength.value,
        options: charset.value,
      })
    } else if (selectedMode.value === 'memorable') {
      password = await invoke<string>('generate_memorable_password', {
        wordCount: wordCount.value,
      })
    } else if (selectedMode.value === 'pin') {
      password = await invoke<string>('generate_pin', {
        length: pinLength.value,
      })
    }

    generatedPassword.value = password
    copied.value = false
  } catch (error) {
    console.error('Failed to generate password:', error)
    // Fallback: 生成简单密码
    generatedPassword.value = generateFallbackPassword()
  }
}

const generateFallbackPassword = () => {
  const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*'
  const length = selectedStrength.value === 'weak' ? 10 :
                 selectedStrength.value === 'medium' ? 14 :
                 selectedStrength.value === 'strong' ? 18 : 28
  return Array.from({ length }, () => chars[Math.floor(Math.random() * chars.length)]).join('')
}

const copyPassword = async () => {
  try {
    await navigator.clipboard.writeText(generatedPassword.value)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (error) {
    console.error('Failed to copy:', error)
  }
}

const usePassword = () => {
  emit('select', generatedPassword.value)
  closeDialog()
}

const closeDialog = () => {
  emit('close')
}

// 自动生成一个初始密码
if (props.isOpen && !generatedPassword.value) {
  generatePassword()
}
</script>
