import { createWebHistory, createRouter } from "vue-router";
import Home from "../views/Home.vue";
import Editor from "../views/Editor.vue";
import Monitor from "../views/Monitor.vue";
import Settings from "../views/Settings.vue";

const routes = [
  { path: "/", name: "home", component: Home },
  { path: "/editor", name: "editor", component: Editor },
  { path: "/editor/:id", name: "editor-edit", component: Editor },
  { path: "/monitor", name: "monitor", component: Monitor },
  { path: "/settings", name: "settings", component: Settings },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
