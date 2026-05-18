import { createApp } from 'vue'
import App from './App.vue'
import { pinia } from './stores'
import { useTheme } from './composables/useTheme'
import i18n from './i18n'
import './style.css'

// Initialize theme before mount
useTheme()

const app = createApp(App)
app.use(i18n)
app.use(pinia)
app.mount('#app')
