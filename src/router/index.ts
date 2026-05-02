import { createRouter, createWebHistory } from 'vue-router'
import LiteGraphEditor from '../pages/LiteGraphEditor.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'editor', component: LiteGraphEditor },
  ],
})

export default router
