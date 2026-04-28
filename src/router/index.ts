import { createRouter, createWebHistory } from 'vue-router'
import Dashboard from '../pages/Dashboard.vue'
import Editor from '../pages/Editor.vue'
import DAGEditor from '../pages/DAGEditor.vue'
import RunHistory from '../pages/RunHistory.vue'
import Settings from '../pages/Settings.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'dashboard', component: Dashboard },
    { path: '/editor/new', name: 'editor-new', component: Editor },
    { path: '/editor/:id', name: 'editor', component: Editor },
    { path: '/dag-editor', name: 'dag-editor', component: DAGEditor },
    { path: '/history', name: 'history', component: RunHistory },
    { path: '/settings', name: 'settings', component: Settings },
  ],
})

export default router
