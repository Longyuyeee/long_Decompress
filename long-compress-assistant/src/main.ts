import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import { translations } from './i18n'

import './assets/css/main.css'
import './styles/design-tokens.css'
import './styles/accessibility.css'
import './styles/responsive-utilities.css'
import './styles/animation-utilities.css'
import 'primeicons/primeicons.css'

const app = createApp(App)

const pinia = createPinia()
app.use(pinia)
app.use(router)

// Global translation function
app.config.globalProperties.$t = (key: string, fallback?: string): string => {
  const lang = localStorage.getItem('app-language') || 'zh-CN'
  return translations[lang]?.[key] || translations['zh-CN']?.[key] || fallback || key
}

app.mount('#app')
