import { createApp } from 'vue'
import App from './App.vue'
import { pinia } from './stores'
import { useTheme } from './composables/useTheme'
import i18n from './i18n'
import './style.css'

// Initialize theme before mount
useTheme()

const app = createApp(App)

// 全局错误处理
app.config.errorHandler = (err, _instance, info) => {
  console.error('[Vue Error]', err, info)
  // 可选：上报到错误收集服务
}

window.addEventListener('unhandledrejection', (event) => {
  console.error('[Unhandled Promise]', event.reason)
  event.preventDefault() // 防止控制台报红
})

app.use(i18n)
app.use(pinia)
app.mount('#app')
