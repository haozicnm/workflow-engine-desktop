import { createRouter, createWebHistory } from 'vue-router'
import Dashboard from '../pages/Dashboard.vue'
import LiteGraphEditor from '../pages/LiteGraphEditor.vue'
import RunHistory from '../pages/RunHistory.vue'
import Settings from '../pages/Settings.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'dashboard', component: Dashboard },
    { path: '/editor/new', name: 'editor-new', component: LiteGraphEditor },
    { path: '/editor/:id', name: 'editor', component: LiteGraphEditor },
    { path: '/history', name: 'history', component: RunHistory },
    { path: '/settings', name: 'settings', component: Settings },
  ],
})

export default router
