<template>
  <Modal
    :visible="isOpen"
    size="md"
    layer="nested"
    show-footer
    :title="$t('password.generator.title', '密码生成器')"
    description="按用途生成并立即使用"
    icon="pi pi-key"
    @close="closeDialog"
  >
    <div class="password-generator" data-testid="password-generator">
      <section class="password-result" data-testid="password-generator-result" aria-live="polite">
        <div class="result-heading"><span>生成结果</span><strong :class="strengthTextColor">{{ generatedPassword ? strengthLabel : '等待生成' }}</strong></div>
        <div class="result-value">
          <code>{{ generatedPassword || appStore.t('password.generator.placeholder', '点击生成密码...') }}</code>
          <button v-if="generatedPassword" type="button" :title="appStore.t('password.generator.copy', '复制')" @click="copyPassword">
            <i :class="copied ? 'pi pi-check' : 'pi pi-copy'"></i><span>{{ copied ? '已复制' : appStore.t('password.generator.copy', '复制') }}</span>
          </button>
        </div>
        <div v-if="generatedPassword" class="strength-meter"><span><i :class="strengthColor" :style="{ width: `${strengthScore}%` }"></i></span><output>{{ strengthScore }}</output></div>
      </section>

      <section class="generator-section">
        <header><div><strong>{{ appStore.t('password.generator.mode', '生成模式') }}</strong><small>选择更适合当前用途的密码结构</small></div></header>
        <div class="mode-grid">
          <button v-for="mode in modes" :key="mode.value" type="button" :data-testid="`password-mode-${mode.value}`" :class="{ selected: selectedMode === mode.value }" @click="selectedMode = mode.value">
            <i :class="mode.icon"></i><span>{{ mode.label }}</span><i v-if="selectedMode === mode.value" class="pi pi-check selected-mark"></i>
          </button>
        </div>
      </section>

      <section v-if="selectedMode === 'standard'" class="generator-section standard-options">
        <header><div><strong>{{ appStore.t('password.generator.strength', '强度级别') }}</strong><small>强度越高，生成的密码越长</small></div></header>
        <div class="strength-grid">
          <button v-for="strength in strengths" :key="strength.value" type="button" :class="{ selected: selectedStrength === strength.value }" @click="selectedStrength = strength.value">{{ strength.label }}</button>
        </div>
        <header class="charset-heading"><div><strong>{{ appStore.t('password.generator.charset', '字符集') }}</strong><small>控制密码中允许出现的字符</small></div></header>
        <div class="charset-grid">
          <label v-for="option in charsetOptions" :key="option.key" :class="{ wide: option.key === 'excludeAmbiguous' }">
            <input v-model="charset[option.key]" type="checkbox"><span class="check-box"><i class="pi pi-check"></i></span><span>{{ option.label }}</span>
          </label>
        </div>
      </section>

      <section v-else class="generator-section range-section">
        <header><div><strong>{{ selectedMode === 'memorable' ? appStore.t('password.generator.word_count', '单词数量') : appStore.t('password.generator.pin_length', 'PIN 长度') }}</strong><small>{{ selectedMode === 'memorable' ? '更多单词更难猜测，同时仍便于记忆' : '增加位数可以提高随机组合数量' }}</small></div><output>{{ selectedMode === 'memorable' ? `${wordCount} 个单词` : `${pinLength} 位` }}</output></header>
        <input v-if="selectedMode === 'memorable'" v-model.number="wordCount" type="range" min="2" max="5">
        <input v-else v-model.number="pinLength" type="range" min="4" max="12">
      </section>
    </div>
    <template #footer>
      <button type="button" class="generator-secondary" data-testid="password-generate" @click="generatePassword"><i class="pi pi-refresh"></i>{{ appStore.t('password.generator.generate', '重新生成') }}</button>
      <button type="button" class="generator-primary" data-testid="password-use" :disabled="!generatedPassword" @click="usePassword"><i class="pi pi-check"></i>{{ appStore.t('password.generator.use', '使用此密码') }}</button>
    </template>
  </Modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useAppStore } from '@/stores/app'
import { getCurrentInstance } from 'vue'
import Modal from '@/components/ui/Modal.vue'

const appStore = useAppStore()
const instance = getCurrentInstance()
const $t = instance?.appContext.config.globalProperties.$t || ((key: string, fallback?: string) => fallback || key)

const props = defineProps<{
  isOpen: boolean
}>()

const emit = defineEmits<{
  close: []
  select: [password: string]
}>()

type ModeType = 'standard' | 'memorable' | 'pin'
type StrengthType = 'weak' | 'medium' | 'strong' | 'very_strong'

const selectedMode = ref<ModeType>('standard')
const selectedStrength = ref<StrengthType>('strong')
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

const modes: Array<{ value: ModeType; label: string; icon: string }> = [
  { value: 'standard', label: '标准', icon: 'pi pi-key' },
  { value: 'memorable', label: '易记', icon: 'pi pi-book' },
  { value: 'pin', label: 'PIN', icon: 'pi pi-hashtag' },
]

const strengths: Array<{ value: StrengthType; label: string }> = [
  { value: 'weak', label: '弱' },
  { value: 'medium', label: '中' },
  { value: 'strong', label: '强' },
  { value: 'very_strong', label: '超强' },
]

type CharsetKey = 'lowercase' | 'uppercase' | 'numbers' | 'symbols' | 'excludeAmbiguous'

