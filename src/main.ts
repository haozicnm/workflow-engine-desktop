import { createApp } from 'vue'
import App from './App.vue'
import { pinia } from './stores'
import i18n from './i18n'
import './style.css'

const app = createApp(App)

app.config.errorHandler = (err, _instance, info) => {
  console.error('[Vue Error]', err, info)
}

window.addEventListener('unhandledrejection', (event) => {
  console.error('[Unhandled Promise]', event.reason)
  event.preventDefault()
})

app.use(i18n)
app.use(pinia)
app.mount('#app')
