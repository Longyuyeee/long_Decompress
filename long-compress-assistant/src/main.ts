import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'

import './assets/css/main.css'
import './styles/design-tokens.css'
import './styles/responsive-utilities.css'
import './styles/animation-utilities.css'
import 'primeicons/primeicons.css'

const app = createApp(App)

app.use(createPinia())
app.use(router)

app.mount('#app')