const charsetOptions: Array<{ key: CharsetKey; label: string }> = [
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
    generatedPassword.value = ''
    appStore.setError(`${appStore.t('common.error')}: ${String(error)}`)
  }
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

// 当对话框打开时自动生成密码
watch(() => props.isOpen, (isOpen) => {
  if (isOpen && !generatedPassword.value) {
    generatePassword()
  }
}, { immediate: true })
</script>

<style scoped>
.password-generator{display:grid;min-width:0;gap:.7rem}.password-result,.generator-section{min-width:0;border:1px solid var(--border-subtle);border-radius:.9rem;background:color-mix(in srgb,var(--bg-input) 72%,transparent);padding:.75rem}.result-heading,.generator-section>header{display:flex;align-items:center;justify-content:space-between;gap:.75rem}.result-heading span,.generator-section header strong{color:var(--text-content);font-size:.65rem;font-weight:900}.result-heading strong{font-size:.57rem;font-weight:900}.generator-section header small{display:block;margin-top:.08rem;color:var(--text-muted);font-size:.51rem;font-weight:650}.result-value{display:flex;min-width:0;align-items:center;gap:.5rem;margin-top:.5rem;border-radius:.7rem;background:var(--bg-card);padding:.55rem .65rem}.result-value code{min-width:0;flex:1;overflow-wrap:anywhere;color:var(--dynamic-accent);font-size:.72rem;font-weight:850;line-height:1.35;user-select:all}.result-value button{display:flex;height:1.85rem;flex:0 0 auto;align-items:center;gap:.3rem;border-radius:.55rem;background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent);padding:0 .55rem;color:var(--dynamic-accent);font-size:.56rem;font-weight:850}.strength-meter{display:grid;grid-template-columns:minmax(0,1fr) 1.8rem;align-items:center;gap:.45rem;margin-top:.45rem}.strength-meter>span{height:.28rem;overflow:hidden;border-radius:999px;background:var(--bg-card)}.strength-meter i{display:block;height:100%;border-radius:inherit;transition:width .25s ease}.strength-meter output{color:var(--text-muted);font-size:.53rem;font-weight:850;text-align:right}.mode-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:.4rem;margin-top:.55rem}.mode-grid button{display:grid;min-width:0;height:2.45rem;grid-template-columns:1rem minmax(0,1fr) .75rem;align-items:center;gap:.3rem;border:1px solid var(--border-subtle);border-radius:.68rem;background:var(--bg-card);padding:0 .55rem;color:var(--text-muted);font-size:.62rem;font-weight:850;text-align:left;transition:border-color .16s ease,background-color .16s ease,color .16s ease}.mode-grid button:hover,.mode-grid button.selected,.strength-grid button:hover,.strength-grid button.selected{border-color:color-mix(in srgb,var(--dynamic-accent) 65%,var(--border-subtle));background:color-mix(in srgb,var(--dynamic-accent) 9%,var(--bg-card));color:var(--dynamic-accent)}.mode-grid button>i:first-child{font-size:.65rem}.mode-grid .selected-mark{font-size:.5rem}.strength-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:.35rem;margin-top:.5rem}.strength-grid button{height:2rem;border:1px solid var(--border-subtle);border-radius:.58rem;background:var(--bg-card);color:var(--text-muted);font-size:.58rem;font-weight:850}.charset-heading{margin-top:.7rem}.charset-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:.35rem;margin-top:.5rem}.charset-grid label{display:flex;min-width:0;min-height:2rem;align-items:center;gap:.45rem;border-radius:.58rem;background:var(--bg-card);padding:.35rem .5rem;color:var(--text-muted);font-size:.56rem;font-weight:750;cursor:pointer}.charset-grid label.wide{grid-column:1/-1}.charset-grid input{position:absolute;width:1px;height:1px;opacity:0}.check-box{display:grid;width:1rem;height:1rem;flex:0 0 1rem;place-items:center;border:1px solid var(--border-subtle);border-radius:.3rem;color:transparent;font-size:.45rem}.charset-grid input:checked+.check-box{border-color:var(--dynamic-accent);background:var(--dynamic-accent);color:white}.range-section header output{flex:0 0 auto;border-radius:999px;background:color-mix(in srgb,var(--dynamic-accent) 12%,transparent);padding:.3rem .55rem;color:var(--dynamic-accent);font-size:.6rem;font-weight:900}.range-section>input{width:100%;margin-top:.75rem;accent-color:var(--dynamic-accent)}.generator-secondary,.generator-primary{display:flex;min-width:7.5rem;height:2.35rem;align-items:center;justify-content:center;gap:.4rem;border-radius:.72rem;padding:0 .85rem;font-size:.64rem;font-weight:900}.generator-secondary{border:1px solid var(--border-subtle);background:var(--bg-input);color:var(--text-content)}.generator-primary{background:var(--dynamic-accent);color:white;box-shadow:0 10px 24px -16px var(--dynamic-accent)}.generator-primary:disabled{cursor:not-allowed;opacity:.4}@media(max-width:520px){.mode-grid{grid-template-columns:1fr}.strength-grid{grid-template-columns:repeat(2,minmax(0,1fr))}.charset-grid{grid-template-columns:1fr}.charset-grid label.wide{grid-column:auto}.generator-secondary,.generator-primary{min-width:0;flex:1}}
</style>
