import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { VitePWA } from "vite-plugin-pwa";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const host = process.env.TAURI_DEV_HOST;

// Path to project-root templates/
const templatesDir = path.resolve(__dirname, "templates");

export default defineConfig({
  plugins: [
    vue(),
    tailwindcss(),
    VitePWA({
      registerType: "autoUpdate",
      includeAssets: ["favicon.ico"],
      manifest: {
        name: "Workflow Engine",
        short_name: "Workflow",
        description: "可视化工作流引擎 — 编排、执行、监控",
        lang: "zh-CN",
        theme_color: "#0d1117",
        background_color: "#0d1117",
        display: "standalone",
        display_override: ["window-controls-overlay"],
        orientation: "any",
        start_url: "/",
        categories: ["productivity", "developer-tools", "utilities"],
        dir: "ltr",
        icons: [
          {
            src: "/icon-192.png",
            sizes: "192x192",
            type: "image/png",
          },
          {
            src: "/icon-512.png",
            sizes: "512x512",
            type: "image/png",
            purpose: "any maskable",
          },
        ],
        shortcuts: [
          {
            name: "新建工作流",
            short_name: "新建",
            description: "创建新的工作流",
            url: "/?action=new",
            icons: [{ src: "/icon-192.png", sizes: "192x192" }],
          },
          {
            name: "运行记录",
            short_name: "记录",
            description: "查看最近运行记录",
            url: "/?view=history",
          },
          {
            name: "模板库",
            short_name: "模板",
            description: "浏览工作流模板",
            url: "/?view=templates",
          },
        ],
      },
      workbox: {
        navigateFallback: "/offline.html",
        globPatterns: ["**/*.{js,css,html,ico,png,svg,woff2}"],
        runtimeCaching: [
          {
            urlPattern: /^\/api\/.*/i,
            handler: "NetworkFirst",
            options: {
              cacheName: "api-cache",
              expiration: { maxEntries: 50, maxAgeSeconds: 300 },
            },
          },
        ],
      },
    }),
    {
      name: "serve-templates",
      configureServer(server) {
        server.middlewares.use("/api/templates", (req, res) => {
          const url = (req.url || "").split("?")[0];
          const templateId = url.replace("/api/templates", "").replace(/^\/+/, "");

          // Prevent path traversal attacks
          if (templateId.includes('/') || templateId.includes('\\') || templateId.includes('..') || templateId.includes('~')) {
            res.statusCode = 400;
            res.end("Invalid template ID");
            return;
          }

          if (!templateId) {
            const files = fs.readdirSync(templatesDir).filter(f => f.endsWith(".json"));
            const list = files.map(f => {
              const id = f.replace(".json", "");
              try {
                const raw = fs.readFileSync(path.join(templatesDir, f), "utf-8");
                const json = JSON.parse(raw);
                return { id, name: json.name || id, description: json.description || "" };
              } catch {
                return { id, name: id, description: "" };
              }
            });
            res.setHeader("Content-Type", "application/json");
            res.end(JSON.stringify(list));
          } else {
            const filePath = path.join(templatesDir, `${templateId}.json`);
            if (!fs.existsSync(filePath)) {
              res.statusCode = 404;
              res.end("Template not found");
              return;
            }
            res.setHeader("Content-Type", "application/json");
            res.end(fs.readFileSync(filePath, "utf-8"));
          }
        });
      },
    },
    // Resolve @tauri-apps/api .ts sources (package ships .ts only, no .js)
    {
      name: "resolve-tauri-api",
      enforce: "pre",
      resolveId(source) {
        if (source.startsWith("@tauri-apps/api/")) {
          const subpath = source.replace("@tauri-apps/api/", "");
          const tsPath = path.resolve(__dirname, "node_modules/@tauri-apps/api/src", subpath + ".ts");
          if (fs.existsSync(tsPath)) return tsPath;
        }
        return null;
      },
    },
  ],

  resolve: {
    alias: {
      // 使用 runtime-only 构建，避免 CSP unsafe-eval
      'vue': 'vue/dist/vue.runtime.esm-bundler.js',
      // vue-i18n runtime 构建不含 message compiler (new Function)，兼容 CSP
      'vue-i18n': 'vue-i18n/dist/vue-i18n.runtime.mjs',
      '@': path.resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  build: {
    outDir: "./dist",
    emptyOutDir: true,
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
